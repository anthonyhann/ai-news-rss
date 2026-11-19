use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::collections::HashSet;
use std::path::Path;
use std::str::FromStr;

use crate::models::{
    Alert, Brief, BriefSummary, DeliveryReceipt, RawItem, RunStats, Score, SourceHealth,
    SourceSpec, Story,
};

const SCHEMA: &str = include_str!("../../migrations/0001_init.sql");

#[derive(Clone)]
pub struct Store {
    pool: SqlitePool,
}

impl Store {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))?
            .create_if_missing(true)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(opts)
            .await
            .with_context(|| format!("open sqlite {}", path.display()))?;
        let store = Self { pool };
        store.migrate().await?;
        Ok(store)
    }

    pub async fn migrate(&self) -> Result<()> {
        for statement in SCHEMA.split(';') {
            let sql = statement.trim();
            if sql.is_empty() {
                continue;
            }
            sqlx::query(sql).execute(&self.pool).await?;
        }
        Ok(())
    }

    pub async fn upsert_sources(&self, sources: &[SourceSpec]) -> Result<()> {
        for source in sources {
            sqlx::query(
                r#"INSERT INTO sources(id, name, kind, tier, enabled)
                   VALUES(?1, ?2, ?3, ?4, ?5)
                   ON CONFLICT(id) DO UPDATE SET
                     name=excluded.name, kind=excluded.kind, tier=excluded.tier,
                     enabled=excluded.enabled"#,
            )
            .bind(&source.id)
            .bind(&source.name)
            .bind(source.kind.as_str())
            .bind(source.tier as i64)
            .bind(if source.enabled { 1i64 } else { 0i64 })
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn record_source_health(&self, health: &SourceHealth) -> Result<()> {
        if health.ok {
            sqlx::query(
                r#"UPDATE sources SET last_ok_at=?1, last_error=NULL, failure_count=0 WHERE id=?2"#,
            )
            .bind(health.checked_at.to_rfc3339())
            .bind(&health.source_id)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                r#"UPDATE sources SET last_error=?1,
                   failure_count=failure_count+1 WHERE id=?2"#,
            )
            .bind(
                health
                    .error
                    .clone()
                    .unwrap_or_else(|| "unknown error".into()),
            )
            .bind(&health.source_id)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn source_health(&self, limit: usize) -> Result<Vec<SourceHealth>> {
        let rows = sqlx::query(
            r#"SELECT s.id, s.name, s.kind, s.tier, s.enabled, s.last_ok_at,
                      s.last_error, s.failure_count,
                      (SELECT COUNT(*) FROM items i WHERE i.source_id=s.id) AS item_count
               FROM sources s ORDER BY s.tier ASC, s.name ASC LIMIT ?1"#,
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let last_ok: Option<String> = row.get("last_ok_at");
                let item_count = row.get::<i64, _>("item_count") as usize;
                let last_error: Option<String> = row.get("last_error");
                SourceHealth {
                    source_id: row.get("id"),
                    name: row.get("name"),
                    kind: serde_json::from_value(serde_json::Value::String(
                        row.get::<String, _>("kind"),
                    ))
                    .unwrap_or_default(),
                    tier: row.get::<i64, _>("tier") as u8,
                    enabled: row.get::<i64, _>("enabled") != 0,
                    ok: last_error.is_none() && (last_ok.is_some() || item_count > 0),
                    item_count,
                    error: last_error,
                    checked_at: last_ok
                        .and_then(|v| DateTime::parse_from_rfc3339(&v).ok())
                        .map(|v| v.with_timezone(&Utc))
                        .unwrap_or_else(Utc::now),
                }
            })
            .collect())
    }

    pub async fn failed_source_ids(&self, threshold: i64) -> Result<HashSet<String>> {
        let rows = sqlx::query("SELECT id FROM sources WHERE failure_count >= ?1")
            .bind(threshold)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|row| row.get("id")).collect())
    }

    pub async fn reset_source_failure(&self, source_id: &str) -> Result<u64> {
        let result = sqlx::query("UPDATE sources SET failure_count=0, last_error=NULL WHERE id=?1")
            .bind(source_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn upsert_item(&self, item: &RawItem) -> Result<()> {
        let payload = serde_json::to_string(item)?;
        let hash = crate::dedup::content_hash(&format!(
            "{}\n{}",
            item.title,
            item.content.chars().take(4000).collect::<String>()
        ));
        sqlx::query(
            r#"INSERT INTO items(id, source_id, kind, title, url, canonical_url, content_hash,
                simhash, published_at, fetched_at, payload_json)
               VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
               ON CONFLICT(id) DO UPDATE SET payload_json=excluded.payload_json,
                 fetched_at=excluded.fetched_at, simhash=excluded.simhash"#,
        )
        .bind(&item.id)
        .bind(&item.source_id)
        .bind(item.kind.as_str())
        .bind(&item.title)
        .bind(&item.url)
        .bind(&item.canonical_url)
        .bind(hash)
        .bind(item.simhash as i64)
        .bind(item.published_at.to_rfc3339())
        .bind(item.fetched_at.to_rfc3339())
        .bind(payload)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn find_item_story(
        &self,
        canonical_url: &str,
        simhash: u64,
        title: &str,
        days: i64,
        max_hamming: u32,
        min_jaccard: f64,
    ) -> Result<Option<Story>> {
        let since = Utc::now() - Duration::days(days.max(1));
        let rows = sqlx::query(
            r#"SELECT s.payload_json, s.score FROM stories s
               WHERE s.last_seen_at >= ?1 ORDER BY s.last_seen_at DESC LIMIT 1000"#,
        )
        .bind(since.to_rfc3339())
        .fetch_all(&self.pool)
        .await?;
        for row in rows {
            let json: String = row.get("payload_json");
            if let Ok(story) = serde_json::from_str::<Story>(&json) {
                if story
                    .sources
                    .iter()
                    .any(|source| crate::dedup::normalize_url(&source.url) == canonical_url)
                {
                    return Ok(Some(story));
                }
                if story.sources.iter().any(|source| {
                    let hash =
                        crate::dedup::simhash64(&format!("{} {}", source.title, source.excerpt));
                    crate::dedup::near_duplicate(
                        title,
                        simhash,
                        &source.title,
                        hash,
                        max_hamming,
                        min_jaccard,
                    )
                }) {
                    return Ok(Some(story));
                }
            }
        }
        Ok(None)
    }

    pub async fn insert_story(&self, story: &Story) -> Result<()> {
        let payload = serde_json::to_string(story)?;
        sqlx::query(
            r#"INSERT INTO stories(id, primary_item_id, first_seen_at, last_seen_at,
                score, version, payload_json) VALUES(?1,?2,?3,?4,?5,?6,?7)
               ON CONFLICT(id) DO UPDATE SET last_seen_at=excluded.last_seen_at,
                 score=excluded.score, version=excluded.version, payload_json=excluded.payload_json"#,
        )
        .bind(&story.id)
        .bind(&story.primary_item_id)
        .bind(story.first_seen_at.to_rfc3339())
        .bind(story.last_seen_at.to_rfc3339())
        .bind(story.score)
        .bind(story.version)
        .bind(payload)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn link_item(
        &self,
        story_id: &str,
        item_id: &str,
        similarity: f64,
        is_primary: bool,
    ) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO story_items(story_id, item_id, similarity, is_primary)
               VALUES(?1,?2,?3,?4)
               ON CONFLICT(story_id,item_id) DO UPDATE SET similarity=excluded.similarity"#,
        )
        .bind(story_id)
        .bind(item_id)
        .bind(similarity)
        .bind(if is_primary { 1i64 } else { 0i64 })
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn recent_stories(&self, limit: usize, min_score: f64) -> Result<Vec<Story>> {
        let rows = sqlx::query(
            r#"SELECT payload_json FROM stories WHERE score >= ?1
               ORDER BY score DESC, last_seen_at DESC LIMIT ?2"#,
        )
        .bind(min_score)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        let mut stories = Vec::new();
        for row in rows {
            let json: String = row.get("payload_json");
            if let Ok(story) = serde_json::from_str::<Story>(&json) {
                if story.status != "merged" {
                    stories.push(story);
                }
            }
        }
        Ok(stories)
    }

    pub async fn stories_needing_score(&self, limit: usize) -> Result<Vec<Story>> {
        let rows = sqlx::query(
            r#"SELECT payload_json FROM stories WHERE score <= 0
               ORDER BY last_seen_at DESC LIMIT ?1"#,
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        let mut stories = Vec::new();
        for row in rows {
            let json: String = row.get("payload_json");
            if let Ok(story) = serde_json::from_str::<Story>(&json) {
                if story.status != "merged" {
                    stories.push(story);
                }
            }
        }
        Ok(stories)
    }

    pub async fn merge_stories(&self, primary_id: &str, secondary_id: &str) -> Result<bool> {
        if primary_id == secondary_id {
            return Ok(false);
        }
        let primary_json: Option<String> =
            sqlx::query_scalar("SELECT payload_json FROM stories WHERE id=?1")
                .bind(primary_id)
                .fetch_optional(&self.pool)
                .await?;
        let secondary_json: Option<String> =
            sqlx::query_scalar("SELECT payload_json FROM stories WHERE id=?1")
                .bind(secondary_id)
                .fetch_optional(&self.pool)
                .await?;
        let (Some(primary_json), Some(secondary_json)) = (primary_json, secondary_json) else {
            return Ok(false);
        };
        let mut primary: Story = serde_json::from_str(&primary_json)?;
        let mut secondary: Story = serde_json::from_str(&secondary_json)?;
        if secondary.status == "merged" || primary.id == secondary.id {
            return Ok(false);
        }
        for source in secondary.sources.drain(..) {
            if !primary
                .sources
                .iter()
                .any(|existing| existing.item_id == source.item_id)
            {
                primary.sources.push(source);
            }
        }
        primary.first_seen_at = primary.first_seen_at.min(secondary.first_seen_at);
        primary.last_seen_at = primary.last_seen_at.max(secondary.last_seen_at);
        primary.version = primary.version.max(secondary.version) + 1;
        primary.score = primary.score.max(secondary.score);
        primary.confidence = primary.confidence.max(secondary.confidence);
        if primary.title_zh.is_empty() {
            primary.title_zh = secondary.title_zh.clone();
        }
        if primary.title_en.is_empty() {
            primary.title_en = secondary.title_en.clone();
        }
        if primary.summary_zh.is_empty() {
            primary.summary_zh = secondary.summary_zh.clone();
        }
        if primary.summary_en.is_empty() {
            primary.summary_en = secondary.summary_en.clone();
        }
        primary.status = "merged_primary".into();
        secondary.status = "merged".into();
        secondary.score = 0.0;
        self.insert_story(&primary).await?;
        self.insert_story(&secondary).await?;
        Ok(true)
    }

    pub async fn save_score(&self, story: &mut Story, score: &Score) -> Result<()> {
        story.score = score.total;
        story.confidence = score.confidence;
        story.breakdown = score.breakdown();
        let json = serde_json::to_string(score)?;
        sqlx::query(
            r#"INSERT INTO scores(story_id, score_json, model, prompt_version, created_at)
               VALUES(?1,?2,?3,?4,?5)
               ON CONFLICT(story_id) DO UPDATE SET score_json=excluded.score_json,
                 model=excluded.model, prompt_version=excluded.prompt_version,
                 created_at=excluded.created_at"#,
        )
        .bind(&story.id)
        .bind(json)
        .bind(&score.model)
        .bind(&score.prompt_version)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        self.insert_story(story).await
    }

    pub async fn save_brief(&self, brief: &Brief) -> Result<()> {
        let id = format!("{}-{}-{}", brief.date, brief.slot, brief.lang);
        let json = serde_json::to_string(brief)?;
        sqlx::query(
            r#"INSERT INTO briefs(id,date,slot,lang,markdown,json,model,created_at)
               VALUES(?1,?2,?3,?4,?5,?6,?7,?8)
               ON CONFLICT(id) DO UPDATE SET markdown=excluded.markdown,
                 json=excluded.json, model=excluded.model, created_at=excluded.created_at"#,
        )
        .bind(id)
        .bind(&brief.date)
        .bind(&brief.slot)
        .bind(&brief.lang)
        .bind(&brief.markdown)
        .bind(json)
        .bind(&brief.model)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn save_action_plan(
        &self,
        date: &str,
        plan: &crate::models::ActionPlan,
    ) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO action_plans(date,json,created_at) VALUES(?1,?2,?3)
               ON CONFLICT(date) DO UPDATE SET json=excluded.json, created_at=excluded.created_at"#,
        )
        .bind(date)
        .bind(serde_json::to_string(plan)?)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn was_alerted(&self, dedup_key: &str, hours: i64) -> Result<bool> {
        let since = Utc::now() - Duration::hours(hours);
        let row = sqlx::query(
            r#"SELECT COUNT(*) AS n FROM deliveries
               WHERE dedup_key=?1 AND kind='alert' AND status='sent' AND sent_at >= ?2"#,
        )
        .bind(dedup_key)
        .bind(since.to_rfc3339())
        .fetch_one(&self.pool)
        .await?;
        Ok(row.get::<i64, _>("n") > 0)
    }

    pub async fn record_delivery(
        &self,
        story_id: Option<&str>,
        brief_id: Option<&str>,
        receipt: &DeliveryReceipt,
        kind: &str,
        dedup_key: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO deliveries(story_id,brief_id,channel,kind,status,dedup_key,sent_at,error)
               VALUES(?1,?2,?3,?4,?5,?6,?7,?8)"#,
        )
        .bind(story_id)
        .bind(brief_id)
        .bind(&receipt.channel)
        .bind(kind)
        .bind(if receipt.ok { "sent" } else { "failed" })
        .bind(dedup_key)
        .bind(receipt.sent_at.to_rfc3339())
        .bind(if receipt.ok {
            None
        } else {
            Some(receipt.detail.clone())
        })
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn save_run(&self, run: &RunStats) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO runs(id,started_at,finished_at,status,stats_json)
               VALUES(?1,?2,?3,?4,?5)
               ON CONFLICT(id) DO UPDATE SET finished_at=excluded.finished_at,
                 status=excluded.status, stats_json=excluded.stats_json"#,
        )
        .bind(&run.id)
        .bind(run.started_at.to_rfc3339())
        .bind(run.finished_at.map(|v| v.to_rfc3339()))
        .bind(&run.status)
        .bind(serde_json::to_string(run)?)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn last_run(&self) -> Result<Option<RunStats>> {
        let json: Option<String> =
            sqlx::query_scalar("SELECT stats_json FROM runs ORDER BY started_at DESC LIMIT 1")
                .fetch_optional(&self.pool)
                .await?;
        Ok(json.and_then(|value| serde_json::from_str(&value).ok()))
    }

    pub async fn list_briefs(&self, limit: usize) -> Result<Vec<BriefSummary>> {
        let rows = sqlx::query(
            r#"SELECT date, slot, lang, json FROM briefs ORDER BY date DESC, slot DESC LIMIT ?1"#,
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .filter_map(|row| {
                let json: String = row.get("json");
                let brief: Brief = serde_json::from_str(&json).ok()?;
                let title = brief
                    .executive_summary
                    .first()
                    .cloned()
                    .unwrap_or_else(|| format!("AI Brief {}", brief.date));
                Some(BriefSummary {
                    date: brief.date.clone(),
                    slot: brief.slot.clone(),
                    lang: brief.lang.clone(),
                    title,
                    path: format!("briefs/{}-{}-{}.json", brief.date, brief.slot, brief.lang),
                })
            })
            .collect())
    }

    pub async fn get_briefs_for_date(&self, date: &str) -> Result<Vec<Brief>> {
        let rows =
            sqlx::query("SELECT json FROM briefs WHERE date=?1 ORDER BY slot DESC, lang ASC")
                .bind(date)
                .fetch_all(&self.pool)
                .await?;
        Ok(rows
            .into_iter()
            .filter_map(|row| serde_json::from_str::<Brief>(&row.get::<String, _>("json")).ok())
            .collect())
    }
}

pub fn alert_from_story(story: &Story) -> Alert {
    let primary = story.sources.first();
    Alert {
        story_id: story.id.clone(),
        level: if story.score >= 95.0 {
            "critical"
        } else {
            "high"
        }
        .into(),
        title_zh: story.title_zh.clone(),
        title_en: story.title_en.clone(),
        summary_zh: story.summary_zh.clone(),
        why_it_matters: story.why_it_matters.clone(),
        url: primary.map(|s| s.url.clone()).unwrap_or_default(),
        score: story.score,
        confidence: story.confidence,
        material_hash: format!("score:{:.0}", story.score),
        sources: story.sources.clone(),
    }
}

use anyhow::{Context, Result};
use chrono::{Datelike, FixedOffset, TimeZone, Utc};
use reqwest::Client;
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::dedup::{new_story, story_source_from_item};
use crate::llm::{EventRelation, LlmProvider, MockLlm, OpenAiLlm};
use crate::models::{
    CreatorPack, FeedPayload, RawItem, RunStats, Score, SourceKind, SourceSpec, Story,
};
use crate::notify;
use crate::sources;
use crate::store::{Store, alert_from_story};

pub struct Pipeline {
    pub cfg: AppConfig,
    pub store: Store,
    pub client: Client,
    pub llm: Box<dyn LlmProvider>,
    pub only_source: Option<String>,
}

impl Pipeline {
    pub async fn new(cfg: AppConfig, fixtures: bool) -> Result<Self> {
        let store = Store::open(cfg.db_path()).await?;
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("ainews/0.1")
            .build()?;
        let llm: Box<dyn LlmProvider> = if cfg.llm_available() && !fixtures {
            Box::new(OpenAiLlm::from_config(&cfg.ai)?)
        } else {
            if !fixtures {
                tracing::warn!(
                    "LLM is not configured or unavailable; collection will use deterministic mock scoring"
                );
            }
            Box::new(MockLlm::new())
        };
        Ok(Self {
            cfg,
            store,
            client,
            llm,
            only_source: None,
        })
    }

    pub async fn run(&self, fixtures: bool, with_brief: bool, slot: &str) -> Result<RunStats> {
        let started_at = Utc::now();
        let mut run = RunStats {
            id: Uuid::new_v4().to_string(),
            started_at,
            status: "running".into(),
            ..RunStats::default()
        };
        let disabled = self.store.failed_source_ids(5).await?;
        let specs: Vec<SourceSpec> = self
            .cfg
            .load_sources()
            .unwrap_or_default()
            .into_iter()
            .filter(|spec| {
                !disabled.contains(&spec.id)
                    && self
                        .only_source
                        .as_ref()
                        .map(|only| spec.id == *only || spec.kind.as_str() == only)
                        .unwrap_or(true)
            })
            .collect();
        self.store.upsert_sources(&specs).await?;
        let source_map: HashMap<String, (String, u8)> = specs
            .iter()
            .map(|s| (s.id.clone(), (s.name.clone(), s.tier)))
            .collect();

        let mut any_source_healthy = fixtures;
        let mut items = if fixtures {
            load_fixtures(&self.cfg.app.fixture_file)?
        } else {
            let collected = sources::collect_all(&self.cfg, &specs, &self.client).await;
            let mut all = Vec::new();
            for (health, fetched) in collected {
                any_source_healthy |= health.ok;
                self.store.record_source_health(&health).await?;
                all.extend(fetched);
            }
            sources::dedupe_items(all)
        };
        if !any_source_healthy && !specs.is_empty() {
            anyhow::bail!("all selected sources failed");
        }
        run.fetched = items.len();
        for item in &mut items {
            item.ensure_derived();
        }

        let mut new_stories = Vec::new();
        for item in &items {
            self.store.upsert_item(item).await?;
            let (source_name, tier) = source_map
                .get(&item.source_id)
                .cloned()
                .unwrap_or_else(|| (item.source_id.clone(), 3));
            let existing = self
                .store
                .find_item_story(
                    &item.canonical_url,
                    item.simhash,
                    &item.title,
                    self.cfg.pipeline.dedup_days,
                    self.cfg.pipeline.simhash_hamming,
                    self.cfg.pipeline.title_jaccard,
                )
                .await?;
            if let Some(mut story) = existing {
                if !story.sources.iter().any(|source| source.item_id == item.id) {
                    story
                        .sources
                        .push(story_source_from_item(item, &source_name, tier));
                    story.last_seen_at = item.fetched_at;
                    story.version += 1;
                    story.status = "updated".into();
                    if story.title_zh.is_empty() {
                        story.title_zh =
                            item.title_zh.clone().unwrap_or_else(|| item.title.clone());
                    }
                    if story.title_en.is_empty() {
                        story.title_en =
                            item.title_en.clone().unwrap_or_else(|| item.title.clone());
                    }
                    self.store.insert_story(&story).await?;
                }
                self.store
                    .link_item(&story.id, &item.id, 1.0, false)
                    .await?;
            } else {
                let mut story = new_story(item, &source_name, tier);
                fill_story_from_sources(&mut story);
                new_stories.push(story.clone());
                self.store.insert_story(&story).await?;
                self.store.link_item(&story.id, &item.id, 1.0, true).await?;
            }
        }
        run.stories = new_stories.len();

        // Stage-2 hook: decisions are recorded through the LLM provider. The
        // deterministic URL/SimHash pass already prevents the common cases.
        let recent_for_merge = self.store.recent_stories(80, 0.0).await?;
        if !recent_for_merge.is_empty() {
            match self.llm.merge_events(&recent_for_merge).await {
                Ok(decisions) => {
                    for decision in decisions {
                        if decision.relation != EventRelation::SameEvent {
                            continue;
                        }
                        if let Some(target) = decision.target_story_id {
                            if let Err(error) =
                                self.store.merge_stories(&target, &decision.story_id).await
                            {
                                tracing::warn!(%error, "stage-2 merge failed");
                            }
                        }
                    }
                }
                Err(error) => tracing::warn!(%error, "stage-2 event merge skipped"),
            }
        }

        let pending = self
            .store
            .stories_needing_score(self.cfg.ai.max_items_per_run)
            .await?;
        let mut scored = 0usize;
        if !pending.is_empty() {
            match self.llm.score_stories(&pending).await {
                Ok(scores) => {
                    let score_map: HashMap<String, Score> = scores
                        .into_iter()
                        .map(|s| (s.story_id.clone(), s))
                        .collect();
                    for mut story in pending {
                        if let Some(mut score) = score_map.get(&story.id).cloned() {
                            apply_score_caps(&story, &mut score, &self.cfg);
                            fill_story_from_score(&mut story, &score);
                            self.store.save_score(&mut story, &score).await?;
                            scored += 1;
                        }
                    }
                }
                Err(error) => {
                    tracing::error!(%error, "LLM scoring failed; falling back to mock scores");
                    let mock = MockLlm::new();
                    if let Ok(scores) = mock.score_stories(&pending).await {
                        for mut story in pending {
                            if let Some(mut score) =
                                scores.iter().find(|s| s.story_id == story.id).cloned()
                            {
                                apply_score_caps(&story, &mut score, &self.cfg);
                                fill_story_from_score(&mut story, &score);
                                self.store.save_score(&mut story, &score).await?;
                                scored += 1;
                            }
                        }
                    }
                }
            }
        }
        run.scored = scored;

        let alert_candidates = self
            .store
            .recent_stories(
                self.cfg.pipeline.max_alerts_per_run * 10,
                self.cfg.pipeline.alert_score,
            )
            .await?;
        let mut alerts_sent = 0usize;
        for story in alert_candidates {
            if alerts_sent >= self.cfg.pipeline.max_alerts_per_run {
                break;
            }
            if story.confidence < self.cfg.pipeline.alert_confidence {
                continue;
            }
            let alert = alert_from_story(&story);
            let dedup_key = format!("{}:{}", alert.story_id, alert.material_hash);
            if self.store.was_alerted(&dedup_key, 24).await? {
                continue;
            }
            let receipts = notify::send_alert(&self.cfg.delivery, &alert).await;
            if receipts.is_empty() {
                tracing::info!(story = %story.id, "alert threshold met but no delivery channel enabled");
                continue;
            }
            for receipt in &receipts {
                self.store
                    .record_delivery(Some(&story.id), None, receipt, "alert", &dedup_key)
                    .await?;
            }
            alerts_sent += 1;
        }
        run.alerts = alerts_sent;

        let date = local_date();
        if with_brief {
            let top = self
                .store
                .recent_stories(40, self.cfg.pipeline.digest_score)
                .await?;
            let action_plan = self.llm.plan_actions(&date, &top).await?;
            self.store.save_action_plan(&date, &action_plan).await?;
            self.write_actions_public(&action_plan)?;
            let mut brief_count = 0usize;
            let mut briefs = Vec::new();
            let mut creator_pack: Option<CreatorPack> = None;
            let mut creator_pack_loaded = false;
            for lang in &self.cfg.pipeline.languages {
                let mut brief = self
                    .llm
                    .write_brief(&date, slot, lang, &top, Some(&action_plan))
                    .await?;
                if brief.creator_pack.is_none() {
                    if !creator_pack_loaded {
                        creator_pack = self.llm.write_creator_pack(&date, &brief, &top).await?;
                        creator_pack_loaded = true;
                    }
                    brief.creator_pack = creator_pack.clone();
                } else if !creator_pack_loaded {
                    creator_pack = brief.creator_pack.clone();
                    creator_pack_loaded = true;
                }
                self.store.save_brief(&brief).await?;
                self.write_brief_public(&brief)?;
                briefs.push(brief);
                brief_count += 1;
            }
            let _ = notify::send_digest(&self.cfg.delivery, &briefs).await;
            run.briefs = brief_count;
        }

        run.finished_at = Some(Utc::now());
        run.status = "success".into();
        self.store.save_run(&run).await?;
        self.export(&run).await?;
        Ok(run)
    }

    pub async fn brief_only(&self, slot: &str) -> Result<RunStats> {
        let started_at = Utc::now();
        let date = local_date();
        let previous = self.store.last_run().await?.unwrap_or_default();
        let top = self
            .store
            .recent_stories(40, self.cfg.pipeline.digest_score)
            .await?;
        let action_plan = self.llm.plan_actions(&date, &top).await?;
        self.store.save_action_plan(&date, &action_plan).await?;
        self.write_actions_public(&action_plan)?;
        let mut run = RunStats {
            id: Uuid::new_v4().to_string(),
            started_at,
            status: "running".into(),
            fetched: previous.fetched,
            stories: previous.stories,
            scored: previous.scored,
            ..RunStats::default()
        };
        let mut briefs = Vec::new();
        let mut creator_pack: Option<CreatorPack> = None;
        let mut creator_pack_loaded = false;
        for lang in &self.cfg.pipeline.languages {
            let mut brief = self
                .llm
                .write_brief(&date, slot, lang, &top, Some(&action_plan))
                .await?;
            if brief.creator_pack.is_none() {
                if !creator_pack_loaded {
                    creator_pack = self.llm.write_creator_pack(&date, &brief, &top).await?;
                    creator_pack_loaded = true;
                }
                brief.creator_pack = creator_pack.clone();
            } else if !creator_pack_loaded {
                creator_pack = brief.creator_pack.clone();
                creator_pack_loaded = true;
            }
            self.store.save_brief(&brief).await?;
            self.write_brief_public(&brief)?;
            briefs.push(brief);
            run.briefs += 1;
        }
        let _ = notify::send_digest(&self.cfg.delivery, &briefs).await;
        run.finished_at = Some(Utc::now());
        run.status = "success".into();
        self.store.save_run(&run).await?;
        self.export(&run).await?;
        Ok(run)
    }

    pub async fn export(&self, run: &RunStats) -> Result<()> {
        let public_dir = self.cfg.public_dir();
        tokio::fs::create_dir_all(&public_dir).await?;
        let items = self.store.recent_stories(500, 0.0).await?;
        let sources = self.store.source_health(500).await?;
        let briefs = self.store.list_briefs(50).await?;
        for brief in self.store.recent_briefs(briefs.len()).await? {
            self.write_brief_public(&brief)?;
        }
        let action_plan = self
            .store
            .latest_action_plan()
            .await?
            .filter(|plan| plan.date == local_date());
        if let Some(plan) = &action_plan {
            write_json(&public_dir.join("actions.json"), plan).await?;
        }
        let payload = FeedPayload {
            schema_version: 1,
            generated_at: Utc::now(),
            total_items: items.len(),
            items,
            sources: sources.clone(),
            briefs,
            action_plan,
            run: run.clone(),
        };
        write_json(&public_dir.join("latest.json"), &payload).await?;
        write_json(&public_dir.join("sources.json"), &sources).await?;
        let runs_dir = public_dir.join("runs");
        tokio::fs::create_dir_all(&runs_dir).await?;
        write_json(&runs_dir.join("latest.json"), run).await?;
        Ok(())
    }

    fn write_brief_public(&self, brief: &crate::models::Brief) -> Result<()> {
        let dir = self.cfg.public_dir().join("briefs");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}-{}-{}.json", brief.date, brief.slot, brief.lang));
        std::fs::write(&path, serde_json::to_vec_pretty(brief)?)?;
        Ok(())
    }

    fn write_actions_public(&self, plan: &crate::models::ActionPlan) -> Result<()> {
        std::fs::create_dir_all(self.cfg.public_dir())?;
        std::fs::write(
            self.cfg.public_dir().join("actions.json"),
            serde_json::to_vec_pretty(plan)?,
        )?;
        Ok(())
    }
}

fn fill_story_from_sources(story: &mut Story) {
    let primary = story.sources.first();
    let title = primary.map(|s| s.title.as_str()).unwrap_or("");
    if story.title_zh.is_empty() {
        story.title_zh = title.into();
    }
    if story.title_en.is_empty() {
        story.title_en = title.into();
    }
    let excerpt = primary.map(|s| s.excerpt.as_str()).unwrap_or("");
    if story.summary_zh.is_empty() {
        story.summary_zh = excerpt.chars().take(240).collect();
    }
    if story.summary_en.is_empty() {
        story.summary_en = excerpt.chars().take(240).collect();
    }
    if story.community.is_empty()
        && story.sources.iter().any(|s| {
            matches!(
                s.kind,
                SourceKind::HackerNews | SourceKind::Reddit | SourceKind::X
            )
        })
    {
        story.community = excerpt.chars().take(500).collect();
    }
}

fn fill_story_from_score(story: &mut Story, score: &Score) {
    story.score = score.total;
    story.confidence = score.confidence;
    story.breakdown = score.breakdown();
    if let Some(title) = &score.title_zh {
        if !title.trim().is_empty() {
            story.title_zh = title.clone();
        }
    }
    if let Some(title) = &score.title_en {
        if !title.trim().is_empty() {
            story.title_en = title.clone();
        }
    }
    if let Some(summary) = &score.summary_zh {
        if !summary.trim().is_empty() {
            story.summary_zh = summary.clone();
        }
    }
    if let Some(summary) = &score.summary_en {
        if !summary.trim().is_empty() {
            story.summary_en = summary.clone();
        }
    }
    if let Some(tip) = &score.user_tip_zh {
        if !tip.trim().is_empty() {
            story.user_tip_zh = tip.clone();
        }
    }
    if let Some(tip) = &score.user_tip_en {
        if !tip.trim().is_empty() {
            story.user_tip_en = tip.clone();
        }
    }
    if story.why_it_matters.is_empty() {
        story.why_it_matters = "这条消息可能影响你正在用的 AI 工具、价格或工作方式。".into();
    }
    if story.why_it_matters_en.is_empty() {
        story.why_it_matters_en =
            "This may change the AI tools, prices or workflows you already use.".into();
    }
    if story.summary_zh.is_empty() {
        story.summary_zh = story.title_zh.clone();
    }
    if story.summary_en.is_empty() {
        story.summary_en = story.title_en.clone();
    }
    story.status = "scored".into();
}

fn apply_score_caps(story: &Story, score: &mut Score, cfg: &AppConfig) {
    let text = format!(
        "{} {} {}",
        story.title_zh,
        story.title_en,
        story
            .sources
            .iter()
            .map(|s| s.excerpt.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    )
    .to_ascii_lowercase();
    let ai = [
        "ai",
        "llm",
        "gpt",
        "claude",
        "gemini",
        "agent",
        "model",
        "大模型",
        "人工智能",
    ]
    .iter()
    .any(|k| text.contains(k));
    let official = story.sources.iter().any(|s| s.tier == 1);
    let independent = story.sources.len() >= 2;
    let mut cap: f64 = 100.0;
    if !ai {
        cap = cap.min(59.0);
    }
    if !official && !independent && score.confidence < cfg.pipeline.alert_confidence {
        cap = cap.min(69.0);
    }
    if story.sources.is_empty() || story.sources.iter().all(|s| s.url.is_empty()) {
        cap = cap.min(50.0);
    }
    if !official && score.total > 89.0 {
        cap = cap.min(89.0);
    }
    score.total = score.total.min(cap).clamp(0.0, 100.0);
}

fn load_fixtures(path: impl AsRef<Path>) -> Result<Vec<RawItem>> {
    let content = std::fs::read_to_string(path.as_ref())
        .with_context(|| format!("read fixtures {}", path.as_ref().display()))?;
    let mut items: Vec<RawItem> = serde_json::from_str(&content)?;
    let now = Utc::now();
    for item in &mut items {
        if item.fetched_at.timestamp() == 0 {
            item.fetched_at = now;
        }
        item.ensure_derived();
    }
    Ok(items)
}

async fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let bytes = serde_json::to_vec_pretty(value)?;
    tokio::fs::write(path, bytes).await?;
    Ok(())
}

pub fn local_date() -> String {
    let tz = FixedOffset::east_opt(8 * 3600).unwrap();
    let now = tz.from_utc_datetime(&Utc::now().naive_utc());
    format!("{:04}-{:02}-{:02}", now.year(), now.month(), now.day())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::StorySource;

    fn story_with_url(url: &str, tier: u8) -> Story {
        Story {
            id: "test-story".into(),
            primary_item_id: "item".into(),
            title_zh: "OpenAI 发布新模型".into(),
            title_en: "OpenAI releases a new model".into(),
            summary_zh: "官方发布新的 AI 模型。".into(),
            summary_en: "An official AI model release.".into(),
            first_seen_at: Utc::now(),
            last_seen_at: Utc::now(),
            sources: vec![StorySource {
                item_id: "item".into(),
                source_id: "source".into(),
                source_name: "Source".into(),
                kind: SourceKind::Rss,
                tier,
                title: "OpenAI releases a new model".into(),
                url: url.into(),
                author: "OpenAI".into(),
                published_at: Utc::now(),
                excerpt: "An official AI model release.".into(),
                score: None,
            }],
            ..Story::default()
        }
    }

    fn high_score() -> Score {
        Score {
            story_id: "test-story".into(),
            relevance: 30.0,
            impact: 25.0,
            novelty: 15.0,
            credibility: 15.0,
            community: 5.0,
            actionability: 5.0,
            total: 95.0,
            confidence: 0.92,
            ..Score::default()
        }
    }

    #[test]
    fn non_ai_is_capped_below_sixty() {
        let mut story = story_with_url("https://example.com/css", 3);
        story.title_zh = "前端 CSS reset".into();
        story.title_en = "A frontend CSS reset".into();
        story.summary_zh = "新的浏览器样式重置。".into();
        story.summary_en = "A new browser CSS reset.".into();
        story.sources[0].title = "A frontend CSS reset".into();
        story.sources[0].excerpt = "A new browser CSS reset.".into();
        let mut score = high_score();
        apply_score_caps(&story, &mut score, &AppConfig::default());
        assert!(score.total <= 59.0);
    }

    #[test]
    fn official_release_can_cross_ninety() {
        let story = story_with_url("https://openai.com/model", 1);
        let mut score = high_score();
        apply_score_caps(&story, &mut score, &AppConfig::default());
        assert_eq!(score.total, 95.0);
    }

    #[test]
    fn missing_link_is_capped() {
        let story = story_with_url("", 1);
        let mut score = high_score();
        apply_score_caps(&story, &mut score, &AppConfig::default());
        assert!(score.total <= 50.0);
    }
}

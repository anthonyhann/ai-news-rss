use anyhow::{Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use futures::stream::{self, StreamExt};
use reqwest::Client;
use scraper::{Html, Selector};
use serde_json::{Value, json};
use std::collections::HashSet;

use crate::config::AppConfig;
use crate::dedup::{normalize_url, simhash64};
use crate::models::{RawItem, SourceHealth, SourceKind, SourceSpec};

pub mod x;

pub async fn collect_all(
    cfg: &AppConfig,
    specs: &[SourceSpec],
    client: &Client,
) -> Vec<(SourceHealth, Vec<RawItem>)> {
    let selected: Vec<SourceSpec> = specs.iter().filter(|s| s.enabled).cloned().collect();
    stream::iter(selected)
        .map(|spec| {
            let cfg = cfg.clone();
            let client = client.clone();
            async move { collect_one(&cfg, &spec, &client).await }
        })
        .buffer_unordered(6)
        .collect()
        .await
}

pub async fn collect_one(
    cfg: &AppConfig,
    spec: &SourceSpec,
    client: &Client,
) -> (SourceHealth, Vec<RawItem>) {
    let now = Utc::now();
    let result = match spec.kind {
        SourceKind::Rss | SourceKind::Wechat | SourceKind::Arxiv | SourceKind::HuggingFace => {
            fetch_rss(spec, client).await
        }
        SourceKind::HackerNews => fetch_hackernews(spec, client).await,
        SourceKind::Reddit => fetch_reddit(spec, client).await,
        SourceKind::Github => fetch_github(spec, client).await,
        SourceKind::OssInsight => fetch_ossinsight(spec, client).await,
        SourceKind::Telegram => fetch_telegram(spec, client).await,
        SourceKind::Youtube => fetch_youtube(spec, client).await,
        SourceKind::X => x::fetch_x(cfg, spec).await,
        SourceKind::Fixture => Ok(vec![]),
    };
    match result {
        Ok(items) => {
            let health = SourceHealth {
                source_id: spec.id.clone(),
                name: spec.name.clone(),
                kind: spec.kind,
                tier: spec.tier,
                enabled: spec.enabled,
                ok: true,
                item_count: items.len(),
                error: None,
                checked_at: now,
            };
            (health, items)
        }
        Err(error) => {
            tracing::warn!(source = %spec.id, %error, "source failed");
            let health = SourceHealth {
                source_id: spec.id.clone(),
                name: spec.name.clone(),
                kind: spec.kind,
                tier: spec.tier,
                enabled: spec.enabled,
                ok: false,
                item_count: 0,
                error: Some(format!("{error:#}")),
                checked_at: now,
            };
            (health, vec![])
        }
    }
}

pub async fn fetch_rss(spec: &SourceSpec, client: &Client) -> Result<Vec<RawItem>> {
    let url = spec.target_url().context("RSS source needs url")?;
    let response = client
        .get(url)
        .header("User-Agent", "ainews/0.1 (+https://github.com/ai-news-rss)")
        .send()
        .await?
        .error_for_status()?;
    let bytes = response.bytes().await?;
    let feed = feed_rs::parser::parse(bytes.as_ref())?;
    let now = Utc::now();
    let mut items = Vec::new();
    for entry in feed.entries.into_iter().take(spec.max_items) {
        let title = entry
            .title
            .map(|t| t.content)
            .unwrap_or_else(|| "Untitled".into());
        if title.trim().is_empty() {
            continue;
        }
        let url = entry
            .links
            .iter()
            .find(|link| link.rel.as_deref().is_none_or(|r| r == "alternate"))
            .map(|link| link.href.clone())
            .or_else(|| entry.links.first().map(|link| link.href.clone()))
            .unwrap_or_default();
        if url.is_empty() {
            continue;
        }
        let html = entry
            .content
            .and_then(|c| c.body)
            .or_else(|| entry.summary.map(|s| s.content))
            .unwrap_or_default();
        let content = html_to_text(&html);
        let published_at = entry
            .published
            .or(entry.updated)
            .and_then(|d| DateTime::<Utc>::from_naive_utc_and_offset(d.naive_utc(), Utc).into())
            .unwrap_or(now);
        let author = entry
            .authors
            .first()
            .and_then(|p| p.name.clone())
            .unwrap_or_else(|| spec.name.clone());
        let tags: Vec<String> = entry
            .categories
            .iter()
            .map(|c| c.term.clone())
            .filter(|c| !c.is_empty())
            .take(5)
            .collect();
        let mut item = RawItem {
            id: format!("{}:{}", spec.id, entry.id),
            source_id: spec.id.clone(),
            kind: spec.kind,
            title: title.clone(),
            url: url.clone(),
            canonical_url: normalize_url(&url),
            author,
            published_at,
            fetched_at: now,
            content,
            excerpt: String::new(),
            lang: spec.lang.clone(),
            title_zh: None,
            title_en: None,
            summary_zh: None,
            summary_en: None,
            score: None,
            simhash: 0,
            metadata: json!({
                "feed_name": spec.name,
                "category": spec.category,
                "tags": tags,
            }),
        };
        if spec
            .connector
            .get("fetch_full_text")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            if let Ok(response) = client.get(&url).send().await {
                if let Ok(html) = response.text().await {
                    if let Some(full) = extract_article(&html, &url) {
                        item.content = full;
                    }
                }
            }
        }
        item.ensure_derived();
        items.push(item);
    }
    Ok(items)
}

pub async fn fetch_hackernews(spec: &SourceSpec, client: &Client) -> Result<Vec<RawItem>> {
    let base = "https://hacker-news.firebaseio.com/v0";
    let ids: Vec<u64> = client
        .get(format!("{base}/topstories.json"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let now = Utc::now();
    let ids: Vec<u64> = ids.into_iter().take(spec.max_items.max(1)).collect();
    let mut out = Vec::new();
    for id in ids {
        let Ok(response) = client.get(format!("{base}/item/{id}.json")).send().await else {
            continue;
        };
        let Ok(story) = response.json::<Value>().await else {
            continue;
        };
        let Some(title) = story.get("title").and_then(Value::as_str) else {
            continue;
        };
        let score = story.get("score").and_then(Value::as_u64).unwrap_or(0);
        if score < 60 {
            continue;
        }
        let published_at = story
            .get("time")
            .and_then(Value::as_i64)
            .and_then(|ts| Utc.timestamp_opt(ts, 0).single())
            .unwrap_or(now);
        let url = story
            .get("url")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| format!("https://news.ycombinator.com/item?id={id}"));
        let mut content = story
            .get("text")
            .and_then(Value::as_str)
            .map(html_to_text)
            .unwrap_or_default();
        let kids: Vec<u64> = story
            .get("kids")
            .and_then(Value::as_array)
            .map(|v| v.iter().filter_map(Value::as_u64).take(3).collect())
            .unwrap_or_default();
        if !kids.is_empty() {
            content.push_str("\n\n--- Top Comments ---\n");
            for kid in kids {
                if let Ok(resp) = client.get(format!("{base}/item/{kid}.json")).send().await {
                    if let Ok(comment) = resp.json::<Value>().await {
                        if let Some(text) = comment.get("text").and_then(Value::as_str) {
                            content.push_str(&format!(
                                "[{}]: {}\n",
                                comment.get("by").and_then(Value::as_str).unwrap_or("anon"),
                                html_to_text(text).chars().take(500).collect::<String>()
                            ));
                        }
                    }
                }
            }
        }
        let mut item = RawItem {
            id: format!("{}:hn:{id}", spec.id),
            source_id: spec.id.clone(),
            kind: SourceKind::HackerNews,
            title: title.into(),
            url: url.clone(),
            canonical_url: normalize_url(&url),
            author: story
                .get("by")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .into(),
            published_at,
            fetched_at: now,
            content,
            excerpt: String::new(),
            lang: "en".into(),
            title_zh: None,
            title_en: None,
            summary_zh: None,
            summary_en: None,
            score: None,
            simhash: 0,
            metadata: json!({
                "score": score,
                "descendants": story.get("descendants").and_then(Value::as_u64),
                "discussion_url": format!("https://news.ycombinator.com/item?id={id}"),
                "category": "community",
            }),
        };
        item.ensure_derived();
        out.push(item);
    }
    Ok(out)
}

pub async fn fetch_reddit(spec: &SourceSpec, client: &Client) -> Result<Vec<RawItem>> {
    let subreddit = spec
        .subreddit
        .clone()
        .or_else(|| {
            spec.connector
                .get("subreddit")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| "LocalLLaMA".into());
    let url = format!(
        "https://www.reddit.com/r/{subreddit}/hot.json?limit={}",
        spec.max_items
    );
    let response = client
        .get(&url)
        .header("User-Agent", "ainews/0.1")
        .send()
        .await?;
    if !response.status().is_success() {
        return fetch_rss(
            &SourceSpec {
                url: Some(format!("https://www.reddit.com/r/{subreddit}/hot/.rss")),
                kind: SourceKind::Rss,
                ..spec.clone()
            },
            client,
        )
        .await;
    }
    let value: Value = response.json().await?;
    let now = Utc::now();
    let mut out = Vec::new();
    for child in value
        .pointer("/data/children")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let data = &child["data"];
        let Some(title) = data.get("title").and_then(Value::as_str) else {
            continue;
        };
        let score = data.get("score").and_then(Value::as_i64).unwrap_or(0);
        if score < 10 {
            continue;
        }
        let permalink = data
            .get("permalink")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let url = format!("https://www.reddit.com{permalink}");
        let published_at = data
            .get("created_utc")
            .and_then(Value::as_f64)
            .and_then(|ts| Utc.timestamp_opt(ts as i64, 0).single())
            .unwrap_or(now);
        let body = data
            .get("selftext")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let mut item = RawItem {
            id: format!(
                "{}:reddit:{}",
                spec.id,
                data.get("id").and_then(Value::as_str).unwrap_or("unknown")
            ),
            source_id: spec.id.clone(),
            kind: SourceKind::Reddit,
            title: title.into(),
            url: url.clone(),
            canonical_url: normalize_url(&url),
            author: data
                .get("author")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .into(),
            published_at,
            fetched_at: now,
            content: body.into(),
            excerpt: String::new(),
            lang: "en".into(),
            title_zh: None,
            title_en: None,
            summary_zh: None,
            summary_en: None,
            score: None,
            simhash: 0,
            metadata: json!({
                "subreddit": subreddit,
                "score": score,
                "num_comments": data.get("num_comments"),
                "discussion_url": url,
                "category": "community",
            }),
        };
        item.ensure_derived();
        out.push(item);
    }
    Ok(out)
}

pub async fn fetch_github(spec: &SourceSpec, client: &Client) -> Result<Vec<RawItem>> {
    let owner = spec.owner.clone().or_else(|| {
        spec.connector
            .get("owner")
            .and_then(Value::as_str)
            .map(str::to_string)
    });
    let repo = spec.repo.clone().or_else(|| {
        spec.connector
            .get("repo")
            .and_then(Value::as_str)
            .map(str::to_string)
    });
    let (Some(owner), Some(repo)) = (owner, repo) else {
        return fetch_rss(
            &SourceSpec {
                url: Some("https://mshibanami.github.io/GitHubTrendingRSS/daily/all.xml".into()),
                kind: SourceKind::Rss,
                ..spec.clone()
            },
            client,
        )
        .await;
    };
    let mut request = client.get(format!(
        "https://api.github.com/repos/{owner}/{repo}/releases?per_page={}",
        spec.max_items
    ));
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if !token.trim().is_empty() {
            request = request.bearer_auth(token);
        }
    }
    let releases: Vec<Value> = request
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let now = Utc::now();
    let mut out = Vec::new();
    for release in releases {
        let title = release
            .get("name")
            .and_then(Value::as_str)
            .or_else(|| release.get("tag_name").and_then(Value::as_str))
            .unwrap_or("GitHub release");
        let url = release
            .get("html_url")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if url.is_empty() {
            continue;
        }
        let body = release
            .get("body")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let published_at = release
            .get("published_at")
            .and_then(Value::as_str)
            .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
            .map(|v| v.with_timezone(&Utc))
            .unwrap_or(now);
        let mut item = RawItem {
            id: format!(
                "{}:gh:{}",
                spec.id,
                release
                    .get("id")
                    .and_then(Value::as_i64)
                    .unwrap_or_default()
            ),
            source_id: spec.id.clone(),
            kind: SourceKind::Github,
            title: format!("{owner}/{repo}: {title}"),
            url: url.into(),
            canonical_url: normalize_url(url),
            author: owner.clone(),
            published_at,
            fetched_at: now,
            content: body.into(),
            excerpt: String::new(),
            lang: "en".into(),
            title_zh: None,
            title_en: None,
            summary_zh: None,
            summary_en: None,
            score: None,
            simhash: 0,
            metadata: json!({
                "repo": format!("{owner}/{repo}"),
                "category": "open-source",
                "tag": release.get("tag_name"),
            }),
        };
        item.ensure_derived();
        out.push(item);
    }
    Ok(out)
}

pub async fn fetch_ossinsight(spec: &SourceSpec, client: &Client) -> Result<Vec<RawItem>> {
    let url = "https://api.ossinsight.io/v1/trends/repos/?period=past_24_hours&language=All";
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return fetch_rss(
            &SourceSpec {
                url: Some("https://mshibanami.github.io/GitHubTrendingRSS/daily/all.xml".into()),
                kind: SourceKind::Rss,
                ..spec.clone()
            },
            client,
        )
        .await;
    }
    let value: Value = response.json().await?;
    let rows = value
        .pointer("/data/rows")
        .or_else(|| value.get("data"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let now = Utc::now();
    let mut out = Vec::new();
    for row in rows.into_iter().take(spec.max_items) {
        let repo = row
            .get("repo_name")
            .or_else(|| row.get("name"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        if repo.is_empty() {
            continue;
        }
        let description = row
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("OSS Insight trending repository");
        let url = format!("https://github.com/{repo}");
        let mut item = RawItem {
            id: format!("{}:oss:{repo}", spec.id),
            source_id: spec.id.clone(),
            kind: SourceKind::OssInsight,
            title: format!("Trending: {repo}"),
            url: url.clone(),
            canonical_url: normalize_url(&url),
            author: "OSS Insight".into(),
            published_at: now,
            fetched_at: now,
            content: description.into(),
            excerpt: String::new(),
            lang: "en".into(),
            title_zh: None,
            title_en: None,
            summary_zh: None,
            summary_en: None,
            score: None,
            simhash: 0,
            metadata: json!({"category": "open-source", "row": row}),
        };
        item.ensure_derived();
        out.push(item);
    }
    Ok(out)
}

pub async fn fetch_telegram(spec: &SourceSpec, client: &Client) -> Result<Vec<RawItem>> {
    let channel = spec
        .channel
        .clone()
        .or_else(|| {
            spec.connector
                .get("channel")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .context("Telegram source needs channel")?;
    let url = format!("https://t.me/s/{channel}");
    let html = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let document = Html::parse_document(&html);
    let post_sel = Selector::parse(".tgme_widget_message").unwrap();
    let text_sel = Selector::parse(".tgme_widget_message_text").unwrap();
    let link_sel = Selector::parse("a.tgme_widget_message_date").unwrap();
    let time_sel = Selector::parse("time").unwrap();
    let now = Utc::now();
    let mut out = Vec::new();
    for post in document.select(&post_sel) {
        let Some(text_node) = post.select(&text_sel).next() else {
            continue;
        };
        let text = text_node
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();
        if text.is_empty() {
            continue;
        }
        let link = post
            .select(&link_sel)
            .next()
            .and_then(|a| a.value().attr("href"))
            .map(str::to_string)
            .unwrap_or_else(|| format!("https://t.me/{channel}"));
        let published_at = post
            .select(&time_sel)
            .next()
            .and_then(|t| t.value().attr("datetime"))
            .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
            .map(|v| v.with_timezone(&Utc))
            .unwrap_or(now);
        let post_id = post
            .value()
            .attr("data-post")
            .unwrap_or("unknown")
            .replace('/', "-");
        let title = text
            .lines()
            .next()
            .unwrap_or("Telegram post")
            .chars()
            .take(120)
            .collect();
        let mut item = RawItem {
            id: format!("{}:tg:{post_id}", spec.id),
            source_id: spec.id.clone(),
            kind: SourceKind::Telegram,
            title,
            url: link.clone(),
            canonical_url: normalize_url(&link),
            author: channel.clone(),
            published_at,
            fetched_at: now,
            content: text,
            excerpt: String::new(),
            lang: spec.lang.clone(),
            title_zh: None,
            title_en: None,
            summary_zh: None,
            summary_en: None,
            score: None,
            simhash: 0,
            metadata: json!({"channel": channel, "category": "community"}),
        };
        item.ensure_derived();
        out.push(item);
        if out.len() >= spec.max_items {
            break;
        }
    }
    Ok(out)
}

pub async fn fetch_youtube(spec: &SourceSpec, client: &Client) -> Result<Vec<RawItem>> {
    let url = spec
        .target_url()
        .context("YouTube source needs channel feed URL")?;
    let mut items = fetch_rss(
        &SourceSpec {
            url: Some(url.to_string()),
            kind: SourceKind::Rss,
            ..spec.clone()
        },
        client,
    )
    .await?;
    let transcript_enabled = spec
        .connector
        .get("transcript")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if transcript_enabled {
        for item in &mut items {
            if let Ok(transcript) = fetch_youtube_transcript(&item.url).await {
                if !transcript.is_empty() {
                    item.content = transcript;
                    item.excerpt = item.content.chars().take(280).collect();
                    item.simhash = simhash64(&format!("{} {}", item.title, item.excerpt));
                }
            }
        }
    }
    Ok(items)
}

async fn fetch_youtube_transcript(url: &str) -> Result<String> {
    let temp = tempfile::tempdir()?;
    let pattern = format!("{}/%(id)s.%(ext)s", temp.path().display());
    let status = tokio::process::Command::new("yt-dlp")
        .args([
            "--skip-download",
            "--write-auto-subs",
            "--write-subs",
            "--sub-langs",
            "en.*,zh.*",
            "--sub-format",
            "vtt",
            "-o",
            &pattern,
            url,
        ])
        .status()
        .await?;
    if !status.success() {
        anyhow::bail!("yt-dlp failed with status {status}");
    }
    let mut files = Vec::new();
    for entry in std::fs::read_dir(temp.path())? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) == Some("vtt") {
            files.push(path);
        }
    }
    files.sort();
    let Some(file) = files.first() else {
        return Ok(String::new());
    };
    let raw = std::fs::read_to_string(file)?;
    Ok(clean_vtt(&raw))
}

fn clean_vtt(raw: &str) -> String {
    let mut out = Vec::new();
    let mut previous = String::new();
    let tag_re = regex::Regex::new(r"<[^>]+>").expect("valid VTT tag regex");
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with("WEBVTT")
            || line.starts_with("Kind:")
            || line.starts_with("Language:")
            || line.contains("-->")
            || line
                .chars()
                .all(|c| c.is_ascii_digit() || c == ':' || c == '.' || c == '-')
        {
            continue;
        }
        let clean = tag_re.replace_all(line, "").to_string();
        if clean != previous && !clean.is_empty() {
            out.push(clean.clone());
            previous = clean;
        }
    }
    out.join(" ")
}

pub fn html_to_text(html: &str) -> String {
    if !html.contains('<') {
        return normalize_whitespace(html);
    }
    let without_scripts =
        regex::Regex::new(r"(?is)<script[^>]*>.*?</script>|<style[^>]*>.*?</style>")
            .expect("valid script stripping regex")
            .replace_all(html, " ");
    let document = Html::parse_fragment(&without_scripts);
    let root = document.root_element();
    let text = root.text().collect::<Vec<_>>().join(" ");
    normalize_whitespace(&text)
}

pub fn extract_article(html: &str, url: &str) -> Option<String> {
    let mut readability = dom_smoothie::Readability::new(html, Some(url), None).ok()?;
    let article = readability.parse().ok()?;
    let text = normalize_whitespace(&article.text_content);
    if text.len() > 80 { Some(text) } else { None }
}

fn normalize_whitespace(value: &str) -> String {
    let re = regex::Regex::new(r"\s+").unwrap();
    re.replace_all(value, " ").trim().to_string()
}

pub fn dedupe_items(items: Vec<RawItem>) -> Vec<RawItem> {
    let mut seen = HashSet::new();
    items
        .into_iter()
        .filter(|item| seen.insert(item.canonical_url.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_html_to_text() {
        assert_eq!(
            html_to_text("<p>Hello <b>world</b></p><script>ignore()</script>"),
            "Hello world"
        );
    }

    #[test]
    fn cleans_vtt_timestamps_and_duplicate_cues() {
        let raw = "WEBVTT\n\n00:00:01.000 --> 00:00:02.000\nHello\n\n00:00:02.000 --> 00:00:03.000\nHello\n\n00:00:03.000 --> 00:00:04.000\nworld\n";
        assert_eq!(clean_vtt(raw), "Hello world");
    }
}

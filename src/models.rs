use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    #[default]
    Rss,
    #[serde(rename = "hackernews", alias = "hacker_news")]
    HackerNews,
    Reddit,
    Github,
    #[serde(rename = "ossinsight", alias = "oss_insight")]
    OssInsight,
    Telegram,
    Youtube,
    Wechat,
    X,
    Arxiv,
    #[serde(rename = "huggingface", alias = "hugging_face")]
    HuggingFace,
    Fixture,
}

impl SourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rss => "rss",
            Self::HackerNews => "hackernews",
            Self::Reddit => "reddit",
            Self::Github => "github",
            Self::OssInsight => "ossinsight",
            Self::Telegram => "telegram",
            Self::Youtube => "youtube",
            Self::Wechat => "wechat",
            Self::X => "x",
            Self::Arxiv => "arxiv",
            Self::HuggingFace => "huggingface",
            Self::Fixture => "fixture",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSpec {
    pub id: String,
    pub kind: SourceKind,
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub list_url: Option<String>,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub subreddit: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default = "default_tier")]
    pub tier: u8,
    #[serde(default = "default_lang")]
    pub lang: String,
    #[serde(default)]
    pub category: String,
    #[serde(default = "default_profile")]
    pub profile: String,
    #[serde(default = "default_weight")]
    pub weight: f64,
    #[serde(default = "default_interval")]
    pub fetch_interval_minutes: u64,
    #[serde(default = "default_max_items")]
    pub max_items: usize,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub connector: Value,
}

fn default_tier() -> u8 {
    3
}
fn default_lang() -> String {
    "en".into()
}
fn default_profile() -> String {
    "ai-news".into()
}
fn default_weight() -> f64 {
    1.0
}
fn default_interval() -> u64 {
    120
}
fn default_max_items() -> usize {
    30
}
fn default_enabled() -> bool {
    true
}

impl SourceSpec {
    pub fn target_url(&self) -> Option<&str> {
        self.list_url.as_deref().or(self.url.as_deref())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RawItem {
    pub id: String,
    pub source_id: String,
    pub kind: SourceKind,
    pub title: String,
    pub url: String,
    #[serde(default)]
    pub canonical_url: String,
    #[serde(default)]
    pub author: String,
    pub published_at: DateTime<Utc>,
    pub fetched_at: DateTime<Utc>,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub excerpt: String,
    #[serde(default = "default_lang")]
    pub lang: String,
    #[serde(default)]
    pub title_zh: Option<String>,
    #[serde(default)]
    pub title_en: Option<String>,
    #[serde(default)]
    pub summary_zh: Option<String>,
    #[serde(default)]
    pub summary_en: Option<String>,
    #[serde(default)]
    pub score: Option<f64>,
    #[serde(default)]
    pub simhash: u64,
    #[serde(default)]
    pub metadata: Value,
}

impl RawItem {
    pub fn ensure_derived(&mut self) {
        if self.canonical_url.is_empty() {
            self.canonical_url = crate::dedup::normalize_url(&self.url);
        }
        if self.excerpt.is_empty() {
            self.excerpt = self.content.chars().take(280).collect();
        }
        if self.simhash == 0 {
            self.simhash = crate::dedup::simhash64(&format!("{} {}", self.title, self.excerpt));
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StorySource {
    pub item_id: String,
    pub source_id: String,
    pub source_name: String,
    pub kind: SourceKind,
    pub tier: u8,
    pub title: String,
    pub url: String,
    pub author: String,
    pub published_at: DateTime<Utc>,
    #[serde(default)]
    pub excerpt: String,
    #[serde(default)]
    pub score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScoreBreakdown {
    pub relevance: f64,
    pub impact: f64,
    pub novelty: f64,
    pub credibility: f64,
    pub community: f64,
    pub actionability: f64,
}

impl ScoreBreakdown {
    pub fn total(&self) -> f64 {
        (self.relevance
            + self.impact
            + self.novelty
            + self.credibility
            + self.community
            + self.actionability)
            .clamp(0.0, 100.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Score {
    pub story_id: String,
    #[serde(default)]
    pub title_zh: Option<String>,
    #[serde(default)]
    pub title_en: Option<String>,
    #[serde(default)]
    pub summary_zh: Option<String>,
    #[serde(default)]
    pub summary_en: Option<String>,
    #[serde(default)]
    pub relevance: f64,
    #[serde(default)]
    pub impact: f64,
    #[serde(default)]
    pub novelty: f64,
    #[serde(default)]
    pub credibility: f64,
    #[serde(default)]
    pub community: f64,
    #[serde(default)]
    pub actionability: f64,
    #[serde(default)]
    pub total: f64,
    #[serde(default)]
    pub confidence: f64,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub prompt_version: String,
}

impl Score {
    pub fn breakdown(&self) -> ScoreBreakdown {
        ScoreBreakdown {
            relevance: self.relevance,
            impact: self.impact,
            novelty: self.novelty,
            credibility: self.credibility,
            community: self.community,
            actionability: self.actionability,
        }
    }

    pub fn recalculate_total(&mut self) {
        self.total = self.breakdown().total();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Story {
    pub id: String,
    pub primary_item_id: String,
    pub title_zh: String,
    pub title_en: String,
    pub summary_zh: String,
    pub summary_en: String,
    #[serde(default)]
    pub why_it_matters: String,
    #[serde(default)]
    pub background: String,
    #[serde(default)]
    pub community: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub score: f64,
    #[serde(default)]
    pub breakdown: ScoreBreakdown,
    #[serde(default)]
    pub confidence: f64,
    #[serde(default = "story_status")]
    pub status: String,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    #[serde(default)]
    pub sources: Vec<StorySource>,
    #[serde(default = "default_version")]
    pub version: i64,
    #[serde(default)]
    pub category: String,
}

fn story_status() -> String {
    "new".into()
}
fn default_version() -> i64 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BriefSection {
    pub title_zh: String,
    pub title_en: String,
    pub items: Vec<TopSignal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TopSignal {
    pub story_id: String,
    pub title_zh: String,
    pub title_en: String,
    pub summary_zh: String,
    pub summary_en: String,
    #[serde(default)]
    pub why_it_matters: String,
    #[serde(default)]
    pub background: String,
    #[serde(default)]
    pub community: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub score: f64,
    pub url: String,
    #[serde(default)]
    pub sources: Vec<StorySource>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActionOption {
    pub title: String,
    pub why_now: String,
    pub impact: f64,
    pub feasibility: f64,
    pub speed: f64,
    pub confidence: f64,
    #[serde(default)]
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActionPlan {
    pub date: String,
    #[serde(default)]
    pub candidates: Vec<ActionOption>,
    pub top_action: Option<ActionOption>,
    #[serde(default)]
    pub experiments: Vec<ActionOption>,
    #[serde(default)]
    pub watchlist: Vec<String>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Brief {
    pub schema_version: u32,
    pub date: String,
    pub slot: String,
    pub lang: String,
    pub executive_summary: Vec<String>,
    pub sections: Vec<BriefSection>,
    #[serde(default)]
    pub top_signals: Vec<TopSignal>,
    #[serde(default)]
    pub action_plan: Option<ActionPlan>,
    #[serde(default)]
    pub markdown: String,
    #[serde(default)]
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Alert {
    pub story_id: String,
    pub level: String,
    pub title_zh: String,
    pub title_en: String,
    pub summary_zh: String,
    pub why_it_matters: String,
    pub url: String,
    pub score: f64,
    pub confidence: f64,
    pub material_hash: String,
    pub sources: Vec<StorySource>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeliveryReceipt {
    pub channel: String,
    pub ok: bool,
    #[serde(default)]
    pub detail: String,
    pub sent_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourceHealth {
    pub source_id: String,
    pub name: String,
    pub kind: SourceKind,
    pub tier: u8,
    pub enabled: bool,
    pub ok: bool,
    pub item_count: usize,
    #[serde(default)]
    pub error: Option<String>,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BriefSummary {
    pub date: String,
    pub slot: String,
    pub lang: String,
    pub title: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunStats {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub status: String,
    pub fetched: usize,
    pub stories: usize,
    pub scored: usize,
    pub alerts: usize,
    pub briefs: usize,
    #[serde(default)]
    pub token_usage: BTreeMap<String, u64>,
    #[serde(default)]
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FeedPayload {
    pub schema_version: u32,
    pub generated_at: DateTime<Utc>,
    pub total_items: usize,
    pub items: Vec<Story>,
    #[serde(default)]
    pub sources: Vec<SourceHealth>,
    #[serde(default)]
    pub briefs: Vec<BriefSummary>,
    #[serde(default)]
    pub action_plan: Option<ActionPlan>,
    #[serde(default)]
    pub run: RunStats,
}

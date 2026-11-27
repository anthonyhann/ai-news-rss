use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::models::SourceSpec;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct AppConfig {
    pub app: AppSection,
    pub ai: AiSection,
    pub pipeline: PipelineSection,
    pub delivery: DeliverySection,
    pub browser: BrowserSection,
    pub server: ServerSection,
    pub retention: RetentionSection,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AppSection {
    pub timezone: String,
    pub data_dir: String,
    pub public_dir: String,
    pub source_dir: String,
    pub profile_dir: String,
    pub fixture_file: String,
}

impl Default for AppSection {
    fn default() -> Self {
        Self {
            timezone: "Asia/Shanghai".into(),
            data_dir: "data".into(),
            public_dir: "web/public/data".into(),
            source_dir: "config/sources".into(),
            profile_dir: "config/profiles".into(),
            fixture_file: "tests/fixtures/items.json".into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AiSection {
    pub base_url: String,
    pub api_key_env: String,
    pub scoring_model: String,
    pub editor_model: String,
    pub temperature: f64,
    pub max_items_per_run: usize,
    pub max_editor_items: usize,
    pub max_tokens_per_run: u64,
    pub request_timeout_seconds: u64,
    pub request_attempts: u32,
    pub prompt_version: String,
}

impl Default for AiSection {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            api_key_env: "LLM_API_KEY".into(),
            scoring_model: String::new(),
            editor_model: String::new(),
            temperature: 0.2,
            max_items_per_run: 300,
            max_editor_items: 30,
            max_tokens_per_run: 350_000,
            request_timeout_seconds: 300,
            request_attempts: 1,
            prompt_version: "v1".into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PipelineSection {
    pub dedup_days: i64,
    pub simhash_hamming: u32,
    pub title_jaccard: f64,
    pub digest_score: f64,
    pub alert_score: f64,
    pub alert_confidence: f64,
    pub max_alerts_per_run: usize,
    pub languages: Vec<String>,
    pub translate_max_new: usize,
}

impl Default for PipelineSection {
    fn default() -> Self {
        Self {
            dedup_days: 7,
            simhash_hamming: 3,
            title_jaccard: 0.72,
            digest_score: 60.0,
            alert_score: 90.0,
            alert_confidence: 0.8,
            max_alerts_per_run: 3,
            languages: vec!["zh".into(), "en".into()],
            translate_max_new: 80,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DeliverySection {
    pub wecom: WebhookChannel,
    pub feishu: WebhookChannel,
    pub dingtalk: WebhookChannel,
    pub telegram: TelegramChannel,
    pub email: EmailChannel,
}

impl Default for DeliverySection {
    fn default() -> Self {
        Self {
            wecom: WebhookChannel {
                enabled: false,
                url_env: "WECOM_WEBHOOK_URL".into(),
            },
            feishu: WebhookChannel {
                enabled: false,
                url_env: "FEISHU_WEBHOOK_URL".into(),
            },
            dingtalk: WebhookChannel {
                enabled: false,
                url_env: "DINGTALK_WEBHOOK_URL".into(),
            },
            telegram: TelegramChannel::default(),
            email: EmailChannel::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct WebhookChannel {
    pub enabled: bool,
    pub url_env: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct TelegramChannel {
    pub enabled: bool,
    pub token_env: String,
    pub chat_id_env: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct EmailChannel {
    pub enabled: bool,
    pub smtp_server: String,
    pub smtp_port: u16,
    pub username: String,
    pub password_env: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct BrowserSection {
    pub provider: String,
    pub headless: bool,
    pub profile_dir: String,
    pub storage_state_env: String,
    pub max_scrolls: usize,
    pub scroll_delay_ms: u64,
    pub timeout_ms: u64,
    pub proxy_env: String,
    pub user_agent: String,
    pub worker_dir: String,
}

impl Default for BrowserSection {
    fn default() -> Self {
        Self {
            provider: "auto".into(),
            headless: true,
            profile_dir: "~/.easel-browser-profiles/TwitterXProfile".into(),
            storage_state_env: "X_STORAGE_STATE_B64".into(),
            max_scrolls: 12,
            scroll_delay_ms: 1800,
            timeout_ms: 45_000,
            proxy_env: "EASEL_PROXY".into(),
            user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/153.0.0.0 Safari/537.36".into(),
            worker_dir: "browser-worker".into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerSection {
    pub bind: String,
    pub admin_token_env: String,
}

impl Default for ServerSection {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:8080".into(),
            admin_token_env: "ADMIN_TOKEN".into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RetentionSection {
    pub items_days: i64,
    pub briefs_days: i64,
    pub deliveries_days: i64,
    pub runs_days: i64,
}

impl Default for RetentionSection {
    fn default() -> Self {
        Self {
            items_days: 30,
            briefs_days: 365,
            deliveries_days: 90,
            runs_days: 30,
        }
    }
}

impl AppConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("read config {}", path.display()))?;
        let expanded = expand_env(&content);
        let mut cfg: Self = toml::from_str(&expanded)
            .with_context(|| format!("parse config {}", path.display()))?;
        cfg.app.data_dir = expand_string(&cfg.app.data_dir);
        cfg.app.public_dir = expand_string(&cfg.app.public_dir);
        cfg.ai.base_url = expand_string(&cfg.ai.base_url);
        cfg.browser.profile_dir = expand_string(&cfg.browser.profile_dir);
        Ok(cfg)
    }

    pub fn source_dir(&self) -> PathBuf {
        PathBuf::from(&self.app.source_dir)
    }

    pub fn data_dir(&self) -> PathBuf {
        PathBuf::from(&self.app.data_dir)
    }

    pub fn public_dir(&self) -> PathBuf {
        PathBuf::from(&self.app.public_dir)
    }

    pub fn db_path(&self) -> PathBuf {
        self.data_dir().join("state.sqlite3")
    }

    pub fn load_sources(&self) -> Result<Vec<SourceSpec>> {
        let dir = self.source_dir();
        let mut specs = Vec::new();
        if !dir.exists() {
            return Ok(specs);
        }
        let mut entries: Vec<_> = std::fs::read_dir(&dir)
            .with_context(|| format!("read source dir {}", dir.display()))?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .and_then(|x| x.to_str())
                    .map(|x| x == "yaml" || x == "yml")
                    .unwrap_or(false)
            })
            .collect();
        entries.sort();
        for path in entries {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("read source file {}", path.display()))?;
            let expanded = expand_env(&content);
            let value: serde_yaml::Value = serde_yaml::from_str(&expanded)
                .with_context(|| format!("parse source file {}", path.display()))?;
            let items = match value {
                serde_yaml::Value::Sequence(seq) => seq,
                serde_yaml::Value::Mapping(mut map) => map
                    .remove(serde_yaml::Value::String("sources".into()))
                    .and_then(|v| v.as_sequence().cloned())
                    .unwrap_or_default(),
                _ => Vec::new(),
            };
            for item in items {
                let spec: SourceSpec = serde_yaml::from_value(item)
                    .with_context(|| format!("parse source entry in {}", path.display()))?;
                specs.push(spec);
            }
        }
        specs.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(specs)
    }

    pub fn llm_available(&self) -> bool {
        !self.ai.base_url.is_empty()
            && !self.ai.base_url.contains("${")
            && !self.ai.scoring_model.is_empty()
            && !self.ai.scoring_model.contains("${")
            && std::env::var(&self.ai.api_key_env)
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false)
    }

    pub fn browser_provider(&self) -> &str {
        let p = self.browser.provider.to_ascii_lowercase();
        if p == "auto" {
            if std::env::var("EGO_BROWSER_ENABLED").ok().as_deref() == Some("1")
                && command_exists("ego-browser")
            {
                "ego"
            } else {
                "playwright"
            }
        } else if p == "ego" {
            "ego"
        } else {
            "playwright"
        }
    }
}

pub fn command_exists(name: &str) -> bool {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {}", shell_escape(name)))
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub fn expand_env(input: &str) -> String {
    let re = regex::Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}").expect("valid env regex");
    re.replace_all(input, |caps: &regex::Captures| {
        std::env::var(&caps[1]).unwrap_or_else(|_| caps[0].to_string())
    })
    .to_string()
}

fn expand_string(value: &str) -> String {
    expand_env(value)
}

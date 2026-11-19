use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::config::AppConfig;
use crate::models::{RawItem, SourceKind, SourceSpec};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XPost {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub likes: Option<u64>,
    #[serde(default)]
    pub retweets: Option<u64>,
    #[serde(default)]
    pub replies: Option<u64>,
    #[serde(default)]
    pub views: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct WorkerEnvelope {
    #[serde(default)]
    ok: bool,
    #[serde(default)]
    items: Vec<XPost>,
    #[serde(default)]
    error: Option<String>,
}

pub async fn fetch_x(cfg: &AppConfig, spec: &SourceSpec) -> Result<Vec<RawItem>> {
    let list_url = spec
        .list_url
        .as_deref()
        .or(spec.url.as_deref())
        .context("X source needs list_url or url")?;
    let provider = cfg.browser_provider();
    let input = json!({
        "list_url": list_url,
        "headless": cfg.browser.headless,
        "profile_dir": expand_home(&cfg.browser.profile_dir),
        "storage_state_b64_env": cfg.browser.storage_state_env,
        "max_items": spec.max_items,
        "max_scrolls": cfg.browser.max_scrolls,
        "scroll_delay_ms": cfg.browser.scroll_delay_ms,
        "timeout_ms": cfg.browser.timeout_ms,
        "proxy": proxy_from_env(&cfg.browser.proxy_env),
        "user_agent": cfg.browser.user_agent,
    });
    let worker_dir = Path::new(&cfg.browser.worker_dir);
    let (cmd, args, stdin_payload) = if provider == "ego" {
        (
            "ego-browser".to_string(),
            vec!["nodejs".to_string()],
            ego_script(&input)?.into_bytes(),
        )
    } else {
        (
            "node".to_string(),
            vec![worker_dir.join("playwright-x.mjs").display().to_string()],
            serde_json::to_vec(&input)?,
        )
    };
    let extra_timeout = if provider == "ego" { 90_000 } else { 30_000 };
    let worker_timeout = cfg.browser.timeout_ms
        + (cfg.browser.max_scrolls as u64 * cfg.browser.scroll_delay_ms)
        + extra_timeout;
    let output = run_browser_worker(&cmd, &args, &stdin_payload, worker_timeout).await?;
    let envelope: WorkerEnvelope = serde_json::from_slice(&output).with_context(|| {
        format!(
            "parse X browser worker output: {}",
            String::from_utf8_lossy(&output)
        )
    })?;
    if !envelope.ok {
        anyhow::bail!(
            "{}",
            envelope
                .error
                .unwrap_or_else(|| "X browser worker failed without error".into())
        );
    }
    let now = Utc::now();
    let mut out = Vec::new();
    for post in envelope.items.into_iter().take(spec.max_items) {
        if post.text.trim().is_empty() || post.url.trim().is_empty() {
            continue;
        }
        let published_at = post
            .published_at
            .as_deref()
            .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
            .map(|v| v.with_timezone(&Utc))
            .unwrap_or(now);
        let mut item = RawItem {
            id: format!("{}:x:{}", spec.id, stable_post_id(&post.url)),
            source_id: spec.id.clone(),
            kind: SourceKind::X,
            title: post
                .text
                .lines()
                .next()
                .unwrap_or("X post")
                .chars()
                .take(120)
                .collect(),
            url: post.url.clone(),
            canonical_url: crate::dedup::normalize_url(&post.url),
            author: post.author.clone(),
            published_at,
            fetched_at: now,
            content: post.text,
            excerpt: String::new(),
            lang: spec.lang.clone(),
            title_zh: None,
            title_en: None,
            summary_zh: None,
            summary_en: None,
            score: None,
            simhash: 0,
            metadata: json!({
                "likes": post.likes,
                "retweets": post.retweets,
                "replies": post.replies,
                "views": post.views,
                "category": "social",
                "provider": provider,
            }),
        };
        item.ensure_derived();
        out.push(item);
    }
    Ok(out)
}

async fn run_browser_worker(
    cmd: &str,
    args: &[String],
    stdin_payload: &[u8],
    timeout_ms: u64,
) -> Result<Vec<u8>> {
    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("spawn browser worker {cmd}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(stdin_payload).await?;
    }
    let output = tokio::time::timeout(
        Duration::from_millis(timeout_ms.max(10_000)),
        child.wait_with_output(),
    )
    .await
    .context("browser worker timed out")??;
    if !output.status.success() {
        anyhow::bail!(
            "browser worker exited {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let last_json = stdout
        .lines()
        .rev()
        .find(|line| line.trim_start().starts_with('{'))
        .unwrap_or(stdout.trim());
    Ok(last_json.as_bytes().to_vec())
}

fn ego_script(input: &Value) -> Result<String> {
    let cfg = serde_json::to_string(input)?;
    Ok(format!(
        r#"
const cfg = {cfg};
const task = await taskSpace("ainews X list collection");
const page = task.page("p1");
try {{
  await page.goto(cfg.list_url, {{ timeout: cfg.timeout_ms }});
  await page.waitForTimeout(5000);
  const logged = await page.evaluate(() => !!document.querySelector("[data-testid='SideNav_AccountSwitcher_Button']"));
  if (!logged) {{
    console.log(JSON.stringify({{ ok: false, error: "X browser session is not logged in", items: [] }}));
    await task.finish({{ keep: [] }});
    process.exit(0);
  }}
  for (let i = 0; i < cfg.max_scrolls; i++) {{
    await page.mouse.wheel(0, 2600);
    await page.waitForTimeout(cfg.scroll_delay_ms);
  }}
  const items = await page.evaluate((maxItems) => {{
    const parseCount = (node) => {{
      if (!node) return null;
      const raw = (node.getAttribute("aria-label") || node.innerText || "").replace(/,/g, "");
      const m = raw.match(/([\d.]+)\s*(万|w|k|m|b|亿)?/i);
      if (!m) return null;
      let n = parseFloat(m[1]);
      const u = (m[2] || "").toLowerCase();
      if (u === "万" || u === "w") n *= 10000;
      else if (u === "亿") n *= 100000000;
      else if (u === "k") n *= 1000;
      else if (u === "m") n *= 1000000;
      else if (u === "b") n *= 1000000000;
      return Math.round(n);
    }};
    const articles = [...document.querySelectorAll('article[data-testid="tweet"]')];
    return articles.map((a) => {{
      const textEl = a.querySelector('[data-testid="tweetText"]');
      const timeEl = a.querySelector("time");
      const linkEl = [...a.querySelectorAll('a[href*="/status/"]')].find((x) => x.href);
      const authorEl = a.querySelector('a[href^="/"] [dir="ltr"]');
      return {{
        url: linkEl ? linkEl.href : "",
        text: textEl ? textEl.innerText : "",
        author: authorEl ? authorEl.innerText : "",
        published_at: timeEl ? timeEl.getAttribute("datetime") : null,
        likes: parseCount(a.querySelector('[data-testid="like"]')),
        retweets: parseCount(a.querySelector('[data-testid="retweet"]')),
        replies: parseCount(a.querySelector('[data-testid="reply"]')),
        views: parseCount(a.querySelector('[aria-label*="查看"], [aria-label*="View"]'))
      }};
    }}).filter((x) => x.url && x.text).slice(0, maxItems);
  }}, cfg.max_items);
  console.log(JSON.stringify({{ ok: true, items }}));
  await task.finish({{ keep: [] }});
}} catch (e) {{
  console.log(JSON.stringify({{ ok: false, error: String(e), items: [] }}));
  try {{ await task.finish({{ keep: [] }}); }} catch (_) {{}}
  process.exit(1);
}}
"#
    ))
}

fn stable_post_id(url: &str) -> String {
    url.rsplit('/').next().unwrap_or("post").to_string()
}

fn proxy_from_env(primary: &str) -> Option<String> {
    [
        primary,
        "EASEL_PROXY",
        "TWITTER_X_PROXY",
        "TWITTER_PROXY",
        "X_PROXY",
        "https_proxy",
        "HTTPS_PROXY",
        "all_proxy",
        "ALL_PROXY",
    ]
    .iter()
    .find_map(|key| std::env::var(key).ok().filter(|v| !v.trim().is_empty()))
}

fn expand_home(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{home}/{rest}");
        }
    }
    path.to_string()
}

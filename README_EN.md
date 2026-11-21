# Po Radar · AI News RSS

**English** | [中文](README.md)

[![CI](https://github.com/anthonyhann/ai-news-rss/actions/workflows/ci.yml/badge.svg)](https://github.com/anthonyhann/ai-news-rss/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.98.1-orange.svg)](rust-toolchain.toml)
[![React](https://img.shields.io/badge/React-19-149eca.svg)](web/package.json)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)
[![Live](https://img.shields.io/badge/Live-Dashboard-d8ff65)](https://anthonyhann.github.io/ai-news-rss/)

Po Radar is a self-hosted intelligence desk for the AI ecosystem, hosted by
the original companion character **Po**. A Rust core collects and deduplicates
sources, scores them with an editor model, writes a bilingual brief, generates
a daily action plan, and delivers high-signal alerts. A React dashboard
publishes the same data as a static site or an axum-backed server.

The goal is not more information. The goal is fewer bad judgments and a
traceable path back to the original source.

> First-time setup starts with the [Configuration SOP](#configuration-sop),
> which covers secrets, sources, verification, and troubleshooting in order.

![Po Radar dashboard](docs/assets/dashboard.png)

## Meet Po

![Po, the companion editor](docs/assets/po.png)

Po uses the IP artwork supplied for this project: a tangerine plush body,
cream face and belly, coral inner ears and feet, and a small leaf on top. The
round silhouette and minimal dot eyes carry Nailong's Q-version softness, while
the companion-like warmth echoes the spirit of Teletubbies Po. Face crops are
used in the status pill and story rows; the full-body illustration appears on
the dashboard and in briefs, animated with breathing, floating, and soft
shadows.

### Soft Plush Citrus

- **Oatmilk `#F7F1E8`**: canvas and breathing room.
- **Tangerine `#F19A4B`**: Po, primary emphasis, and key CTAs.
- **Cream `#F8E8CA`**: face, belly, cards, and reading surfaces.
- **Coral `#D96849`**: feet, links, alerts, and scores.
- **Leaf `#8A9A4B`**: source health, community signal, and secondary states.

Motion is limited to four patterns: a 6-second character breath, a 6.5-second
float, a 210ms status reveal, and a 320ms card rise. No blue-purple gradients,
heavy glows, or large saturated fields. The interface stays tool-like while
keeping the warmth of a plush collectible.

## Why This Exists

AI news is fragmented across lab blogs, X, YouTube, Reddit, Hacker News,
GitHub, Telegram, WeChat RSS bridges, and newsletters. Most aggregators stop at
“here are more links.” Po Radar adds an editorial layer:

- **Deterministic first:** URL normalization, content hashes, and 64-bit
  SimHash remove exact and near-duplicate stories.
- **Model-assisted second:** a model classifies candidates as
  `same_event`, `follow_up`, or `different_event` before scoring.
- **Editorial scoring:** relevance, impact, novelty, credibility, community
  signal, and actionability produce a 0–100 score.
- **Bilingual output:** titles, summaries, and briefs are produced in Chinese
  and English.
- **Action, not just reading:** a Tree-of-Thoughts planner proposes one top
  action, two experiments, and one watchlist item.
- **Portable state:** SQLite works locally, in Docker, and in a single-commit
  GitHub Actions `state` branch.

## Features

| Area | What is included |
|---|---|
| Sources | RSS/Atom, Hacker News, Reddit, GitHub Releases, OSS Insight, Telegram, YouTube, arXiv, WeChat RSS bridges, X Lists |
| Deduplication | canonical URLs, SHA-256 content hashes, SimHash buckets, title Jaccard, model event merge |
| Scoring | 0–100 editorial rubric with hard caps for non-AI, rumors, missing links, and stale content |
| Briefs | Chinese and English editions, executive summary, top signals, background, community discussion, citations |
| Action plan | Tree-of-Thoughts generates 3–5 candidates ranked by impact, feasibility, speed and confidence; score summaries are visible in the brief |
| Alerts | score ≥ 90, confidence ≥ 0.80, 24-hour story deduplication, optional material follow-ups |
| Delivery | WeCom, Feishu, DingTalk, Telegram, email, web publication |
| Web | Today, Briefs, Sources, Alerts, Archive, About; language-aware brief/archive lists, search, filters, bookmarks, dark/light mode |
| Deployment | GitHub Actions + Pages, local CLI, Docker/Compose, axum API |
| Browser | Playwright worker for Actions/servers; ego-lite for a local logged-in browser |

### Tree-of-Thoughts Action Plan

Each daily brief keeps the candidate actions and their four-dimensional score
summary. The editor selects one primary action, two reversible experiments,
and one watch item; every recommendation links back to evidence. The UI exposes
the auditable scoring summary without revealing hidden reasoning.

![Po Radar action plan](docs/assets/action-plan.png)

## Source Coverage

The seed catalog contains **117 logical sources**:

- 64 RSS/Atom feeds: AI labs, companies, researchers, developer tools, and media.
- 20 WeChat RSS-bridge feeds.
- 13 YouTube channels.
- 11 Reddit subreddits.
- 2 public Telegram channels.
- 3 GitHub release feeds.
- Hacker News, OSS Insight, arXiv, and one X List connector.

The X connector is one List, not one request per account. Put the builder
handles in `config/sources/x.yaml` and set `X_LIST_URL` to the List you create.
A List can contain 300+ accounts while still being collected as one browser
run.

The original product target was **370+ sources**. That is a catalog target:
add more official feeds, WeChat bridges, and X handles as your needs grow. It
does not mean firing 370 independent network requests every two hours.

## Architecture

```text
Sources
  RSS / HN / Reddit / GitHub / OSS Insight / Telegram / YouTube / WeChat / X
        │
        ▼
Stage 1: URL + content hash + SimHash + title Jaccard
        │
        ▼
Stage 2: model event merge (same_event / follow_up / different_event)
        │
        ▼
Score: 0–100 editorial rubric + deterministic hard caps
        │
        ├── score ≥ 60 → bilingual brief candidate
        └── score ≥ 90 + confidence ≥ 0.80 → alert
        │
        ▼
Editor model → Chinese/English brief + Tree-of-Thoughts action plan
        │
        ▼
SQLite → static JSON / Pages / axum API / WeCom / Feishu / DingTalk / Telegram / email
```

Key components:

- `src/sources/` connects and normalizes each source type.
- `src/dedup.rs` owns URL normalization, tokenization, SimHash, and Jaccard.
- `src/browser.rs` runs the Playwright or ego-lite X provider.
- `src/llm/` implements the OpenAI-compatible provider and mock provider.
- `src/pipeline/` coordinates collection, scoring, briefs, alerts, and export.
- `src/store/` owns SQLite schema, queries, source health, and merge state.
- `web/` is the React/Vite dashboard.
- `browser-worker/playwright-x.mjs` is the headless X List worker.

## Quick Start

Requirements: Rust 1.98.1+, Node.js 24+, pnpm 11+, and optionally Docker.

```bash
cp .env.example .env

# Run the complete pipeline with fixture data (no network or API key required)
cargo run -- run --fixtures --slot morning

# Build and preview the dashboard
pnpm --dir web install
pnpm --dir web build
pnpm --dir web preview

# Local API + dashboard
cargo run -- serve
```

Open:

- `http://127.0.0.1:4173/` for Vite preview.
- `http://127.0.0.1:8080/` for the axum server.

Useful commands:

```bash
cargo run -- doctor
cargo run -- sources count
cargo run -- sources check
cargo run -- sources reset x-ai-builders
cargo run -- collect --only openai-blog
cargo run -- brief --slot evening
cargo run -- export
```

## Configuration SOP

Goal: go from a fresh checkout to a working local pipeline with a real model,
X collection, Feishu delivery, and automated GitHub Actions deployment. Follow
the steps in order.

### 0. Configuration file map

| File or directory | Purpose | Commit? |
|---|---|---|
| `.env.example` | Names-only template for all secrets | Yes; never add real values |
| `.env` | Local secret values; loaded automatically by the CLI | No; ignored by Git |
| `config/app.toml` | Timezone, model budget, scoring thresholds, delivery switches, browser settings | Yes |
| `config/sources/*.yaml` | Source catalog grouped by RSS, community, YouTube, WeChat and X | Yes |
| `config/prompts/*.md` | Scoring, dedup, editor, bilingual and action prompts | Yes |
| `.github/workflows/*.yml` | Collection, digest, Pages and CI schedules | Yes |

`.env`, `.env.*`, `cookies.json`, `storage-state*.json`, `*.pem`, and `*.key`
are ignored. GitHub Secrets remain encrypted in GitHub and never enter the
repository or Pages artifacts.

### 1. First-time setup

```bash
cp .env.example .env
chmod 600 .env
pnpm --dir web install
pnpm --dir browser-worker install
cargo run -- doctor
```

`doctor` should report Node, the Playwright worker, and `yt-dlp` as available.
`llm.configured=false` is expected until `LLM_API_KEY` is set.

### 2. LLM configuration

Edit `.env`:

```dotenv
LLM_BASE_URL=https://api.deepseek.com/v1
LLM_API_KEY=sk-YOUR_KEY
LLM_SCORING_MODEL=deepseek-chat
LLM_EDITOR_MODEL=deepseek-chat
```

Get `LLM_API_KEY` from the [DeepSeek console](https://platform.deepseek.com/api_keys):
create an API key and copy the `sk-...` value. The project is not tied to a
vendor; for another OpenAI-compatible endpoint, change the base URL and both
model names together.

The ARK Agent Plan OpenAI-compatible data plane is
`https://ark.cn-beijing.volces.com/api/plan/v3`. Using `/api/plan` directly
returns 404, while the ordinary `/api/v3` path returns 401 for a plan key.

GitHub-hosted runners must be able to reach the ARK China endpoint. When they
cannot, the run falls back to the mock scorer. Use a self-hosted runner or a
reachable OpenAI-compatible gateway for stable real-model runs.

```bash
cargo run -- doctor | jq '.llm'
```

`"configured": true` means the real model path is active. Without an API key,
collection still runs with `mock-editor`; do not treat those briefs as
production output.

Tune `max_items_per_run`, `max_editor_items`, `max_tokens_per_run`, and
`temperature` in `[ai]` after the first successful run.

### 3. X collection

1. Create or open the target AI-builder List in X **Lists** and add accounts.
2. Copy the list URL, for example `https://x.com/i/lists/1234567890123456789`.
3. Log in to `x.com` with an account that can access the List.
4. Open DevTools → Application → Storage → Cookies → `https://x.com`, then
   copy `auth_token` and `ct0`.
5. Add the values to `.env`:

```dotenv
X_LIST_URL=https://x.com/i/lists/1234567890123456789
TWITTER_AUTH_TOKEN=YOUR_AUTH_TOKEN
TWITTER_CT0=YOUR_CT0
TWITTER_PROXY=
```

Private Lists require cookies from an account with access. Expired cookies
produce `X browser session is not logged in`. Set `TWITTER_PROXY` only when
needed. Never commit or share the cookies.

```bash
cargo run -- collect --only x-ai-builders
cargo run -- sources check
```

X failures are isolated to the source and do not block RSS/HN/Reddit/GitHub.
`[browser]` in `config/app.toml` controls `max_scrolls`, `scroll_delay_ms`, and
`timeout_ms`; avoid overly aggressive values.

If the account has no X List, `X_LIST_URL` may be set to `https://x.com/home` to
collect the authenticated home timeline; the scoring layer filters non-AI
content afterwards.

Use `TWITTER_PROXY` only for local networks that need a proxy. Never store a
`127.0.0.1:7890`-style local address as a GitHub Actions Secret; the hosted
runner cannot reach it.

GitHub-hosted runners use a different egress IP from Easel. X can reject that
session; `X browser session is not logged in` usually means the runner/VPN
path is not accepted. Use a self-hosted runner, a network-reachable proxy, or
the official X API. The Easel login remains usable for local runs.

### 4. Feishu delivery

1. In the target Feishu group, open Settings → Group Bots → Add custom bot.
2. Copy the Webhook, for example
   `https://open.feishu.cn/open-apis/bot/v2/hook/YOUR_HOOK`.
3. The current adapter does not sign requests. Use a custom keyword or IP
   allowlist; `AI` is a practical keyword because alerts and briefs include it.
4. Add it to `.env`:

```dotenv
FEISHU_WEBHOOK_URL=https://open.feishu.cn/open-apis/bot/v2/hook/YOUR_HOOK
PUBLIC_SITE_URL=https://anthonyhann.github.io/ai-news-rss
```

`[delivery.feishu] enabled = true` is already set. For other channels, enable
their sections and provide the matching URL, token, or SMTP values.

Daily pushes use **A · Po Morning Letter**: a 60-second brief, today's lead,
two more signals, the ToT action, and a link to the full brief. Feishu receives
an interactive card; WeCom and DingTalk receive Markdown; Telegram receives
HTML; email receives one merged bilingual 600px HTML edition. `PUBLIC_SITE_URL`
builds the “open full brief” links, can be overridden by a repository variable
in Actions, and falls back to this project's Pages URL.

Optional connectivity test:

```bash
set -a; . ./.env; set +a
curl -fsS -X POST "$FEISHU_WEBHOOK_URL" \
  -H 'Content-Type: application/json' \
  -d '{"msg_type":"text","content":{"text":"AI Po Radar configuration test"}}'
```

### 5. GitHub Actions Secrets

Never put real values in workflow YAML. Use the GitHub CLI; it prompts for each
value interactively:

```bash
gh secret set LLM_BASE_URL --repo anthonyhann/ai-news-rss
gh secret set LLM_API_KEY --repo anthonyhann/ai-news-rss
gh secret set LLM_SCORING_MODEL --repo anthonyhann/ai-news-rss
gh secret set LLM_EDITOR_MODEL --repo anthonyhann/ai-news-rss
gh secret set X_LIST_URL --repo anthonyhann/ai-news-rss
gh secret set TWITTER_AUTH_TOKEN --repo anthonyhann/ai-news-rss
gh secret set TWITTER_CT0 --repo anthonyhann/ai-news-rss
gh secret set FEISHU_WEBHOOK_URL --repo anthonyhann/ai-news-rss
```

Verify names only:

```bash
gh secret list --repo anthonyhann/ai-news-rss
```

Keep `TWITTER_PROXY` in the local `.env` only. GitHub-hosted runners cannot
reach a `127.0.0.1` proxy; use a self-hosted runner or a proxy address that the
runner can reach.

### 6. Sources, thresholds and prompts

Add a unique source ID in the relevant YAML file:

```yaml
sources:
  - id: example-ai-blog
    kind: rss
    name: Example AI Blog
    url: https://example.com/feed.xml
    tier: 2
    lang: en
    category: model
    max_items: 20
    enabled: true
```

Use `enabled: false` to keep a source in the catalog without collecting it.
After five consecutive failures a source is skipped; clear the counter after
fixing it:

```bash
cargo run -- sources reset example-ai-blog
```

Important `[pipeline]` settings:

- `digest_score`: minimum score for a brief, default 60.
- `alert_score` / `alert_confidence`: instant alert threshold, default 90 / 0.80.
- `max_alerts_per_run`: default 3.
- `dedup_days`, `simhash_hamming`, `title_jaccard`: cross-day dedup strictness.
- `languages`: brief languages.

Prompts live in `config/prompts/`. Changes to `score.md`, `dedup.md`,
`brief.md`, or `action.md` affect the next model call; test with fixtures or a
single source before a full run.

### 7. Verification order

```bash
# 1. Dependencies and configuration
cargo run -- doctor
cargo run -- sources count
cargo run -- sources check

# 2. One network source
cargo run -- collect --only openai-blog

# 3. Full collection and scoring
cargo run -- run --no-brief

# 4. Brief and action plan
cargo run -- brief --slot morning

# 5. Export and preview
cargo run -- export
pnpm --dir web build
pnpm --dir web preview
```

Inspect `web/public/data/latest.json` and `actions.json` for `action_plan`,
briefs, and source links. For Actions, use `workflow_dispatch`: check Collect,
then Digest, then the Pages deployment.

### 8. Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `configured=false` or `mock-editor` | Missing `LLM_API_KEY` or endpoint/model | Fill `.env`, run `doctor` |
| `X browser session is not logged in` | Expired cookies or no access to a private List | Refresh `auth_token` / `ct0` |
| X health failure while other sources work | X source isolated | Check List URL, cookies and proxy |
| No Feishu message | Missing Webhook, keyword/signature mismatch | Check `FEISHU_WEBHOOK_URL` and bot security |
| No brief | Nothing reached `digest_score`, or no model | Inspect run statistics and score logs |
| Source auto-disabled | Five consecutive failures | Fix source, run `sources reset` |
| Actions cannot write `state` | Missing `contents: write` or branch protection | Check workflow permissions |
| Local `.env` changes ignored | Not started from repo root or empty variables | Restart the command from root |

### 9. Security rules

- Put real values only in `.env` or GitHub Secrets.
- Check `git status` and `git diff` before committing; no `.env`, cookies,
  Webhook URLs, or API keys may appear.
- Prefer a dedicated X account or official API over personal-account cookies.
- Rotate `LLM_API_KEY`, X cookies, Feishu Webhook, and SMTP passwords regularly.
- Revoke a leaked key first, then update the corresponding Secret.

## Sources

Source files live in `config/sources/`:

- `official.yaml`: labs, AI companies, research groups, developer tools, media.
- `community.yaml`: Hacker News, Reddit, GitHub Releases, OSS Insight, Telegram.
- `youtube.yaml`: AI channels and optional transcript collection.
- `wechat.yaml`: WeChat RSS-bridge feeds.
- `x.yaml`: X List URL and seed handles.

Add a unique `id` for each source. Use `${ENV_VAR}` for private URLs. Do not
commit secrets.

WeChat Official Accounts do not expose a stable public scraping API. This
project consumes a self-hosted RSS bridge such as WeRSS or wechat2rss. If
`WECHAT_RSS_BASE` is missing, those sources fail health checks without
blocking other sources.

## X: Playwright and ego-lite

### Playwright

```bash
pnpm --dir browser-worker install
pnpm --dir browser-worker exec playwright install chromium
```

Set `X_LIST_URL`, `TWITTER_AUTH_TOKEN`, `TWITTER_CT0`, and optionally
`TWITTER_PROXY`. The worker injects the X cookies when available, or uses a
Playwright `storageState` / persistent profile. This is the path used by
GitHub Actions and servers.

### ego-lite

On a machine with a logged-in ego-lite browser:

```bash
ego-browser skill
EGO_BROWSER_ENABLED=1 cargo run -- collect
```

`provider = "auto"` selects ego-lite when `ego-browser` is installed and
`EGO_BROWSER_ENABLED=1`; otherwise it uses Playwright. ego-lite does not run in
GitHub Actions.

### Important

X automation rules and anti-bot systems change. Use a dedicated account or a
paid API where possible. A failed X collection is isolated as a source-health
failure; it does not block RSS/HN/Reddit/GitHub collection.

## GitHub Actions

- `.github/workflows/ci.yml` runs fmt, clippy, tests, web build, and worker
  syntax checks.
- `.github/workflows/deploy.yml` restores SQLite from the `state` branch,
  materializes every brief JSON file, and deploys Pages on pushes to `main`.
- `.github/workflows/collect.yml` collects and scores every two hours, then
  builds and deploys the static dashboard.
- `.github/workflows/digest.yml` generates Chinese/English briefs at 08:20 and
  20:20 Asia/Shanghai.

Run a single-source smoke test, for example X only:

```bash
gh workflow run collect.yml --repo anthonyhann/ai-news-rss -f only=x-ai-builders
```

The workflows use a **single-commit `state` branch** for SQLite. This keeps
the main branch and Git history small. The public repository gets standard
GitHub-hosted runner minutes for free, but LLM calls, X/Playwright proxies, and
YouTube network access may still cost money.

Required repository secrets are documented in `.env.example` and the workflow
files. Never commit `.env`, `cookies.json`, `data/`, or `web/public/data/`.

Secret acquisition, GitHub configuration, and verification are consolidated in
the **Configuration SOP** above. Real values belong only in `.env` or GitHub
Secrets; the repository tracks only the key-free `.env.example`.

## Docker

```bash
docker compose up -d --build
```

The runtime image is based on the official Playwright image so X automation and
`yt-dlp` are available. SQLite is stored in the `data/` volume. The dashboard
is served by `ainews serve`; the API is read-only except for the optional
admin trigger.

## Scoring and Editorial Rules

The 100-point rubric:

- Relevance to AI: 30
- Impact: 25
- Novelty: 15
- Source credibility: 15
- Community signal: 10
- Actionability: 5

Hard caps:

- Non-AI topic: 59
- Unverified rumor: 69
- No original link: 50
- Duplicate or older than seven days: 30
- KOL/media repost of a first-party release: 89

Alerts require score ≥ 90 and confidence ≥ 0.80. A story is not alerted again
within 24 hours unless the score materially changes. The brief keeps the
highest-signal stories and cites original URLs. The site does not republish
full articles or complete transcripts.

## Project Layout

```text
src/                 Rust pipeline, sources, LLM, store, server
web/                 React/Vite dashboard
browser-worker/      Playwright X List worker
config/sources/      YAML source catalog
config/profiles/     editorial profiles and weights
config/prompts/      scoring, brief, dedup, and action prompts
migrations/          SQLite schema
tests/fixtures/      deterministic fixture data
scripts/             Actions state restore/save
.github/workflows/   CI, collection, digest
Dockerfile           Playwright-based server image
```

## Tests

```bash
cargo fmt --all -- --check
cargo test
cargo clippy --all-targets -- -D warnings
pnpm --dir web build
pnpm --dir browser-worker exec node --check playwright-x.mjs
docker build --check .
```

The fixture pipeline is deterministic: eight raw items produce seven stories
after deduplication, a 93-point top signal, and four bilingual briefs.

## Contributing

Please read [CONTRIBUTING.md](CONTRIBUTING.md) and
[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). Add sources through YAML, keep
secrets out of the repository, and include parser fixtures when changing a
connector.

## Security

Please do not open a public issue for secrets or exploitable vulnerabilities.
See [SECURITY.md](SECURITY.md).

## License

MIT. See [LICENSE](LICENSE).

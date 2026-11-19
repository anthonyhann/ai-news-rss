CREATE TABLE IF NOT EXISTS sources (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    tier INTEGER NOT NULL DEFAULT 3,
    enabled INTEGER NOT NULL DEFAULT 1,
    last_ok_at TEXT,
    last_error TEXT,
    failure_count INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS items (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    title TEXT NOT NULL,
    url TEXT NOT NULL,
    canonical_url TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    simhash INTEGER NOT NULL,
    published_at TEXT NOT NULL,
    fetched_at TEXT NOT NULL,
    payload_json TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_items_canonical_url ON items(canonical_url);
CREATE INDEX IF NOT EXISTS idx_items_simhash ON items(simhash);
CREATE INDEX IF NOT EXISTS idx_items_published_at ON items(published_at);

CREATE TABLE IF NOT EXISTS stories (
    id TEXT PRIMARY KEY,
    primary_item_id TEXT NOT NULL,
    first_seen_at TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    score REAL NOT NULL DEFAULT 0,
    version INTEGER NOT NULL DEFAULT 1,
    payload_json TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_stories_last_seen ON stories(last_seen_at);
CREATE INDEX IF NOT EXISTS idx_stories_score ON stories(score);

CREATE TABLE IF NOT EXISTS story_items (
    story_id TEXT NOT NULL,
    item_id TEXT NOT NULL,
    similarity REAL NOT NULL DEFAULT 1,
    is_primary INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (story_id, item_id)
);

CREATE TABLE IF NOT EXISTS scores (
    story_id TEXT PRIMARY KEY,
    score_json TEXT NOT NULL,
    model TEXT NOT NULL,
    prompt_version TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS briefs (
    id TEXT PRIMARY KEY,
    date TEXT NOT NULL,
    slot TEXT NOT NULL,
    lang TEXT NOT NULL,
    markdown TEXT NOT NULL,
    json TEXT NOT NULL,
    model TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(date, slot, lang)
);

CREATE INDEX IF NOT EXISTS idx_briefs_date ON briefs(date);

CREATE TABLE IF NOT EXISTS action_plans (
    date TEXT PRIMARY KEY,
    json TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS deliveries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    story_id TEXT,
    brief_id TEXT,
    channel TEXT NOT NULL,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    dedup_key TEXT NOT NULL,
    sent_at TEXT NOT NULL,
    error TEXT
);

CREATE INDEX IF NOT EXISTS idx_deliveries_dedup ON deliveries(dedup_key, sent_at);

CREATE TABLE IF NOT EXISTS runs (
    id TEXT PRIMARY KEY,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    status TEXT NOT NULL,
    stats_json TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_runs_started ON runs(started_at);

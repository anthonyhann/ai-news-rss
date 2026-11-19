# Source Catalog

The seed catalog currently contains **117 logical sources**:

- 64 RSS/Atom sources (labs, companies, researchers, developer tools, media)
- 20 WeChat RSS-bridge feeds
- 13 YouTube channels
- 11 Reddit subreddits
- 2 Telegram public channels
- 3 GitHub release feeds
- Hacker News, OSS Insight, arXiv, and one X List source

The X List source is one connector, not one request per account. Put the
builder handles in `x.yaml` under `x_handles` and point `X_LIST_URL` at the
List you create. The connected List may contain 310+ accounts while still
costing one browser run.

The `370+` target in the original product brief is a catalog target. It is
reached by adding more official feeds, WeChat bridge feeds, and X handles; it
does not mean firing 370 independent network requests every two hours.

Source health is stored in SQLite. A source is automatically skipped after
five consecutive failures. Fix the source or clear its failure count before
re-enabling it.

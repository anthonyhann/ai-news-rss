# Contributing

Thanks for helping improve AI Radar. This project prefers small, verifiable
changes over broad rewrites.

## Development Setup

```bash
cp .env.example .env
cargo run -- run --fixtures --slot morning
pnpm --dir web install
pnpm --dir web build
```

The fixture path must keep working without network access or API keys.

## Checks

Run these before opening a pull request:

```bash
cargo fmt --all -- --check
cargo test
cargo clippy --all-targets -- -D warnings
pnpm --dir web build
pnpm --dir browser-worker exec node --check playwright-x.mjs
```

## Source Changes

- Add or edit sources in `config/sources/*.yaml`.
- Keep every `id` unique and stable.
- Use `${ENV_VAR}` for private URLs and tokens.
- Add a parser fixture under `tests/fixtures/` when changing a connector.
- Mark sources disabled when a feed is known to be permanently broken.
- Run `cargo run -- sources check` and `cargo run -- sources reset <id>` when
  testing source health.

## Security

- Never commit `.env`, cookies, browser profiles, SQLite state, or generated
  public data.
- Do not put credentials in source YAML, prompts, logs, screenshots, or issue
  bodies.
- Report vulnerabilities using the process in [SECURITY.md](SECURITY.md), not a
  public issue.

## Pull Requests

Describe the user-visible change, the data contract change, and the exact
validation commands you ran. Keep unrelated refactors out of the same PR.

By contributing, you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

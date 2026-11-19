# Security Policy

## Supported Versions

Security fixes are applied to the current `main` branch. There is no long-term
support guarantee for older snapshots.

## Reporting a Vulnerability

Please do not disclose security-sensitive issues in a public issue.

Use GitHub's private vulnerability reporting feature if available, or contact
the repository maintainer through GitHub with:

- A short description of the issue.
- Reproduction steps or a minimal proof of concept.
- The affected version, commit, or workflow.
- Any suggested mitigation.

Please allow reasonable time for triage and a coordinated fix before public
disclosure.

## Sensitive Data

The following must never be committed:

- `.env` files or API keys.
- X cookies, `storageState`, or browser profiles.
- WeChat bridge URLs that contain tokens.
- Webhook URLs, SMTP passwords, or Telegram tokens.
- Generated `data/` or `web/public/data/` files from a real deployment.

If a credential is exposed, revoke it immediately and rotate the corresponding
GitHub Actions secret.

## Scope

This project aggregates third-party content and may run browser automation.
Reports about third-party platform terms, source availability, or content
ownership should be handled with the source owner. Security reports should
focus on this repository's code, workflows, storage, and deployment.

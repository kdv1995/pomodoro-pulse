# Contributing

Thanks for helping improve Pomodoro Pulse.

## Ground Rules

- Be kind and constructive in issues/PRs.
- For larger changes, open an issue first so we can align on the approach.
- Please **do not** add "contributors" lists (names/handles) to `README.md`. Git history and GitHub already track credit.
- Never commit secrets (API keys, tokens, `.p12`, etc.).
- Low-effort spam or drive-by PRs without context may be closed.
- Maintainers may request changes, tests, or a narrower scope before merging.

## Development Setup

Requirements:

- Node.js + npm
- Rust toolchain
- Tauri prerequisites for macOS

Run locally:

```bash
npm install
npm run tauri dev
```

## Code Standards

- Keep changes focused (one PR = one logical change).
- Prefer TypeScript types over `any`.
- Avoid breaking existing UI flows unless the PR explains the change clearly.

## Before Opening A PR

```bash
npm run verify:twice
```

If you do not need a full macOS bundle build locally, run the minimum set:

```bash
npm run test:run
npm run build
npm run version:check
cd src-tauri && cargo fmt --all -- --check && cargo test
```

## Versioning / Releases

- We use semver (`MAJOR.MINOR.PATCH`).
- Release builds are produced from git tags like `v0.1.4` via GitHub Actions.
- If your PR changes app behavior, add a short note in the PR description describing the user-visible impact.

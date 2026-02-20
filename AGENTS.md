# AGENTS.md

## Project Snapshot
- Name: `pomodoro-pulse`
- App type: Tauri desktop app with a React frontend and Rust backend.
- Languages: TypeScript, Rust, CSS, HTML.
- Package managers/build tools: `npm` + Vite (frontend), Cargo (Rust), Tauri CLI (desktop bundling).

## Detected Frameworks and Runtime
- Frontend: React 19 + TypeScript + Vite 7.
- Backend/Desktop: Tauri 2 + Rust 2021 edition.
- UI/Data libs: Radix UI, TanStack Query, Recharts, Zustand.
- Persistence: SQLite via `rusqlite` (bundled SQLite).

## Repository Layout
- Frontend source: `src/`
- Frontend tests: `tests/`
- Rust/Tauri project: `src-tauri/`
- CI workflows: `.github/workflows/`
- Utility scripts: `scripts/`

## Canonical Commands

### Setup
- `npm install`

### Local development
- `npm run tauri dev`
- `npm run dev` (frontend-only Vite dev server)

### Tests
- Frontend: `npm run test:run` (Vitest, `happy-dom`)
- Rust: `npm run test:rust` (maps to `cargo test --manifest-path src-tauri/Cargo.toml`)

### Build
- Frontend build: `npm run build`
- Tauri bundle build: `npm run tauri build`
- macOS DMG bundle: `npm run tauri build -- --bundles dmg`

### Version/consistency checks
- `npm run version:check`
- Full verification pipeline: `npm run verify:full`

### Formatting/Linting
- Rust format check: `npm run fmt:rust:check`
- Rust format apply (manual): `cargo fmt --manifest-path src-tauri/Cargo.toml --all`
- TypeScript lint/format scripts are not defined in `package.json`.

## CI-Observed Conventions
- CI runs on Linux, Windows, and macOS.
- CI executes:
  1. `npm ci`
  2. `npm run test:run`
  3. `npm run build`
  4. `npm run version:check`
  5. `cargo fmt --all -- --check` (in `src-tauri`)
  6. platform-specific `npm run tauri build -- --bundles ...`
- Release workflow builds and publishes platform bundles from version tags (`v*.*.*`).

## Agent Working Rules For This Repo
- Prefer minimal, scoped changes in one logical unit.
- Do not edit generated or dependency directories (`dist/`, `node_modules/`) unless explicitly requested.
- Keep frontend imports compatible with alias `@/* -> ./src/*`.
- For Rust changes, run `npm run fmt:rust:check` and `npm run test:rust` when possible.
- For frontend changes, run `npm run test:run` and `npm run build` when possible.
- Before release-impacting changes, ensure `npm run version:check` still passes.

## Definition of Done For Agent Tasks
- Changes compile/build for affected surface area.
- Relevant test commands pass for changed code.
- No version mismatch introduced across:
  - `package.json`
  - `src-tauri/Cargo.toml`
  - `src-tauri/tauri.conf.json`
- Documentation/task artifacts in `agents/` are updated when workflows or conventions change.

## Pull Request Guidelines For Agents
- Keep PRs focused on a single logical change.
- Do not mix refactors with feature changes.
- Include a short summary explaining why the change is needed.
- If modifying cross-layer logic (TS <-> Rust), describe the integration boundary.
- Do not update lockfiles unless dependencies are intentionally changed.

## Agent Non-Goals
- Do not introduce new dependencies without justification.
- Do not redesign architecture unless explicitly requested.
- Do not refactor working code for style only.

## Self-Validation (Required Before Marking Any Task Done)

Before responding with “done”, “fixed”, or opening a PR, the agent MUST run and record a self-audit.

### A) Change Intent Check
- Restate the goal in 1 sentence.
- List exactly which user-facing behaviors changed (if any).
- Confirm no unrelated refactors/formatting were included.

### B) Diff Audit (Read Your Own Diff)
- Summarize all modified files and why each changed.
- Confirm changes are minimal and scoped.
- Identify any risky areas touched:
  - `src-tauri/tauri.conf.json`
  - `src-tauri/Cargo.toml`
  - SQLite schema/migrations
  - Rust <-> frontend IPC command boundary

### C) Repo Invariant Audit (Must Be True)
- No version mismatch introduced across:
  - `package.json`
  - `src-tauri/Cargo.toml`
  - `src-tauri/tauri.conf.json`
- Frontend import alias `@/* -> ./src/*` remains valid (no breaking path changes).
- No generated/dependency output committed (`dist/`, `node_modules/`, etc.).
- Lockfiles only changed if dependency changes were intentional (otherwise revert).

### D) Build/Test Verification (Run What Applies)
For every change, run the most relevant commands and report results:

Frontend changes (TypeScript/React):
- `npm run test:run`
- `npm run build`

Rust / Tauri changes:
- `npm run fmt:rust:check`
- `npm run test:rust`

Release-impacting / config changes:
- `npm run version:check`
- Prefer `npm run verify:full` when feasible.

If a command cannot be run (missing toolchain, OS constraints), explicitly say:
- what was not run,
- why,
- and what you did instead (e.g., static reasoning, limited build, targeted checks).

### E) CI Parity Check (Quick Sanity)
- Confirm your local checks align with CI sequence:
  - `npm ci` equivalents (locally: ensure clean install assumptions)
  - `npm run test:run`
  - `npm run build`
  - `npm run version:check`
  - Rust fmt check (`cargo fmt -- --check` behavior)
- If your change could break cross-platform builds (Linux/Windows/macOS), note likely risks.

### F) Failure Handling Rules
If any validation fails:
1. Do NOT finalize.
2. Diagnose the failure.
3. Apply the smallest fix.
4. Re-run only the relevant checks.
5. If still failing after 2 iterations, stop and ask for guidance with:
   - what failed,
   - what you tried,
   - and a proposed next step.

### G) Completion Report Format (Always Include)
At completion, output:

1) ✅ Summary (what changed, why)
2) 🧪 Commands run + results (copy/paste output summary)
3) 🔎 Risks / assumptions (especially for Tauri/SQLite/IPC)
4) 📌 Follow-ups (if any)

(If nothing was executed, explicitly state “No commands were run” and why.)

### H) Truthfulness Guarantee
- Never claim a command was executed unless it actually ran successfully.
- If uncertain, say so and propose the safest next validation step.

### I) No Silent Scope Expansion
- Do not expand the task scope beyond the original request.
- If additional improvements are identified, list them separately as suggestions, not part of the implemented change.
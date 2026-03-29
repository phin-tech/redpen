# Plan: Adopt Best Practices from atuinsh/desktop

## Repositories Compared

- Reference repo: [atuinsh/desktop](https://github.com/atuinsh/desktop)
- Current repo: [phin-tech/redpen](https://github.com/phin-tech/redpen)

## Goal

Adopt high-value engineering and coding practices from `atuinsh/desktop` that improve Red Pen reliability, type safety, and release quality without adding excessive process overhead.

## Current Gaps (with Code References)

1. CI is consolidated and misses some focused quality gates.
   - Red Pen current CI: `/.github/workflows/ci.yml` ([ref](https://github.com/phin-tech/redpen/blob/main/.github/workflows/ci.yml))
   - Atuin split gates: Rust checks/tests and TS checks/tests ([rust.yaml](https://github.com/atuinsh/desktop/blob/main/.github/workflows/rust.yaml), [ts-tests.yaml](https://github.com/atuinsh/desktop/blob/main/.github/workflows/ts-tests.yaml), [tsc-check.yaml](https://github.com/atuinsh/desktop/blob/main/.github/workflows/tsc-check.yaml))

2. Frontend/backend boundary types are manually duplicated.
   - Red Pen manual TS types: `/src/lib/types.ts` ([ref](https://github.com/phin-tech/redpen/blob/main/src/lib/types.ts))
   - Red Pen Rust source types: `/crates/redpen-core/src/annotation.rs` ([ref](https://github.com/phin-tech/redpen/blob/main/crates/redpen-core/src/annotation.rs))
   - Atuin ts-rs pattern: `/crates/atuin-desktop-runtime/src/events/mod.rs` ([ref](https://github.com/atuinsh/desktop/blob/main/crates/atuin-desktop-runtime/src/events/mod.rs))

3. No automated JS/Rust Tauri plugin parity check.
   - Red Pen plugin deps (JS): `/package.json` ([ref](https://github.com/phin-tech/redpen/blob/main/package.json))
   - Red Pen plugin deps (Rust): `/src-tauri/Cargo.toml` ([ref](https://github.com/phin-tech/redpen/blob/main/src-tauri/Cargo.toml))
   - Atuin parity script: `/script/check-tauri-versions` ([ref](https://github.com/atuinsh/desktop/blob/main/script/check-tauri-versions))

4. TypeScript compiler strictness can be tightened further.
   - Red Pen TS config: `/tsconfig.json` ([ref](https://github.com/phin-tech/redpen/blob/main/tsconfig.json))
   - Atuin TS config adds: `noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch` ([ref](https://github.com/atuinsh/desktop/blob/main/tsconfig.json))

5. Repo automation and contribution templates are minimal.
   - Red Pen has no Dependabot/PR template/issue templates yet (`/.github` currently mostly workflows)
   - Atuin examples: [dependabot.yml](https://github.com/atuinsh/desktop/blob/main/.github/dependabot.yml), [pull_request_template.md](https://github.com/atuinsh/desktop/blob/main/.github/pull_request_template.md), [ISSUE_TEMPLATE/bug_report.md](https://github.com/atuinsh/desktop/blob/main/.github/ISSUE_TEMPLATE/bug_report.md), [CONTRIBUTING.md](https://github.com/atuinsh/desktop/blob/main/CONTRIBUTING.md)

6. Release pipeline is largely manual and changelog generation is not automated.
   - Red Pen release tasks: `/Taskfile.yml` ([ref](https://github.com/phin-tech/redpen/blob/main/Taskfile.yml))
   - Atuin release automation: [tauri-release.yaml](https://github.com/atuinsh/desktop/blob/main/.github/workflows/tauri-release.yaml), [cliff.toml](https://github.com/atuinsh/desktop/blob/main/cliff.toml)

## Proposed Implementation Plan

### Phase 1 (Quick Wins): CI Quality Gates and Repo Hygiene

- [ ] Add `tsc` check and frontend test workflow split.
  - New files: `/.github/workflows/tsc.yml`, `/.github/workflows/frontend-tests.yml`
  - Keep existing `ci.yml` or reduce it to Rust-focused gates.

- [ ] Add Rust style/lint gates.
  - Update `/.github/workflows/ci.yml` to run `cargo fmt -- --check` and `cargo clippy -- -D warnings`.

- [ ] Tighten TypeScript compiler settings.
  - Update `/tsconfig.json` with:
    - `noUnusedLocals: true`
    - `noUnusedParameters: true`
    - `noFallthroughCasesInSwitch: true`

- [ ] Add contributor hygiene files.
  - Create `/.github/dependabot.yml`
  - Create `/.github/pull_request_template.md`
  - Create `/.github/ISSUE_TEMPLATE/bug_report.md`
  - Create `/.github/ISSUE_TEMPLATE/feature_request.md`

### Phase 2: Boundary Safety and Dependency Invariants

- [ ] Add Tauri plugin version parity check.
  - Add script: `/scripts/check-tauri-plugin-versions.mjs`
  - Add npm script: `"check:tauri-plugins": "node scripts/check-tauri-plugin-versions.mjs"`
  - Add CI step in TS workflow to fail on plugin version drift.

- [ ] Start ts-rs binding generation for shared types.
  - Add `ts-rs` derive to selected Rust DTOs in `/crates/redpen-core/src/annotation.rs` (and related shared structs).
  - Add output directory (example): `/src/lib/bindings/`
  - Add generation script and CI check ensuring generated files are up to date.

### Phase 3: Rust/Tauri Coding Practice Improvements

- [ ] Introduce typed command errors internally and map to strings only at Tauri command boundary.
  - Current command string-error example: `/src-tauri/src/commands/annotations.rs`
  - Proposed: internal `enum CommandError` per module, `impl From<...>` conversions, final `.map_err(|e| e.to_string())` at command return.

- [ ] Reduce panic-prone `unwrap()` in non-test runtime paths.
  - Prioritize non-test usage in:
    - `/src-tauri/src/commands/annotations.rs`
    - `/src-tauri/src/commands/files.rs`
    - `/src-tauri/src/workspace_index.rs`
    - `/src-tauri/src/lib.rs`
    - `/crates/redpen-cli/src/main.rs`

### Phase 4: Release Automation

- [ ] Add automatic changelog generation.
  - Add `cliff.toml` (or equivalent release-note generator config).

- [ ] Add tag-triggered release workflow.
  - Create `/.github/workflows/release.yml` that:
    - builds app/CLI artifacts,
    - attaches artifacts to GitHub release,
    - uses generated changelog body.

## Suggested Priority

1. Phase 1 (highest ROI, lowest risk)
2. Phase 2 (boundary safety and regression prevention)
3. Phase 3 (robustness hardening)
4. Phase 4 (release workflow maturity)

## Acceptance Criteria

1. PRs fail when TS type-checking or Rust lint/format checks fail.
2. CI fails if JS and Rust Tauri plugin versions drift.
3. Shared Rust/TS boundary types are generated from Rust for at least one core domain.
4. Contributor templates and dependency automation are in place.
5. Tagged release can be created without manual asset/changelog assembly.

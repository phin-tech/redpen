# CI/CD & Release System Design

## Context

Red Pen is a Tauri v2 desktop app (Svelte frontend, Rust backend) with a companion CLI binary (`redpen`). Both share a `redpen-core` crate. There are currently no GitHub Actions workflows. The project uses both pnpm and bun — we're consolidating on bun.

## Goals

- Automated CI on PRs and pushes to `main`
- Tag-triggered releases that build and publish both the macOS app and CLI
- macOS Apple Silicon (aarch64) only for now
- No code signing (deferred)

## Prep Changes

### 1. Consolidate on bun

- Remove `pnpm-lock.yaml`
- Update `src-tauri/tauri.conf.json`:
  - `beforeBuildCommand`: `"pnpm build"` -> `"bun run build"`
  - `beforeDevCommand`: `"pnpm dev"` -> `"bun run dev"`

### 2. Pin Rust toolchain

Add `rust-toolchain.toml` at repo root:

```toml
[toolchain]
channel = "stable"
targets = ["aarch64-apple-darwin"]
```

### 3. Clean up redpen.json artifacts

- Delete all `*.redpen.json` files from the repo
- Add `*.redpen.json` and `.redpen/` to `.gitignore`

## Workflow 1: `ci.yml` — Continuous Integration

**Triggers:** Push to `main`, pull requests targeting `main`

**Runner:** `macos-latest` (Apple Silicon)

**Steps:**

1. Checkout repo
2. Install Rust stable via `dtolnay/rust-toolchain` (reads `rust-toolchain.toml`)
3. Cache Cargo registry and target directory
4. Install bun via `oven-sh/setup-bun`
5. `bun install`
6. `cargo check --workspace`
7. `cargo test --workspace`
8. `bun run build` (verify frontend compiles)

## Workflow 2: `release.yml` — Build & Release

**Triggers:** Tags matching `v*`

**Runner:** `macos-latest` (Apple Silicon)

**Steps:**

1. Checkout repo
2. Install Rust stable via `dtolnay/rust-toolchain`
3. Cache Cargo registry and target directory
4. Install bun via `oven-sh/setup-bun`
5. `bun install`
6. Build Tauri app: `bun run tauri build --target aarch64-apple-darwin`
   - Produces `.dmg` in `src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/`
7. Build CLI: `cargo build --release -p redpen-cli --target aarch64-apple-darwin`
   - Produces `redpen` binary in `src-tauri/target/aarch64-apple-darwin/release/`
   - Note: workspace target dir is `src-tauri/target/` per Tauri convention
8. Create GitHub Release using `softprops/action-gh-release`
9. Attach artifacts:
   - `Red Pen_0.1.0_aarch64.dmg` (Tauri app)
   - `redpen` (CLI binary, renamed to `redpen-aarch64-apple-darwin`)

## Artifact naming

- App: Keep Tauri's default DMG naming
- CLI: Append target triple for clarity (`redpen-aarch64-apple-darwin`), making it easy to add more targets later

## What's deferred

- Code signing and notarization
- Intel (x86_64) builds
- Linux/Windows builds
- Auto-updating via Tauri's updater plugin
- Version bumping automation

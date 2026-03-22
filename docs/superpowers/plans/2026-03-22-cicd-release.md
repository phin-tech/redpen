# CI/CD & Release System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Set up GitHub Actions CI and tag-triggered releases for the macOS Apple Silicon Tauri app and CLI.

**Architecture:** Single CI workflow for PR/push validation, single release workflow triggered by `v*` tags that builds both the Tauri `.dmg` and CLI binary and publishes them as a GitHub Release. Prep work consolidates on bun and cleans up repo artifacts.

**Tech Stack:** GitHub Actions, Tauri v2, Rust stable, bun, `dtolnay/rust-toolchain`, `oven-sh/setup-bun`, `softprops/action-gh-release`

---

## File Structure

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `.github/workflows/ci.yml` | CI on PRs and main pushes |
| Create | `.github/workflows/release.yml` | Tag-triggered build and release |
| Create | `rust-toolchain.toml` | Pin Rust stable + aarch64 target |
| Modify | `src-tauri/tauri.conf.json` | Switch pnpm commands to bun |
| Modify | `.gitignore` | Add `*.redpen.json` and `.redpen/` |
| Delete | `README.md.redpen.json` | Artifact cleanup |
| Delete | `docs/plans/add-root-button.md.redpen.json` | Artifact cleanup |
| Delete | `docs/superpowers/specs/2026-03-22-cicd-release-design.md.redpen.json` | Artifact cleanup |
| Delete | `pnpm-lock.yaml` | Consolidate on bun |

---

### Task 1: Clean up repo and consolidate on bun

**Files:**
- Delete: `pnpm-lock.yaml`
- Delete: `README.md.redpen.json`
- Delete: `docs/plans/add-root-button.md.redpen.json`
- Delete: `docs/superpowers/specs/2026-03-22-cicd-release-design.md.redpen.json`
- Modify: `.gitignore`
- Modify: `src-tauri/tauri.conf.json`
- Create: `rust-toolchain.toml`

- [ ] **Step 1: Remove pnpm lock file**

```bash
rm pnpm-lock.yaml
```

- [ ] **Step 2: Remove all .redpen.json files**

```bash
rm README.md.redpen.json
rm docs/plans/add-root-button.md.redpen.json
rm docs/superpowers/specs/2026-03-22-cicd-release-design.md.redpen.json
```

- [ ] **Step 3: Add redpen patterns to .gitignore**

Append to `.gitignore`:

```
*.redpen.json
.redpen/
```

- [ ] **Step 4: Update tauri.conf.json commands**

In `src-tauri/tauri.conf.json`, change:

```json
"beforeDevCommand": "bun run dev",
"beforeBuildCommand": "bun run build"
```

- [ ] **Step 5: Create rust-toolchain.toml**

```toml
[toolchain]
channel = "stable"
targets = ["aarch64-apple-darwin"]
```

- [ ] **Step 6: Verify bun install and build work locally**

```bash
bun install
bun run build
```

Expected: Frontend builds successfully with no pnpm references.

- [ ] **Step 7: Commit**

```bash
git rm pnpm-lock.yaml
git rm README.md.redpen.json docs/plans/add-root-button.md.redpen.json docs/superpowers/specs/2026-03-22-cicd-release-design.md.redpen.json
git add .gitignore src-tauri/tauri.conf.json rust-toolchain.toml
git commit -m "chore: consolidate on bun, clean up redpen artifacts, pin rust toolchain"
```

---

### Task 2: Create CI workflow

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Create `.github/workflows/ci.yml`**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true

jobs:
  check:
    name: Check & Test
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: src-tauri

      - uses: oven-sh/setup-bun@v2

      - name: Install frontend dependencies
        run: bun install

      - name: Cargo check
        run: cargo check --workspace
        working-directory: src-tauri

      - name: Cargo test
        run: cargo test --workspace
        working-directory: src-tauri

      - name: Build frontend
        run: bun run build
```

- [ ] **Step 2: Verify YAML is valid**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"
```

Expected: No errors.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add CI workflow for PRs and main branch"
```

---

### Task 3: Create release workflow

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Create `.github/workflows/release.yml`**

```yaml
name: Release

on:
  push:
    tags:
      - "v*"

permissions:
  contents: write

jobs:
  release:
    name: Build & Release
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: src-tauri

      - uses: oven-sh/setup-bun@v2

      - name: Install frontend dependencies
        run: bun install

      - name: Build Tauri app
        run: bun run tauri build --target aarch64-apple-darwin

      - name: Build CLI
        run: cargo build --release -p redpen-cli --target aarch64-apple-darwin
        working-directory: src-tauri

      - name: Prepare artifacts
        run: |
          cp src-tauri/target/aarch64-apple-darwin/release/redpen redpen-aarch64-apple-darwin

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          generate_release_notes: true
          files: |
            src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/*.dmg
            redpen-aarch64-apple-darwin
```

- [ ] **Step 2: Verify YAML is valid**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"
```

Expected: No errors.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add tag-triggered release workflow for macOS builds"
```

---

### Task 4: End-to-end validation

- [ ] **Step 1: Verify all workflows parse correctly**

```bash
python3 -c "
import yaml, glob
for f in glob.glob('.github/workflows/*.yml'):
    yaml.safe_load(open(f))
    print(f'{f}: OK')
"
```

- [ ] **Step 2: Verify cargo workspace builds**

```bash
cd src-tauri && cargo check --workspace
```

Expected: No errors.

- [ ] **Step 3: Verify bun frontend builds**

```bash
bun run build
```

Expected: No errors.

- [ ] **Step 4: Push to main and verify CI runs**

Push the branch and confirm the CI workflow triggers on GitHub.

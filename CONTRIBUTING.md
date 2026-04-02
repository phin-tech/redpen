# Contributing to Red Pen

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Bun](https://bun.sh/) (or npm)
- Tauri v2 system dependencies — see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

## Setup

```bash
git clone https://github.com/phin-tech/redpen.git
cd redpen
bun install
```

## Development

```bash
bun run tauri dev    # Full Tauri dev (frontend + backend)
bun run dev          # Frontend only (no Rust)
```

## Testing

```bash
cargo test           # Rust tests
cargo clippy         # Rust lints
bun run build        # Frontend build check
```

## Project structure

```
src/                    # Svelte 5 frontend
src-tauri/              # Tauri backend (Rust)
crates/redpen-core/     # Core annotation logic (Rust library)
crates/redpen-cli/      # CLI tool
crates/redpen-runtime/  # Runtime crate
channels/redpen-channel # MCP channel server
plugin/                 # Claude Code plugin (hooks + skills)
```

## Pull requests

- Branch from `main`
- Keep PRs focused — one issue per PR
- Fill out the PR template checklist
- Ensure CI passes before requesting review

# Red Pen

A desktop code annotation tool for reviewing and marking up source code. Add comments, notes, and labels anchored to specific lines — annotations stay attached even when code changes.

Built with [Tauri](https://tauri.app), [Svelte 5](https://svelte.dev), and [CodeMirror](https://codemirror.net).

## Install

### From Releases

Download the latest `.dmg` from the [Releases](https://github.com/phin-tech/redpen/releases) page.

> **Note:** The app is currently unsigned. macOS will quarantine it on first download. Run the following to clear the quarantine flag:
> ```bash
> xattr -dr com.apple.quarantine /Applications/Red\ Pen.app
> ```

### From Source

```bash
bun install
bun run tauri build
cp -R src-tauri/target/release/bundle/macos/Red\ Pen.app /Applications/
```

### CLI

```bash
cargo install --path crates/redpen-cli
```

## Desktop App

### Opening a Project

- **Drag and drop** a folder onto the app window
- **Deep link**: `open 'redpen://open?file=/path/to/file.rs'` (auto-detects git repo root)
- **Add folder button**: click the `+` icon at the top of the file tree
- **CLI**: `redpen open <file>`

### Creating Annotations

1. Open a file from the file tree
2. Select text in the editor
3. Press `Cmd+Enter` (or `Ctrl+Enter`)
4. Enter your comment and optional labels
5. Click **Save**

Annotations appear in the right sidebar, sorted by line number. Click an annotation to jump to that line.

### Editing & Deleting

- **Double-click** an annotation card to edit
- **Click the trash icon** to delete

### Filtering Changed Files

Click the pen icon at the top of the file tree to toggle between all files and only git-modified files.

### Annotation Storage

Annotations are stored as `.redpen.json` sidecar files alongside source files. They include anchor context (line content, surrounding lines, content hash) so annotations can be relocated when code changes.

## CLI Reference

```
redpen annotate <file> --body "comment" [--line N] [--label tag] [--author name]
redpen list <file>              # Output annotations as JSON
redpen export <file>            # Export as markdown
redpen status <file>            # Show annotation count
redpen open <file> [--line N]   # Open in desktop app via deep link
redpen wait <file> [--timeout N] # Block until review is complete
```

### Examples

```bash
# Add an annotation to line 42
redpen annotate src/main.rs --line 42 --body "This needs error handling" --label todo

# List all annotations as JSON
redpen list src/main.rs

# Export to markdown
redpen export src/main.rs --output review.md

# Check annotation count
redpen status src/main.rs
```

## Agent Review Workflow

Red Pen supports a structured review loop for AI agents and automated workflows. There are two modes: **blocking** (CLI) and **async** (channel).

### Blocking: `redpen wait`

The agent writes a file, opens it for review, and blocks until the human clicks "Done Reviewing" in the app.

```bash
# Agent writes a plan, opens it, and waits
redpen open docs/plan.md
annotations=$(redpen wait docs/plan.md --timeout 300)
# $annotations contains the review feedback as JSON
```

The human reviews in the Red Pen app, adds annotations, and clicks **Done Reviewing** in the sidebar. The `wait` command unblocks and outputs all annotations as JSON.

### Async: Channel Push

For non-blocking workflows, use the Red Pen channel server with [Claude Code channels](https://code.claude.com/docs/en/channels). When the human clicks "Done Reviewing", annotations are pushed directly into the Claude Code session.

```bash
# Start Claude Code with the redpen channel
claude --dangerously-load-development-channels server:redpen
```

The channel server listens on `localhost:8789` (configurable via `REDPEN_CHANNEL_PORT`). When "Done Reviewing" is clicked, the app POSTs annotations to the channel, which pushes them into the session as a `<channel source="redpen">` event.

### Annotation JSON Format

Both `redpen wait` and `redpen list` return an array of annotations:

```json
[
  {
    "id": "UUID",
    "kind": "comment",
    "body": "reviewer's comment",
    "labels": ["todo"],
    "author": "username",
    "anchor": {
      "type": "textContext",
      "lineContent": "the annotated line",
      "range": { "startLine": 8, "startColumn": 0, "endLine": 8, "endColumn": 40 }
    }
  }
]
```

## Supported Languages

Syntax highlighting for: JavaScript, TypeScript, JSX, TSX, Python, Rust, Go, Java, C/C++, CSS, HTML, Markdown, JSON, XML, SVG, SQL, YAML, and Svelte.

## Development

```bash
bun install
bun run tauri dev
```

### Project Structure

```
src/                    # Svelte frontend
src-tauri/              # Tauri backend (Rust)
crates/redpen-core/     # Core annotation logic (Rust library)
crates/redpen-cli/      # CLI tool
channels/redpen-channel # MCP channel server for async review
```

## Deep Link URL Scheme

Red Pen registers the `redpen://` URL scheme. Format:

```
redpen://open?file=/absolute/path/to/file&line=42
```

If the file is inside a git repository, the app automatically opens the repo root in the file tree.

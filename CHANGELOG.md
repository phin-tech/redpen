## [0.1.7] - 2026-03-24

### Bug Fixes

- Correct marketplace.json schema for plugin discovery
Add required name/owner fields and use source instead of path.

### Features

- Add marketplace.json for plugin discovery
Enables installation via `/plugin marketplace add phin-tech/redpen`.
- MacOS code signing and automated release workflow
- Add tauri-action for signed builds with notarization in CI
  - Add Taskfile tasks for local signing, signed builds, and release upload
  - Configure signingIdentity in tauri.conf.json
  - Add .env* to .gitignore

### Miscellaneous

- Parallelize check and test jobs for faster CI (#8)
Split the single "Check & Test" job into two parallel jobs to cut
  wall-clock time from ~170s to ~100s. Also fix rust-cache config to
  use the workspace root target dir.


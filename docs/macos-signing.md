# macOS signing, notarization, and stapling

This repo uses Tauri's built-in macOS signing flow.
For local release builds, signing is expected to happen with a `Developer ID Application` certificate already installed in the `login` keychain on the signing Mac.

## What you need from Apple

- A paid Apple Developer account
- A `Developer ID Application` certificate for direct distribution outside the Mac App Store
- Either:
  - Apple ID notarization credentials: `APPLE_ID`, `APPLE_PASSWORD` (app-specific password), `APPLE_TEAM_ID`
  - App Store Connect API credentials: `APPLE_API_KEY`, `APPLE_API_ISSUER`, and the private key contents

## What lives in 1Password

Store these as fields or file attachments in 1Password:

- `APPLE_SIGNING_IDENTITY`: the exact `Developer ID Application: ...` identity name from `security find-identity -v -p codesigning`
- `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` for Apple ID notarization
- or `APPLE_API_KEY`, `APPLE_API_ISSUER`, `APPLE_API_KEY_CONTENT` for API-key notarization

Optional backup fields for CI or recovery:

- `APPLE_CERTIFICATE`: base64-encoded exported `.p12`
- `APPLE_CERTIFICATE_PASSWORD`: password used when exporting the `.p12`

`APPLE_API_KEY_CONTENT` should be the contents of the downloaded `AuthKey_*.p8` file. The signing wrapper writes it to a temporary file and exports `APPLE_API_KEY_PATH` for Tauri.

## Local setup

1. Install your `Developer ID Application` certificate into the `login` keychain on this Mac.
2. Confirm it appears in `security find-identity -v -p codesigning`.
3. Copy `.env.signing.example` to `.env.signing`.
4. Set `APPLE_SIGNING_IDENTITY` to the exact identity string from the previous command.
5. Replace the notarization placeholders with your own `op://...` secret references.
6. Sign in to 1Password CLI: `op signin`

## Commands

Build, sign, notarize, and staple with `op run`:

```sh
op run --env-file=.env.signing -- task sign
```

Equivalent convenience task:

```sh
task sign:op
```

## What the task does

- Runs `bun run tauri build`, which signs the app using the keychain identity and submits notarization when the Apple env vars are present
- Staples the generated DMG
- Verifies the finished `.app` signature and validates the stapled `.dmg`

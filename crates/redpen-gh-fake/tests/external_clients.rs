//! End-to-end tests using *external* GitHub clients (real binaries, not Rust):
//!   - the `gh` CLI (https + GHE-style /api/v3 paths via `GH_HOST`)
//!   - a Python script (PyGithub if installed, else stdlib urllib)
//!   - a Node script (Octokit if installed via `npm ci`, plus built-in fetch)
//!
//! These tests confirm the wire shape is genuinely GitHub-compatible — not
//! just "Rust talking to Rust over JSON".
//!
//! Tests skip with a print (rather than fail) when the required tool isn't
//! available in the environment, so `cargo test` still passes on minimal hosts.

mod common;

use std::path::PathBuf;
use std::process::Stdio;

use tokio::io::AsyncWriteExt;
use tokio::process::Command;

fn clients_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("clients")
}

async fn tool_available(tool: &str) -> bool {
    Command::new("which")
        .arg(tool)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn run_cmd(mut cmd: Command, label: &str) -> Result<String, String> {
    let out = cmd.output().await.map_err(|e| format!("{label}: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if !out.status.success() {
        return Err(format!(
            "{label} exit={:?}\nstdout:\n{}\nstderr:\n{}",
            out.status.code(),
            stdout,
            stderr
        ));
    }
    Ok(format!("{stdout}{stderr}"))
}

#[tokio::test]
async fn python_client_round_trip() {
    if !tool_available("python3").await {
        eprintln!("python3 not on PATH; skipping");
        return;
    }
    let server = common::spawn_http().await;
    let mut cmd = Command::new("python3");
    cmd.arg(clients_dir().join("test_python.py"))
        .env("BASE_URL", &server.base_url)
        .env("PARENT_ID", server.seeded_comment.database_id.to_string())
        .env("GH_FAKE_TOKEN", "py-agent")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let out = run_cmd(cmd, "python3 test_python.py").await.expect("python");
    eprintln!("--- python output ---\n{out}");

    // Side-effect verification: the python script posted a reply.
    let comments = server.backend.comments_for("session-1");
    assert!(
        comments.iter().any(|c| c.author == "py-agent"),
        "expected a reply authored by py-agent, got: {:?}",
        comments.iter().map(|c| &c.author).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn node_client_round_trip() {
    if !tool_available("node").await {
        eprintln!("node not on PATH; skipping");
        return;
    }
    if !tool_available("npm").await {
        eprintln!("npm not on PATH; skipping");
        return;
    }

    // Lazy-install @octokit/rest into tests/clients/. Idempotent + fast on
    // re-run thanks to npm's cache + node_modules check.
    let install = Command::new("npm")
        .arg("install")
        .arg("--no-fund")
        .arg("--no-audit")
        .arg("--silent")
        .current_dir(clients_dir())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .status()
        .await;
    if !matches!(install, Ok(s) if s.success()) {
        eprintln!("npm install failed; running test without octokit (fetch-only)");
    }

    let server = common::spawn_http().await;
    let mut cmd = Command::new("node");
    cmd.arg(clients_dir().join("test_node.js"))
        .env("BASE_URL", &server.base_url)
        .env("PARENT_ID", server.seeded_comment.database_id.to_string())
        .env("GH_FAKE_TOKEN", "node-agent")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let out = run_cmd(cmd, "node test_node.js").await.expect("node");
    eprintln!("--- node output ---\n{out}");

    // Side-effect: thread should now be resolved (mutation runs unconditionally).
    let threads = server.backend.threads_for("session-1");
    assert!(
        threads.iter().any(|t| t.is_resolved),
        "expected at least one resolved thread"
    );
}

#[tokio::test]
async fn gh_cli_round_trip_via_tls() {
    if !tool_available("gh").await {
        eprintln!("gh not on PATH; skipping");
        return;
    }
    // Go's TLS on macOS uses Security.framework and ignores SSL_CERT_FILE,
    // so a self-signed cert won't be trusted unless it's been added to the
    // keychain manually (`security add-trusted-cert`) or generated via
    // `mkcert -install`. Skip cleanly rather than fail on developer machines.
    #[cfg(target_os = "macos")]
    {
        eprintln!(
            "gh CLI test skipped on macOS: gh's Go runtime ignores SSL_CERT_FILE here. \
             Enable on Linux CI, or run `brew install mkcert && mkcert -install` and \
             rerun with REDPEN_GH_FAKE_FORCE_GH_TEST=1."
        );
        if std::env::var_os("REDPEN_GH_FAKE_FORCE_GH_TEST").is_none() {
            return;
        }
    }

    let server = common::spawn_https().await;
    let cert_pem = server.cert_pem.clone().expect("https spawn returned cert");
    let cert_path = std::env::temp_dir().join(format!("redpen-gh-fake-{}.pem", std::process::id()));
    let mut f = tokio::fs::File::create(&cert_path).await.unwrap();
    f.write_all(cert_pem.as_bytes()).await.unwrap();
    drop(f);

    // GH_HOST strips the scheme; gh prepends https://.
    let host = server.base_url.trim_start_matches("https://");

    let mut cmd = Command::new("bash");
    cmd.arg(clients_dir().join("test_gh_cli.sh"))
        .env("GH_HOST", host)
        .env("GH_TOKEN", "gh-agent")
        .env("SSL_CERT_FILE", &cert_path)
        // On macOS Go uses Security.framework by default; the fallback flag
        // forces honoring SSL_CERT_FILE.
        .env("GODEBUG", "x509usefallbackroots=1")
        .env("PARENT_ID", server.seeded_comment.database_id.to_string())
        .env("GH_PROMPT_DISABLED", "1")
        .env("NO_COLOR", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let result = run_cmd(cmd, "gh CLI test").await;
    let _ = tokio::fs::remove_file(&cert_path).await;
    let out = result.expect("gh");
    eprintln!("--- gh output ---\n{out}");

    let threads = server.backend.threads_for("session-1");
    assert!(
        threads.iter().any(|t| t.is_resolved),
        "expected at least one resolved thread after gh CLI run"
    );
}

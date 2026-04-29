//! Read review findings as JSON from stdin and POST them to a running
//! redpen-gh-fake server as top-level review comments.
//!
//! Input shape (one of):
//!   * `[{"path": "src/foo.rs", "line": 25, "body": "..."}, ...]`
//!   * `{"comments": [...]}`
//!
//! Required env: `BASE_URL` (e.g. http://127.0.0.1:55555), `OWNER`, `REPO`, `PR`.
//! Optional env: `REVIEWER_TOKEN` (default "codex-review") — surfaces as the
//! comment's `user.login`.
//!
//!     cat findings.json | cargo run --example seed_review

use std::io::Read;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let base = std::env::var("BASE_URL").expect("BASE_URL");
    let owner = std::env::var("OWNER").expect("OWNER");
    let repo = std::env::var("REPO").expect("REPO");
    let pr: u64 = std::env::var("PR")
        .expect("PR")
        .parse()
        .expect("PR must be numeric");
    let token = std::env::var("REVIEWER_TOKEN").unwrap_or_else(|_| "codex-review".into());

    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf).expect("stdin");
    let parsed: serde_json::Value = serde_json::from_str(&buf).expect("input must be JSON");
    let findings = match &parsed {
        serde_json::Value::Array(arr) => arr.clone(),
        serde_json::Value::Object(obj) => obj
            .get("comments")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default(),
        _ => Vec::new(),
    };

    let client = reqwest::Client::new();
    let mut posted = 0usize;
    for f in &findings {
        let path = f.get("path").and_then(|v| v.as_str()).unwrap_or("");
        let line = f.get("line").and_then(|v| v.as_u64()).unwrap_or(1);
        let body = f.get("body").and_then(|v| v.as_str()).unwrap_or("");
        if path.is_empty() || body.is_empty() {
            eprintln!("skipping malformed finding: {f}");
            continue;
        }

        let url = format!("{}/repos/{}/{}/pulls/{}/comments", base, owner, repo, pr);
        let resp = client
            .post(&url)
            .bearer_auth(&token)
            .json(&serde_json::json!({
                "path": path,
                "line": line,
                "body": body,
                "side": "RIGHT",
            }))
            .send()
            .await
            .expect("POST");
        if !resp.status().is_success() {
            eprintln!(
                "failed: {} {} (POST {})",
                resp.status(),
                resp.text().await.unwrap_or_default(),
                url
            );
            continue;
        }
        let created: serde_json::Value = resp.json().await.expect("response json");
        println!(
            "  + {}:{} (id={}, author={})",
            path, line, created["id"], created["user"]["login"]
        );
        posted += 1;
    }
    eprintln!("posted {posted}/{} finding(s)", findings.len());
}

//! Client for the optional Red Pen local HTTP server.
//!
//! All functions return `Option` — `None` means the server isn't running
//! and the caller should fall back to deep links / signal files.

use redpen_server::{OkResponse, ReviewPrResponse, ReviewStartResponse, ReviewWaitResponse};
use serde::de::DeserializeOwned;
use serde_json::json;

/// Read the server port from ~/.config/redpen/server.json.
/// Returns None if the file doesn't exist or the server process is dead.
fn server_url() -> Option<String> {
    let path = redpen_server::discovery_path();
    let content = std::fs::read_to_string(&path).ok()?;
    let info: serde_json::Value = serde_json::from_str(&content).ok()?;

    let port = info["port"].as_u64()?;
    let pid = info["pid"].as_u64()?;

    // Check if the process is still alive (Unix: kill -0)
    #[cfg(unix)]
    {
        let alive = unsafe { libc_kill(pid as i32) };
        if !alive {
            let _ = std::fs::remove_file(&path);
            return None;
        }
    }
    let _ = pid; // suppress unused warning on non-unix

    Some(format!("http://127.0.0.1:{}", port))
}

/// Check if a process is alive using raw syscall (avoids libc dependency).
#[cfg(unix)]
unsafe fn libc_kill(pid: i32) -> bool {
    // Use raw syscall — kill(pid, 0) returns 0 if process exists
    extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }
    unsafe { kill(pid, 0) == 0 }
}

fn rpc_post<T: DeserializeOwned>(endpoint: &str, body: &serde_json::Value) -> Option<T> {
    let base = server_url()?;
    let url = format!("{}/rpc/{}", base, endpoint);
    let resp: String = ureq::post(&url)
        .send_json(body)
        .ok()?
        .into_body()
        .read_to_string()
        .ok()?;
    serde_json::from_str(&resp).ok()
}


/// Open a file in the GUI via the server. Returns true if successful.
pub fn open_file(file: &str, line: Option<u32>) -> bool {
    let body = json!({"file": file, "line": line});
    rpc_post::<OkResponse>("open", &body).is_some_and(|r| r.ok)
}

/// Open a file and associate it with an existing review session.
pub fn open_file_in_session(file: &str, line: Option<u32>, session_id: &str) -> bool {
    let body = json!({"file": file, "line": line, "session_id": session_id});
    rpc_post::<OkResponse>("open", &body).is_some_and(|r| r.ok)
}

/// Refresh annotations for a file in the GUI. Returns true if successful.
pub fn refresh_file(file: &str) -> bool {
    let body = json!({"file": file});
    rpc_post::<OkResponse>("refresh", &body).is_some_and(|r| r.ok)
}

/// Start a review session (non-blocking). Returns session_id.
#[allow(dead_code)]
pub fn review_start(file: &str, line: Option<u32>, session_id: Option<&str>) -> Option<String> {
    let body = json!({"file": file, "line": line, "session_id": session_id});
    rpc_post::<ReviewStartResponse>("review.start", &body).map(|r| r.session_id)
}

/// Result of waiting for a review, distinguishing error causes.
pub enum ReviewWaitResult {
    /// Review completed with a verdict.
    Ok(ReviewWaitResponse),
    /// The server timed out waiting for a verdict.
    Timeout,
    /// Could not connect to the server.
    ServerUnavailable,
}

/// Wait for a review session to complete. Blocks.
#[allow(dead_code)]
pub fn review_wait(session_id: &str, timeout: Option<u64>) -> ReviewWaitResult {
    let timeout_secs = timeout.unwrap_or(86400);
    let body = json!({
        "session_id": session_id,
        "timeout": timeout_secs,
    });
    let base = match server_url() {
        Some(url) => url,
        None => return ReviewWaitResult::ServerUnavailable,
    };
    let url = format!("{}/rpc/review.wait", base);
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(timeout_secs + 5)))
        .build();
    let agent = ureq::Agent::new_with_config(config);
    match agent.post(&url).send_json(&body) {
        Ok(resp) => {
            let status = resp.status();
            if let Ok(body_str) = resp.into_body().read_to_string() {
                if status == 504 {
                    return ReviewWaitResult::Timeout;
                }
                if let Ok(parsed) = serde_json::from_str(&body_str) {
                    return ReviewWaitResult::Ok(parsed);
                }
            }
            ReviewWaitResult::ServerUnavailable
        }
        Err(_) => ReviewWaitResult::Timeout,
    }
}

/// Get all annotations for a review session.
pub fn session_annotations(session_id: &str) -> Option<serde_json::Value> {
    let body = json!({"session_id": session_id});
    rpc_post::<serde_json::Value>("session.annotations", &body)
}

pub fn review_pr(pr_ref: &str, local_path_hint: Option<&str>) -> Option<ReviewPrResponse> {
    let body = json!({
        "pr_ref": pr_ref,
        "local_path_hint": local_path_hint,
    });
    rpc_post::<ReviewPrResponse>("review.pr", &body)
}

/// Check if the server is available.
pub fn is_available() -> bool {
    server_url().is_some()
}

//! Local HTTP server for Red Pen — RPC-style API.
//!
//! All endpoints are `POST` with JSON request/response bodies.
//! This replaces deep links, signal files, and the channel server
//! with reliable, bidirectional HTTP communication.

use axum::{
    extract::State as AxumState, http::StatusCode, response::IntoResponse, routing::post, Json,
    Router,
};
use redpen_core::annotation::Annotation;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

// ---------------------------------------------------------------------------
// AppBridge — abstracts Tauri so the server is testable
// ---------------------------------------------------------------------------

pub trait AppBridge: Send + Sync + 'static {
    /// Tell the GUI to open a file (replaces deep links).
    fn open_file(
        &self,
        file: &str,
        line: Option<u32>,
        review_session_id: Option<&str>,
    ) -> Result<(), String>;
    /// Tell the GUI to refresh annotations for a file.
    fn refresh_file(&self, file: &str) -> Result<(), String>;
    /// Load annotations for a file from the sidecar store.
    fn get_annotations(&self, file: &str) -> Result<Vec<Annotation>, String>;
    /// Open a GitHub PR review and return the managed worktree path.
    fn open_pr_review(&self, pr_ref: &str, local_path_hint: Option<&str>)
        -> Result<String, String>;
    /// Persist a newly-started review session.
    fn start_review_session(&self, session_id: &str, file: &str) -> Result<(), String>;
    /// Persist review completion state.
    fn complete_review_session(&self, session_id: &str, verdict: &str) -> Result<(), String>;
    /// Cancel an active review session.
    fn cancel_review_session(&self, session_id: &str) -> Result<(), String>;
    /// Mark a review session as timed out.
    fn timeout_review_session(&self, session_id: &str) -> Result<(), String>;
    /// Read persisted status for a review session.
    fn review_session_status(&self, session_id: &str)
        -> Result<Option<ReviewSessionState>, String>;
}

// ---------------------------------------------------------------------------
// RPC request / response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct OpenRequest {
    pub file: String,
    pub line: Option<u32>,
    /// If provided, associates this file with the given review session.
    pub session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub file: String,
}

#[derive(Debug, Deserialize)]
pub struct GetAnnotationsRequest {
    pub file: String,
}

#[derive(Debug, Deserialize)]
pub struct ReviewStartRequest {
    pub file: String,
    pub line: Option<u32>,
    pub session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReviewDoneRequest {
    pub session_id: String,
    pub verdict: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReviewCancelRequest {
    pub session_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ReviewWaitRequest {
    pub session_id: String,
    pub timeout: Option<u64>,
}

/// Combined open + wait — the most common agent use case.
#[derive(Debug, Deserialize)]
pub struct ReviewRequest {
    pub file: String,
    pub line: Option<u32>,
    pub timeout: Option<u64>,
    pub session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReviewPrRequest {
    pub pr_ref: String,
    pub local_path_hint: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PushCheckRequest {
    #[allow(dead_code)]
    pub repo_root: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PushCheckResponse {
    pub approved: bool,
}

#[derive(Debug, Deserialize)]
pub struct SessionAnnotationsRequest {
    pub session_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OkResponse {
    pub ok: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewStartResponse {
    pub session_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewWaitResponse {
    pub verdict: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewResponse {
    pub session_id: String,
    pub verdict: String,
    pub annotations: Vec<Annotation>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewPrResponse {
    pub worktree_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewSessionState {
    pub status: String,
    pub file: Option<String>,
    pub verdict: Option<String>,
}

// ---------------------------------------------------------------------------
// ReviewSessions — in-memory session tracking (replaces signal files)
// ---------------------------------------------------------------------------

pub struct ReviewSessions {
    senders: Mutex<HashMap<String, (String, oneshot::Sender<String>)>>,
    receivers: Mutex<HashMap<String, oneshot::Receiver<String>>>,
    /// Track all files associated with each session (for session-wide annotation queries).
    session_files: Mutex<HashMap<String, Vec<String>>>,
}

impl Default for ReviewSessions {
    fn default() -> Self {
        Self {
            senders: Mutex::new(HashMap::new()),
            receivers: Mutex::new(HashMap::new()),
            session_files: Mutex::new(HashMap::new()),
        }
    }
}

impl ReviewSessions {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn create(&self, file: String) -> String {
        self.create_with_id(None, file).await
    }

    pub async fn create_with_id(&self, id: Option<String>, file: String) -> String {
        let session_id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let (tx, rx) = oneshot::channel();
        self.senders
            .lock()
            .await
            .insert(session_id.clone(), (file.clone(), tx));
        self.receivers.lock().await.insert(session_id.clone(), rx);
        self.session_files
            .lock()
            .await
            .entry(session_id.clone())
            .or_default()
            .push(file);
        session_id
    }

    /// Add an additional file to a session's file list.
    pub async fn add_file(&self, session_id: &str, file: String) {
        self.session_files
            .lock()
            .await
            .entry(session_id.to_string())
            .or_default()
            .push(file);
    }

    /// Get all tracked session IDs.
    pub async fn session_ids(&self) -> Vec<String> {
        self.session_files.lock().await.keys().cloned().collect()
    }

    /// Get all files associated with a session.
    pub async fn get_files(&self, session_id: &str) -> Vec<String> {
        self.session_files
            .lock()
            .await
            .get(session_id)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn take_receiver(&self, session_id: &str) -> Option<oneshot::Receiver<String>> {
        self.receivers.lock().await.remove(session_id)
    }

    pub async fn complete(&self, session_id: &str, verdict: &str) -> Option<String> {
        let mut senders = self.senders.lock().await;
        if let Some((file, sender)) = senders.remove(session_id) {
            let _ = sender.send(verdict.to_string());
            Some(file)
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct ServerState {
    pub bridge: Arc<dyn AppBridge>,
    pub sessions: Arc<ReviewSessions>,
}

// ---------------------------------------------------------------------------
// RPC handlers
// ---------------------------------------------------------------------------

async fn rpc_open(
    AxumState(state): AxumState<ServerState>,
    Json(req): Json<OpenRequest>,
) -> impl IntoResponse {
    // If a session_id is provided, track this file in the session
    if let Some(ref sid) = req.session_id {
        state.sessions.add_file(sid, req.file.clone()).await;
    }
    match state
        .bridge
        .open_file(&req.file, req.line, req.session_id.as_deref())
    {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({"ok": true}))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

async fn rpc_refresh(
    AxumState(state): AxumState<ServerState>,
    Json(req): Json<RefreshRequest>,
) -> impl IntoResponse {
    match state.bridge.refresh_file(&req.file) {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({"ok": true}))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

async fn rpc_get_annotations(
    AxumState(state): AxumState<ServerState>,
    Json(req): Json<GetAnnotationsRequest>,
) -> impl IntoResponse {
    match state.bridge.get_annotations(&req.file) {
        Ok(annotations) => (StatusCode::OK, Json(serde_json::json!(annotations))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        )
            .into_response(),
    }
}

async fn rpc_review_start(
    AxumState(state): AxumState<ServerState>,
    Json(req): Json<ReviewStartRequest>,
) -> impl IntoResponse {
    let file = req.file.clone();
    let session_id = state
        .sessions
        .create_with_id(req.session_id, file.clone())
        .await;
    let _ = state.bridge.start_review_session(&session_id, &file);
    let _ = state.bridge.open_file(&file, req.line, Some(&session_id));
    (StatusCode::OK, Json(ReviewStartResponse { session_id }))
}

async fn rpc_review_done(
    AxumState(state): AxumState<ServerState>,
    Json(req): Json<ReviewDoneRequest>,
) -> impl IntoResponse {
    let verdict = req.verdict.as_deref().unwrap_or("approved");
    let _ = state
        .bridge
        .complete_review_session(&req.session_id, verdict);
    match state.sessions.complete(&req.session_id, verdict).await {
        Some(_file) => (StatusCode::OK, Json(OkResponse { ok: true })),
        None => (StatusCode::NOT_FOUND, Json(OkResponse { ok: false })),
    }
}

async fn rpc_review_cancel(
    AxumState(state): AxumState<ServerState>,
    Json(req): Json<ReviewCancelRequest>,
) -> impl IntoResponse {
    let _ = state.bridge.cancel_review_session(&req.session_id);
    // Also complete the in-memory channel so any waiters unblock
    match state.sessions.complete(&req.session_id, "cancelled").await {
        Some(_) => (StatusCode::OK, Json(OkResponse { ok: true })),
        None => (StatusCode::OK, Json(OkResponse { ok: true })),
    }
}

async fn rpc_review_wait(
    AxumState(state): AxumState<ServerState>,
    Json(req): Json<ReviewWaitRequest>,
) -> impl IntoResponse {
    let rx = state.sessions.take_receiver(&req.session_id).await;
    match rx {
        Some(rx) => {
            let timeout_secs = req.timeout.unwrap_or(300);
            let result =
                tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), rx).await;
            match result {
                Ok(Ok(verdict)) => (
                    StatusCode::OK,
                    Json(serde_json::json!(ReviewWaitResponse { verdict })),
                )
                    .into_response(),
                Ok(Err(_)) => {
                    let _ = state.bridge.cancel_review_session(&req.session_id);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({"error": "session cancelled"})),
                    )
                        .into_response()
                }
                Err(_) => {
                    let _ = state.bridge.timeout_review_session(&req.session_id);
                    (
                        StatusCode::GATEWAY_TIMEOUT,
                        Json(serde_json::json!({"error": "review timed out"})),
                    )
                        .into_response()
                }
            }
        }
        None => {
            persisted_wait_response(&state.bridge, &req.session_id, req.timeout.unwrap_or(300))
                .await
        }
    }
}

/// Combined open + wait. Blocks until the reviewer signals done.
async fn rpc_review(
    AxumState(state): AxumState<ServerState>,
    Json(req): Json<ReviewRequest>,
) -> impl IntoResponse {
    let file = req.file.clone();
    let session_id = state
        .sessions
        .create_with_id(req.session_id, file.clone())
        .await;
    let _ = state.bridge.start_review_session(&session_id, &file);
    let _ = state.bridge.open_file(&file, req.line, Some(&session_id));
    let rx = state
        .sessions
        .take_receiver(&session_id)
        .await
        .expect("receiver must exist right after create");

    let timeout_secs = req.timeout.unwrap_or(300);
    let result = tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), rx).await;

    match result {
        Ok(Ok(verdict)) => {
            let annotations = state.bridge.get_annotations(&file).unwrap_or_default();
            (
                StatusCode::OK,
                Json(serde_json::json!(ReviewResponse {
                    session_id,
                    verdict,
                    annotations,
                })),
            )
                .into_response()
        }
        Ok(Err(_)) => {
            let _ = state.bridge.cancel_review_session(&session_id);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "session cancelled"})),
            )
                .into_response()
        }
        Err(_) => {
            let _ = state.bridge.timeout_review_session(&session_id);
            (
                StatusCode::GATEWAY_TIMEOUT,
                Json(serde_json::json!({"error": "review timed out"})),
            )
                .into_response()
        }
    }
}

/// Return all annotations for all files in a session.
async fn rpc_session_annotations(
    AxumState(state): AxumState<ServerState>,
    Json(req): Json<SessionAnnotationsRequest>,
) -> impl IntoResponse {
    let files = state.sessions.get_files(&req.session_id).await;
    if files.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "session not found or has no files"})),
        )
            .into_response();
    }

    let mut result = Vec::new();
    for file in &files {
        let annotations = state.bridge.get_annotations(file).unwrap_or_default();
        result.push(serde_json::json!({
            "file": file,
            "annotations": annotations,
        }));
    }
    (StatusCode::OK, Json(serde_json::json!(result))).into_response()
}

async fn rpc_review_pr(
    AxumState(state): AxumState<ServerState>,
    Json(req): Json<ReviewPrRequest>,
) -> impl IntoResponse {
    match state
        .bridge
        .open_pr_review(&req.pr_ref, req.local_path_hint.as_deref())
    {
        Ok(worktree_path) => (
            StatusCode::OK,
            Json(serde_json::json!(ReviewPrResponse { worktree_path })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        )
            .into_response(),
    }
}

/// Check whether any session has an "approved" verdict (used by the git push hook).
async fn rpc_push_check(
    AxumState(state): AxumState<ServerState>,
    Json(_req): Json<PushCheckRequest>,
) -> impl IntoResponse {
    // Check all tracked sessions for a recent approval via the persisted state.
    let ids = state.sessions.session_ids().await;
    for session_id in &ids {
        if let Ok(Some(status)) = state.bridge.review_session_status(session_id) {
            if status.verdict.as_deref() == Some("approved") {
                return Json(PushCheckResponse { approved: true });
            }
        }
    }

    Json(PushCheckResponse { approved: false })
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn build_router(bridge: Arc<dyn AppBridge>, sessions: Arc<ReviewSessions>) -> Router {
    let state = ServerState { bridge, sessions };

    Router::new()
        .route("/rpc/open", post(rpc_open))
        .route("/rpc/refresh", post(rpc_refresh))
        .route("/rpc/get_annotations", post(rpc_get_annotations))
        .route("/rpc/review.start", post(rpc_review_start))
        .route("/rpc/review.done", post(rpc_review_done))
        .route("/rpc/review.cancel", post(rpc_review_cancel))
        .route("/rpc/review.wait", post(rpc_review_wait))
        .route("/rpc/review", post(rpc_review))
        .route("/rpc/review.pr", post(rpc_review_pr))
        .route("/rpc/session.annotations", post(rpc_session_annotations))
        .route("/rpc/push.check", post(rpc_push_check))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Server startup
// ---------------------------------------------------------------------------

pub fn discovery_path() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join(".config")
        .join("redpen")
        .join("server.json")
}

pub async fn start_server(
    bridge: Arc<dyn AppBridge>,
    sessions: Arc<ReviewSessions>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let router = build_router(bridge, sessions);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    let discovery = discovery_path();
    if let Some(parent) = discovery.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(
        &discovery,
        serde_json::to_string_pretty(&serde_json::json!({
            "pid": std::process::id(),
            "port": addr.port(),
        }))?,
    )?;

    eprintln!("Red Pen server listening on {}", addr);
    axum::serve(listener, router).await?;

    let _ = std::fs::remove_file(&discovery);
    Ok(())
}

async fn persisted_wait_response(
    bridge: &Arc<dyn AppBridge>,
    session_id: &str,
    timeout_secs: u64,
) -> axum::response::Response {
    let started = std::time::Instant::now();
    loop {
        match bridge.review_session_status(session_id) {
            Ok(Some(state)) => {
                if let Some(verdict) = state.verdict {
                    return (
                        StatusCode::OK,
                        Json(serde_json::json!(ReviewWaitResponse { verdict })),
                    )
                        .into_response();
                }
            }
            Ok(None) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": "session not found"})),
                )
                    .into_response();
            }
            Err(error) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": error})),
                )
                    .into_response();
            }
        }

        if started.elapsed() >= std::time::Duration::from_secs(timeout_secs) {
            return (
                StatusCode::GATEWAY_TIMEOUT,
                Json(serde_json::json!({"error": "review timed out"})),
            )
                .into_response();
        }

        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct MockBridge {
        opened_files: std::sync::Mutex<Vec<(String, Option<u32>)>>,
        refreshed_files: std::sync::Mutex<Vec<String>>,
        annotations: std::sync::Mutex<HashMap<String, Vec<Annotation>>>,
        fail_open: AtomicBool,
    }

    impl MockBridge {
        fn new() -> Self {
            Self {
                opened_files: std::sync::Mutex::new(Vec::new()),
                refreshed_files: std::sync::Mutex::new(Vec::new()),
                annotations: std::sync::Mutex::new(HashMap::new()),
                fail_open: AtomicBool::new(false),
            }
        }
    }

    impl AppBridge for MockBridge {
        fn open_file(
            &self,
            file: &str,
            line: Option<u32>,
            _review_session_id: Option<&str>,
        ) -> Result<(), String> {
            if self.fail_open.load(Ordering::Relaxed) {
                return Err("open failed".to_string());
            }
            self.opened_files
                .lock()
                .unwrap()
                .push((file.to_string(), line));
            Ok(())
        }

        fn refresh_file(&self, file: &str) -> Result<(), String> {
            self.refreshed_files.lock().unwrap().push(file.to_string());
            Ok(())
        }

        fn get_annotations(&self, file: &str) -> Result<Vec<Annotation>, String> {
            let anns = self.annotations.lock().unwrap();
            Ok(anns.get(file).cloned().unwrap_or_default())
        }

        fn open_pr_review(
            &self,
            pr_ref: &str,
            _local_path_hint: Option<&str>,
        ) -> Result<String, String> {
            Ok(format!("/tmp/{}", pr_ref.replace('/', "-")))
        }

        fn start_review_session(&self, _session_id: &str, _file: &str) -> Result<(), String> {
            Ok(())
        }

        fn complete_review_session(&self, _session_id: &str, _verdict: &str) -> Result<(), String> {
            Ok(())
        }

        fn cancel_review_session(&self, _session_id: &str) -> Result<(), String> {
            Ok(())
        }

        fn timeout_review_session(&self, _session_id: &str) -> Result<(), String> {
            Ok(())
        }

        fn review_session_status(
            &self,
            _session_id: &str,
        ) -> Result<Option<ReviewSessionState>, String> {
            Ok(None)
        }
    }

    fn make_test_annotation(body: &str, line: u32) -> Annotation {
        Annotation::new(
            redpen_core::annotation::AnnotationKind::Comment,
            body.to_string(),
            vec![],
            "tester".to_string(),
            redpen_core::annotation::Anchor::TextContext {
                line_content: "fn main()".to_string(),
                surrounding_lines: vec![],
                content_hash: "abc".to_string(),
                range: redpen_core::annotation::Range {
                    start_line: line,
                    start_column: 0,
                    end_line: line,
                    end_column: 9,
                },
                last_known_line: line,
            },
        )
    }

    async fn spawn_test_server() -> (String, Arc<MockBridge>, Arc<ReviewSessions>) {
        spawn_test_server_with_bridge(Arc::new(MockBridge::new())).await
    }

    async fn spawn_test_server_with_bridge(
        bridge: Arc<MockBridge>,
    ) -> (String, Arc<MockBridge>, Arc<ReviewSessions>) {
        let sessions = Arc::new(ReviewSessions::new());
        let router = build_router(bridge.clone(), sessions.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        (format!("http://{}", addr), bridge, sessions)
    }

    fn post(client: &reqwest::Client, url: &str) -> reqwest::RequestBuilder {
        client.post(url)
    }

    // =======================================================================
    // rpc/open
    // =======================================================================

    #[tokio::test]
    async fn test_open_file() {
        let (base, bridge, _) = spawn_test_server().await;
        let client = reqwest::Client::new();

        let resp = post(&client, &format!("{}/rpc/open", base))
            .json(&serde_json::json!({"file": "/src/main.rs", "line": 42}))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["ok"], true);

        let opened = bridge.opened_files.lock().unwrap();
        assert_eq!(opened.len(), 1);
        assert_eq!(opened[0], ("/src/main.rs".to_string(), Some(42)));
    }

    #[tokio::test]
    async fn test_open_file_without_line() {
        let (base, bridge, _) = spawn_test_server().await;
        let client = reqwest::Client::new();

        let resp = post(&client, &format!("{}/rpc/open", base))
            .json(&serde_json::json!({"file": "/src/lib.rs"}))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
        let opened = bridge.opened_files.lock().unwrap();
        assert_eq!(opened[0], ("/src/lib.rs".to_string(), None));
    }

    #[tokio::test]
    async fn test_open_bridge_error_returns_500() {
        let bridge = Arc::new(MockBridge::new());
        bridge.fail_open.store(true, Ordering::Relaxed);
        let (base, _, _) = spawn_test_server_with_bridge(bridge).await;
        let client = reqwest::Client::new();

        let resp = post(&client, &format!("{}/rpc/open", base))
            .json(&serde_json::json!({"file": "/nope"}))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 500);
        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(body["error"].as_str().unwrap().contains("open failed"));
    }

    // =======================================================================
    // rpc/refresh
    // =======================================================================

    #[tokio::test]
    async fn test_refresh_file() {
        let (base, bridge, _) = spawn_test_server().await;
        let client = reqwest::Client::new();

        let resp = post(&client, &format!("{}/rpc/refresh", base))
            .json(&serde_json::json!({"file": "/src/main.rs"}))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
        let refreshed = bridge.refreshed_files.lock().unwrap();
        assert_eq!(refreshed[0], "/src/main.rs");
    }

    // =======================================================================
    // rpc/get_annotations
    // =======================================================================

    #[tokio::test]
    async fn test_get_annotations_empty() {
        let (base, _, _) = spawn_test_server().await;
        let client = reqwest::Client::new();

        let resp = post(&client, &format!("{}/rpc/get_annotations", base))
            .json(&serde_json::json!({"file": "/src/main.rs"}))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
        let body: Vec<serde_json::Value> = resp.json().await.unwrap();
        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn test_get_annotations_with_data() {
        let bridge = Arc::new(MockBridge::new());
        {
            let mut anns = bridge.annotations.lock().unwrap();
            anns.insert(
                "/src/main.rs".to_string(),
                vec![make_test_annotation("test note", 1)],
            );
        }

        let (base, _, _) = spawn_test_server_with_bridge(bridge).await;
        let client = reqwest::Client::new();

        let resp = post(&client, &format!("{}/rpc/get_annotations", base))
            .json(&serde_json::json!({"file": "/src/main.rs"}))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
        let body: Vec<serde_json::Value> = resp.json().await.unwrap();
        assert_eq!(body.len(), 1);
        assert_eq!(body[0]["body"], "test note");
    }

    // =======================================================================
    // rpc/review.start
    // =======================================================================

    #[tokio::test]
    async fn test_review_start_returns_session_id() {
        let (base, bridge, _) = spawn_test_server().await;
        let client = reqwest::Client::new();

        let resp = post(&client, &format!("{}/rpc/review.start", base))
            .json(&serde_json::json!({"file": "/src/main.rs", "line": 10}))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
        let body: ReviewStartResponse = resp.json().await.unwrap();
        assert!(!body.session_id.is_empty());

        // Should have opened the file
        let opened = bridge.opened_files.lock().unwrap();
        assert_eq!(opened[0], ("/src/main.rs".to_string(), Some(10)));
    }

    // =======================================================================
    // rpc/review.done
    // =======================================================================

    #[tokio::test]
    async fn test_review_done_with_valid_session() {
        let (base, _, sessions) = spawn_test_server().await;
        let client = reqwest::Client::new();

        let session_id = sessions.create("/src/main.rs".to_string()).await;
        let rx = sessions.take_receiver(&session_id).await.unwrap();

        let resp = post(&client, &format!("{}/rpc/review.done", base))
            .json(&serde_json::json!({
                "session_id": session_id,
                "verdict": "approved"
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
        let body: OkResponse = resp.json().await.unwrap();
        assert!(body.ok);
        assert_eq!(rx.await.unwrap(), "approved");
    }

    #[tokio::test]
    async fn test_review_done_unknown_session_returns_404() {
        let (base, _, _) = spawn_test_server().await;
        let client = reqwest::Client::new();

        let resp = post(&client, &format!("{}/rpc/review.done", base))
            .json(&serde_json::json!({
                "session_id": "nonexistent",
                "verdict": "approved"
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 404);
    }

    #[tokio::test]
    async fn test_review_done_defaults_to_approved() {
        let (base, _, sessions) = spawn_test_server().await;
        let client = reqwest::Client::new();

        let session_id = sessions.create("/src/main.rs".to_string()).await;
        let rx = sessions.take_receiver(&session_id).await.unwrap();

        let resp = post(&client, &format!("{}/rpc/review.done", base))
            .json(&serde_json::json!({"session_id": session_id}))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
        assert_eq!(rx.await.unwrap(), "approved");
    }

    // =======================================================================
    // rpc/review.wait
    // =======================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_review_wait_blocks_until_done() {
        let (base, _, sessions) = spawn_test_server().await;

        let session_id = sessions.create("/src/main.rs".to_string()).await;

        let base_clone = base.clone();
        let sid = session_id.clone();
        let wait_handle = tokio::spawn(async move {
            let c = reqwest::Client::new();
            c.post(format!("{}/rpc/review.wait", base_clone))
                .json(&serde_json::json!({"session_id": sid, "timeout": 5}))
                .send()
                .await
                .unwrap()
        });

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        sessions.complete(&session_id, "needs_work").await;

        let resp = wait_handle.await.unwrap();
        assert_eq!(resp.status(), 200);
        let body: ReviewWaitResponse = resp.json().await.unwrap();
        assert_eq!(body.verdict, "needs_work");
    }

    #[tokio::test]
    async fn test_review_wait_unknown_session_returns_404() {
        let (base, _, _) = spawn_test_server().await;
        let client = reqwest::Client::new();

        let resp = post(&client, &format!("{}/rpc/review.wait", base))
            .json(&serde_json::json!({"session_id": "nope", "timeout": 1}))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 404);
    }

    #[tokio::test]
    async fn test_review_wait_timeout() {
        let (base, _, sessions) = spawn_test_server().await;
        let client = reqwest::Client::new();

        let session_id = sessions.create("/src/main.rs".to_string()).await;

        let resp = post(&client, &format!("{}/rpc/review.wait", base))
            .json(&serde_json::json!({"session_id": session_id, "timeout": 1}))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 504);
    }

    // =======================================================================
    // rpc/review (combined open + wait)
    // =======================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_review_combined_flow() {
        let bridge = Arc::new(MockBridge::new());
        {
            let mut anns = bridge.annotations.lock().unwrap();
            anns.insert(
                "/src/main.rs".to_string(),
                vec![make_test_annotation("looks good", 1)],
            );
        }

        let (base, _, sessions) = spawn_test_server_with_bridge(bridge).await;

        let base_clone = base.clone();
        let review_handle = tokio::spawn(async move {
            let c = reqwest::Client::new();
            c.post(format!("{}/rpc/review", base_clone))
                .json(&serde_json::json!({
                    "file": "/src/main.rs",
                    "line": 10,
                    "timeout": 5
                }))
                .send()
                .await
                .unwrap()
        });

        // Poll until the session appears (the spawned request needs time to arrive)
        let session_id = loop {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            let senders = sessions.senders.lock().await;
            if let Some(id) = senders.keys().next() {
                break id.clone();
            }
        };
        sessions.complete(&session_id, "approved").await;

        let resp = review_handle.await.unwrap();
        assert_eq!(resp.status(), 200);
        let body: ReviewResponse = resp.json().await.unwrap();
        assert_eq!(body.verdict, "approved");
        assert_eq!(body.annotations.len(), 1);
        assert_eq!(body.annotations[0].body, "looks good");
    }

    #[tokio::test]
    async fn test_review_timeout() {
        let (base, _, _) = spawn_test_server().await;
        let client = reqwest::Client::new();

        let resp = post(&client, &format!("{}/rpc/review", base))
            .json(&serde_json::json!({
                "file": "/src/main.rs",
                "timeout": 1
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 504);
        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(body["error"].as_str().unwrap().contains("timed out"));
    }

    // =======================================================================
    // Concurrent sessions
    // =======================================================================

    #[tokio::test]
    async fn test_concurrent_review_sessions() {
        let sessions = ReviewSessions::new();

        let id1 = sessions.create("/file1.rs".to_string()).await;
        let id2 = sessions.create("/file2.rs".to_string()).await;
        assert_ne!(id1, id2);

        let rx1 = sessions.take_receiver(&id1).await.unwrap();
        let rx2 = sessions.take_receiver(&id2).await.unwrap();

        // Complete in reverse order
        sessions.complete(&id2, "needs_work").await;
        sessions.complete(&id1, "approved").await;

        assert_eq!(rx1.await.unwrap(), "approved");
        assert_eq!(rx2.await.unwrap(), "needs_work");
    }

    // =======================================================================
    // ReviewSessions unit tests
    // =======================================================================

    #[tokio::test]
    async fn test_session_create_and_complete() {
        let sessions = ReviewSessions::new();
        let id = sessions.create("test.rs".to_string()).await;
        assert!(!id.is_empty());

        let rx = sessions.take_receiver(&id).await.unwrap();
        let file = sessions.complete(&id, "approved").await;
        assert_eq!(file, Some("test.rs".to_string()));
        assert_eq!(rx.await.unwrap(), "approved");
    }

    #[tokio::test]
    async fn test_session_complete_nonexistent() {
        let sessions = ReviewSessions::new();
        assert!(sessions.complete("nope", "approved").await.is_none());
    }

    #[tokio::test]
    async fn test_session_double_complete() {
        let sessions = ReviewSessions::new();
        let id = sessions.create("test.rs".to_string()).await;
        assert!(sessions.complete(&id, "approved").await.is_some());
        assert!(sessions.complete(&id, "approved").await.is_none());
    }

    #[tokio::test]
    async fn test_take_receiver_twice_returns_none() {
        let sessions = ReviewSessions::new();
        let id = sessions.create("test.rs".to_string()).await;
        assert!(sessions.take_receiver(&id).await.is_some());
        assert!(sessions.take_receiver(&id).await.is_none());
    }

    // =======================================================================
    // session.annotations
    // =======================================================================

    #[tokio::test]
    async fn test_session_annotations_returns_files() {
        let bridge = Arc::new(MockBridge::new());
        bridge.annotations.lock().unwrap().insert(
            "/src/a.rs".to_string(),
            vec![make_test_annotation("fix this", 10)],
        );
        let (base, _, sessions) = spawn_test_server_with_bridge(bridge).await;
        let client = reqwest::Client::new();

        // Create a session with two files
        let session_id = sessions.create("session-test".to_string()).await;
        sessions
            .add_file(&session_id, "/src/a.rs".to_string())
            .await;
        sessions
            .add_file(&session_id, "/src/b.rs".to_string())
            .await;

        let resp = post(&client, &format!("{}/rpc/session.annotations", base))
            .json(&serde_json::json!({"session_id": session_id}))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = resp.json().await.unwrap();
        let arr = body.as_array().unwrap();
        assert_eq!(arr.len(), 3); // "session-test" + a.rs + b.rs
        let a_entry = arr.iter().find(|e| e["file"] == "/src/a.rs").unwrap();
        assert_eq!(a_entry["annotations"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_session_annotations_unknown_session_returns_404() {
        let (base, _, _) = spawn_test_server().await;
        let client = reqwest::Client::new();

        let resp = post(&client, &format!("{}/rpc/session.annotations", base))
            .json(&serde_json::json!({"session_id": "does-not-exist"}))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 404);
    }

    #[tokio::test]
    async fn test_open_with_session_tracks_file() {
        let (base, _, sessions) = spawn_test_server().await;
        let client = reqwest::Client::new();

        let session_id = sessions.create("primary.rs".to_string()).await;

        post(&client, &format!("{}/rpc/open", base))
            .json(&serde_json::json!({
                "file": "/src/extra.rs",
                "session_id": session_id
            }))
            .send()
            .await
            .unwrap();

        let files = sessions.get_files(&session_id).await;
        assert!(files.contains(&"/src/extra.rs".to_string()));
    }

    // =======================================================================
    // Discovery
    // =======================================================================

    #[test]
    fn test_discovery_path_is_under_home() {
        let path = discovery_path();
        assert!(path.to_string_lossy().contains(".config/redpen"));
        assert!(path.to_string_lossy().ends_with("server.json"));
    }
}

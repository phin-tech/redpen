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
    fn open_file(&self, file: &str, line: Option<u32>) -> Result<(), String>;
    /// Tell the GUI to refresh annotations for a file.
    fn refresh_file(&self, file: &str) -> Result<(), String>;
    /// Load annotations for a file from the sidecar store.
    fn get_annotations(&self, file: &str) -> Result<Vec<Annotation>, String>;
    /// Open a GitHub PR review and return the managed worktree path.
    fn open_pr_review(&self, pr_ref: &str, local_path_hint: Option<&str>) -> Result<String, String>;
}

// ---------------------------------------------------------------------------
// RPC request / response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct OpenRequest {
    pub file: String,
    pub line: Option<u32>,
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

// ---------------------------------------------------------------------------
// ReviewSessions — in-memory session tracking (replaces signal files)
// ---------------------------------------------------------------------------

pub struct ReviewSessions {
    senders: Mutex<HashMap<String, (String, oneshot::Sender<String>)>>,
    receivers: Mutex<HashMap<String, oneshot::Receiver<String>>>,
}

impl Default for ReviewSessions {
    fn default() -> Self {
        Self {
            senders: Mutex::new(HashMap::new()),
            receivers: Mutex::new(HashMap::new()),
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
            .insert(session_id.clone(), (file, tx));
        self.receivers.lock().await.insert(session_id.clone(), rx);
        session_id
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
    match state.bridge.open_file(&req.file, req.line) {
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
    let _ = state.bridge.open_file(&req.file, req.line);
    let session_id = state.sessions.create_with_id(req.session_id, req.file).await;
    (StatusCode::OK, Json(ReviewStartResponse { session_id }))
}

async fn rpc_review_done(
    AxumState(state): AxumState<ServerState>,
    Json(req): Json<ReviewDoneRequest>,
) -> impl IntoResponse {
    let verdict = req.verdict.as_deref().unwrap_or("approved");
    match state.sessions.complete(&req.session_id, verdict).await {
        Some(_file) => (StatusCode::OK, Json(OkResponse { ok: true })),
        None => (StatusCode::NOT_FOUND, Json(OkResponse { ok: false })),
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
                Ok(Err(_)) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "session cancelled"})),
                )
                    .into_response(),
                Err(_) => (
                    StatusCode::GATEWAY_TIMEOUT,
                    Json(serde_json::json!({"error": "review timed out"})),
                )
                    .into_response(),
            }
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "session not found or already consumed"})),
        )
            .into_response(),
    }
}

/// Combined open + wait. Blocks until the reviewer signals done.
async fn rpc_review(
    AxumState(state): AxumState<ServerState>,
    Json(req): Json<ReviewRequest>,
) -> impl IntoResponse {
    let _ = state.bridge.open_file(&req.file, req.line);

    let file = req.file.clone();
    let session_id = state.sessions.create_with_id(req.session_id, req.file).await;
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
        Ok(Err(_)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "session cancelled"})),
        )
            .into_response(),
        Err(_) => (
            StatusCode::GATEWAY_TIMEOUT,
            Json(serde_json::json!({"error": "review timed out"})),
        )
            .into_response(),
    }
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
        .route("/rpc/review.wait", post(rpc_review_wait))
        .route("/rpc/review", post(rpc_review))
        .route("/rpc/review.pr", post(rpc_review_pr))
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
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let sessions = Arc::new(ReviewSessions::new());
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
        fn open_file(&self, file: &str, line: Option<u32>) -> Result<(), String> {
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
    // Discovery
    // =======================================================================

    #[test]
    fn test_discovery_path_is_under_home() {
        let path = discovery_path();
        assert!(path.to_string_lossy().contains(".config/redpen"));
        assert!(path.to_string_lossy().ends_with("server.json"));
    }
}

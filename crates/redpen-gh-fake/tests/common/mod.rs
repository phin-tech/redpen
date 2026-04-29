//! Shared test helpers: spawn a seeded server (HTTP or HTTPS), kill it on drop.

#![allow(dead_code)]

use std::sync::Arc;

use axum_server::tls_rustls::RustlsConfig;
use redpen_gh_fake::{test_backend::TestBackend, ReviewComment, SessionRef};

pub struct SpawnedServer {
    pub base_url: String,
    pub backend: TestBackend,
    pub seeded_comment: ReviewComment,
    pub cert_pem: Option<String>,
    _shutdown: tokio::sync::oneshot::Sender<()>,
}

pub fn default_session() -> SessionRef {
    SessionRef {
        id: "session-1".into(),
        owner: "octocat".into(),
        repo: "hello".into(),
        number: 42,
        title: "Add greeting".into(),
        body: "Adds a hello message.".into(),
        head_ref: "feature/hello".into(),
        head_sha: "deadbeef".into(),
        base_ref: "main".into(),
        base_sha: "cafebabe".into(),
        author_login: "octocat".into(),
        viewer_login: "reviewer".into(),
        html_url: "redpen://session/session-1".into(),
    }
}

pub async fn spawn_http() -> SpawnedServer {
    let backend = TestBackend::new();
    let session = default_session();
    backend.add_session(session.clone());
    let seeded = backend.seed_comment(&session.id, "src/foo.rs", 10, "nit", "alice");

    let (addr, _handle) = redpen_gh_fake::bind(Arc::new(backend.clone()), 0)
        .await
        .expect("bind http");
    let (tx, _rx) = tokio::sync::oneshot::channel();
    SpawnedServer {
        base_url: format!("http://{}", addr),
        backend,
        seeded_comment: seeded,
        cert_pem: None,
        _shutdown: tx,
    }
}

/// Spawn an HTTPS server with a self-signed cert for `127.0.0.1`. Returns the
/// PEM-encoded cert so the test can pass it to clients via `SSL_CERT_FILE` etc.
pub async fn spawn_https() -> SpawnedServer {
    // rustls needs a default crypto provider installed once per process.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let backend = TestBackend::new();
    let session = default_session();
    backend.add_session(session.clone());
    let seeded = backend.seed_comment(&session.id, "src/foo.rs", 10, "nit", "alice");

    let cert = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into(), "localhost".into()])
        .expect("self-signed cert");
    let cert_pem = cert.cert.pem();
    let key_pem = cert.key_pair.serialize_pem();

    let config = RustlsConfig::from_pem(cert_pem.clone().into_bytes(), key_pem.into_bytes())
        .await
        .expect("rustls config");

    // Discover an ephemeral port via std listener, then bind axum-server.
    let std_listener = std::net::TcpListener::bind("127.0.0.1:0").expect("ephemeral");
    let addr = std_listener.local_addr().unwrap();
    drop(std_listener);

    let app = redpen_gh_fake::router(Arc::new(backend.clone()));
    let (tx, mut rx) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(async move {
        let server = axum_server::bind_rustls(addr, config).serve(app.into_make_service());
        tokio::select! {
            _ = server => {},
            _ = &mut rx => {},
        }
    });

    // Tiny settle so the listener is accepting before tests fire.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    SpawnedServer {
        base_url: format!("https://{}", addr),
        backend,
        seeded_comment: seeded,
        cert_pem: Some(cert_pem),
        _shutdown: tx,
    }
}

//! Spin up a server pre-seeded with a fixed session + comment.
//!
//! Used by cross-language client tests and for manual probing.
//!
//!     cargo run -p redpen-gh-fake --example serve_seeded -- 0
//!
//! Prints `LISTENING=127.0.0.1:<port>` to stdout, then serves indefinitely.

use std::sync::Arc;

use redpen_gh_fake::{test_backend::TestBackend, SessionRef};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let port: u16 = std::env::args()
        .nth(1)
        .as_deref()
        .unwrap_or("0")
        .parse()
        .expect("port");

    let backend = TestBackend::new();
    let session = SessionRef {
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
    };
    backend.add_session(session);
    if std::env::var("REDPEN_SKIP_SEED_COMMENT").is_err() {
        let seeded =
            backend.seed_comment("session-1", "src/foo.rs", 10, "nit: rename this", "alice");
        eprintln!("seeded comment database_id={}", seeded.database_id);
    } else {
        eprintln!("REDPEN_SKIP_SEED_COMMENT set; starting with no comments");
    }

    let (addr, handle) = redpen_gh_fake::bind(Arc::new(backend), port)
        .await
        .expect("bind");
    println!("LISTENING={}", addr);
    handle.await.unwrap();
}

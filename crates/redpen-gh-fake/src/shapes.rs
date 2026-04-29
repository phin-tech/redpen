//! JSON shape helpers — Redpen types → GitHub-API JSON.

use serde_json::{json, Value};

use crate::backend::{ReviewComment, SessionRef};

pub fn pr_json(s: &SessionRef) -> Value {
    json!({
        "number": s.number,
        "title": s.title,
        "body": s.body,
        "html_url": s.html_url,
        "state": "open",
        "head": {
            "ref": s.head_ref,
            "sha": s.head_sha,
            "label": format!("{}:{}", s.owner, s.head_ref),
            "user": { "login": s.author_login },
            "repo": { "full_name": format!("{}/{}", s.owner, s.repo) },
        },
        "base": {
            "ref": s.base_ref,
            "sha": s.base_sha,
            "label": format!("{}:{}", s.owner, s.base_ref),
            "repo": { "full_name": format!("{}/{}", s.owner, s.repo) },
        },
        "user": { "login": s.author_login },
        "draft": false,
    })
}

pub fn review_comment_json(c: &ReviewComment) -> Value {
    let mut obj = json!({
        "id": c.database_id,
        "node_id": c.node_id,
        "path": c.path,
        "line": c.line,
        "original_line": c.line,
        "diff_hunk": c.diff_hunk,
        "body": c.body,
        "user": { "login": c.author },
        "created_at": c.created_at.to_rfc3339(),
        "updated_at": c.updated_at.to_rfc3339(),
        "html_url": c.html_url,
        "side": "RIGHT",
    });
    if let Some(parent) = c.in_reply_to_database_id {
        obj["in_reply_to_id"] = json!(parent);
    }
    obj
}

//! Native PR review-comment workflow. Reads from either the local Redpen
//! gh-fake server or api.github.com, with the same filtering, classification,
//! and reply/resolve logic in both modes.
//!
//! This module replaces the `redpen review` wrapper that used to shell out
//! to `agent-reviews`. The display, bot detection, and meta-comment filters
//! mirror `pbakaus/agent-reviews@1.0.1` so the user-facing UX is the same;
//! the win is one tool that works for sandboxed (Redpen) and real GitHub
//! reviews without requiring Node or a separate package.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::server_client;

// ---------------------------------------------------------------------------
// Public CLI entry points
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct FetchOpts {
    pub remote: bool,
    pub pr: Option<u64>,
    pub repo: Option<String>,
    pub branch: Option<String>,
    pub unanswered: bool,
    pub unresolved: bool,
    pub bots_only: bool,
    pub humans_only: bool,
    pub expanded: bool,
    pub json: bool,
}

pub fn cmd_fetch(opts: FetchOpts) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = ResolvedContext::resolve(opts.remote, opts.pr, opts.repo.clone(), opts.branch.clone())?;
    let raw = fetch_pr_comments(&ctx)?;
    let processed = process_comments(&raw);
    let filtered = apply_filters(&processed, &opts);

    if opts.json {
        println!("{}", serde_json::to_string_pretty(&filtered)?);
    } else {
        render_table(&filtered, opts.expanded);
    }
    Ok(())
}

pub fn cmd_reply(
    id: i64,
    message: String,
    resolve: bool,
    remote: bool,
    pr: Option<u64>,
    repo: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = ResolvedContext::resolve(remote, pr, repo, None)?;

    let reply = post_reply(&ctx, id, &message)?;
    println!(
        "✓ Reply posted (id={}, {})",
        reply.id,
        reply.html_url.as_deref().unwrap_or("(no url)")
    );

    if resolve {
        match resolve_thread(&ctx, id)? {
            ResolveOutcome::Resolved { thread_id } => {
                println!("✓ Thread resolved ({})", thread_id);
            }
            ResolveOutcome::AlreadyResolved { thread_id } => {
                println!("· Thread already resolved ({})", thread_id);
            }
            ResolveOutcome::NotAReviewThread => {
                println!("· Skipped resolve: comment is not part of a review thread");
            }
        }
    }
    Ok(())
}

pub fn cmd_detail(
    id: i64,
    remote: bool,
    pr: Option<u64>,
    repo: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = ResolvedContext::resolve(remote, pr, repo, None)?;
    let raw = fetch_pr_comments(&ctx)?;
    let processed = process_comments(&raw);
    let target = processed
        .iter()
        .find(|c| c.id == id)
        .ok_or_else(|| format!("comment {} not found in PR #{}", id, ctx.pr))?;
    render_one_expanded(target);
    Ok(())
}

// ---------------------------------------------------------------------------
// Resolved context: where to send requests, what auth, what PR
// ---------------------------------------------------------------------------

struct ResolvedContext {
    base_url: String,
    token: String,
    owner: String,
    repo: String,
    pr: u64,
}

impl ResolvedContext {
    fn resolve(
        remote: bool,
        pr_arg: Option<u64>,
        repo_arg: Option<String>,
        branch_arg: Option<String>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // 1. Base URL
        let base_url = if remote {
            std::env::var("GITHUB_API_URL")
                .unwrap_or_else(|_| "https://api.github.com".to_string())
                .trim_end_matches('/')
                .to_string()
        } else {
            server_client::server_url().ok_or(
                "redpen server is not running. Start the desktop app, or use --remote to talk to api.github.com.",
            )?
        };

        // 2. Token. Local mode: anything goes (server is loopback). Remote:
        //    walk the same chain agent-reviews uses.
        let token = if remote {
            resolve_github_token().ok_or(
                "no GitHub token found. Set GITHUB_TOKEN or GH_TOKEN, or run `gh auth login`.",
            )?
        } else {
            std::env::var("GITHUB_TOKEN")
                .or_else(|_| std::env::var("GH_TOKEN"))
                .unwrap_or_else(|_| "redpen-agent".to_string())
        };

        // 3. owner/repo + PR. In local mode the active Redpen session is the
        //    authoritative default — query it once and use it for both.
        let local_solo = if !remote {
            solo_session_identity(&base_url)
        } else {
            None
        };

        let (owner, repo) = match repo_arg.or_else(|| std::env::var("GH_REPO").ok()) {
            Some(s) => parse_owner_repo(&s).ok_or_else(|| format!("invalid repo: {}", s))?,
            None => match &local_solo {
                Some((o, r, _, _)) => (o.clone(), r.clone()),
                None => detect_repo_from_git().ok_or(
                    "couldn't determine owner/repo. Pass --repo owner/name or set GH_REPO.",
                )?,
            },
        };

        // 4. PR number
        let pr = match pr_arg {
            Some(n) => n,
            None => match &local_solo {
                Some((_, _, n, _)) => *n,
                None => {
                    let branch = match branch_arg {
                        Some(b) => b,
                        None => detect_current_branch().ok_or(
                            "couldn't determine branch. Pass --pr <n> or run inside a git repo.",
                        )?,
                    };
                    find_pr_for_branch(&base_url, &token, &owner, &repo, &branch)?.ok_or_else(
                        || format!("no open PR found for {}/{} branch {}", owner, repo, branch),
                    )?
                }
            },
        };

        Ok(Self {
            base_url,
            token,
            owner,
            repo,
            pr,
        })
    }
}

/// In local mode, peek at `/redpen/meta` and return (owner, repo, pr, branch)
/// iff there's exactly one active session. Returns None on any error or on
/// zero/multiple sessions; caller falls back to git-remote detection.
fn solo_session_identity(base_url: &str) -> Option<(String, String, u64, String)> {
    let body: Value = ureq::get(&format!("{}/redpen/meta", base_url))
        .call()
        .ok()?
        .into_body()
        .read_json()
        .ok()?;
    let sessions = body["active_sessions"].as_array()?;
    if sessions.len() != 1 {
        return None;
    }
    let s = &sessions[0];
    Some((
        s["owner"].as_str()?.to_string(),
        s["repo"].as_str()?.to_string(),
        s["number"].as_u64()?,
        s["branch"].as_str().unwrap_or("").to_string(),
    ))
}

fn parse_owner_repo(s: &str) -> Option<(String, String)> {
    s.split_once('/')
        .map(|(o, r)| (o.to_string(), r.trim_end_matches(".git").to_string()))
}

fn detect_repo_from_git() -> Option<(String, String)> {
    let out = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
    // Strip git@github.com: or https://github.com/ prefixes
    let path = url
        .strip_prefix("git@github.com:")
        .or_else(|| url.strip_prefix("https://github.com/"))
        .or_else(|| url.strip_prefix("ssh://git@github.com/"))?;
    parse_owner_repo(path)
}

fn detect_current_branch() -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn resolve_github_token() -> Option<String> {
    if let Ok(t) = std::env::var("GITHUB_TOKEN") {
        if !t.is_empty() {
            return Some(t);
        }
    }
    if let Ok(t) = std::env::var("GH_TOKEN") {
        if !t.is_empty() {
            return Some(t);
        }
    }
    if let Ok(content) = std::fs::read_to_string(".env.local") {
        for line in content.lines() {
            if let Some(rest) = line.trim().strip_prefix("GITHUB_TOKEN=") {
                let v = rest.trim_matches(|c: char| c == '"' || c == '\'').trim();
                if !v.is_empty() {
                    return Some(v.to_string());
                }
            }
        }
    }
    let out = std::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
        .ok()?;
    if out.status.success() {
        let t = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !t.is_empty() {
            return Some(t);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// HTTP — fetching
// ---------------------------------------------------------------------------

#[derive(Default)]
struct RawComments {
    review_comments: Vec<Value>,
    issue_comments: Vec<Value>,
    reviews: Vec<Value>,
}

fn fetch_pr_comments(ctx: &ResolvedContext) -> Result<RawComments, Box<dyn std::error::Error>> {
    let base = format!(
        "{}/repos/{}/{}/pulls/{}",
        ctx.base_url, ctx.owner, ctx.repo, ctx.pr
    );
    let issue_base = format!(
        "{}/repos/{}/{}/issues/{}",
        ctx.base_url, ctx.owner, ctx.repo, ctx.pr
    );

    let review_comments = fetch_paginated(&format!("{}/comments?per_page=100", base), &ctx.token)?;
    let issue_comments =
        fetch_paginated(&format!("{}/comments?per_page=100", issue_base), &ctx.token)?;
    let reviews = fetch_paginated(&format!("{}/reviews?per_page=100", base), &ctx.token)?;

    Ok(RawComments {
        review_comments,
        issue_comments,
        reviews,
    })
}

fn fetch_paginated(url: &str, token: &str) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let mut results = Vec::new();
    let mut next = Some(url.to_string());
    while let Some(u) = next {
        let resp = ureq::get(&u)
            .header("Authorization", &format!("Bearer {}", token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "redpen-cli")
            .call()?;
        let link = resp
            .headers()
            .get("link")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let body: Value = resp.into_body().read_json()?;
        if let Some(arr) = body.as_array() {
            results.extend(arr.iter().cloned());
        }
        next = link.and_then(|h| extract_next_link(&h));
    }
    Ok(results)
}

fn extract_next_link(link_header: &str) -> Option<String> {
    // Format: <url>; rel="next", <url>; rel="last"
    for part in link_header.split(',') {
        let part = part.trim();
        if !part.contains(r#"rel="next""#) {
            continue;
        }
        if let (Some(start), Some(end)) = (part.find('<'), part.find('>')) {
            return Some(part[start + 1..end].to_string());
        }
    }
    None
}

fn find_pr_for_branch(
    base_url: &str,
    token: &str,
    owner: &str,
    repo: &str,
    branch: &str,
) -> Result<Option<u64>, Box<dyn std::error::Error>> {
    let url = format!(
        "{}/repos/{}/{}/pulls?head={}:{}&state=open",
        base_url, owner, repo, owner, branch
    );
    let body: Value = ureq::get(&url)
        .header("Authorization", &format!("Bearer {}", token))
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "redpen-cli")
        .call()?
        .into_body()
        .read_json()?;
    Ok(body
        .as_array()
        .and_then(|a| a.first())
        .and_then(|p| p["number"].as_u64()))
}

// ---------------------------------------------------------------------------
// Bot detection + meta-comment filtering — port of agent-reviews logic
// ---------------------------------------------------------------------------

const KNOWN_BOT_LOGINS: &[&str] = &[
    "cursor",
    "vercel",
    "supabase",
    "chatgpt-codex-connector",
    "github-actions",
    "Copilot",
    "copilot-pull-request-reviewer",
    "coderabbitai",
    "sourcery-ai",
    "codacy-production",
    "sonarcloud",
    "sonarqubecloud",
    "sonarqube-cloud-us",
];

fn is_bot(login: &str) -> bool {
    if login.is_empty() {
        return false;
    }
    if login.ends_with("[bot]") {
        return true;
    }
    if KNOWN_BOT_LOGINS.contains(&login) {
        return true;
    }
    login.contains("bot")
}

/// Each filter takes (login_lowercase_or_with_bot_suffix, body) and returns
/// true if the comment is a meta-status update we should hide from the
/// actionable list.
fn is_meta_comment(login: &str, body: &str) -> bool {
    if body.starts_with("> Re: comment ") {
        return true;
    }
    let l = login;
    macro_rules! login_eq {
        ($name:expr) => {
            l == concat!($name, "[bot]") || l == $name
        };
    }
    if login_eq!("vercel") && body.starts_with("[vc]:") {
        return true;
    }
    if login_eq!("supabase") && body.starts_with("[supa]:") {
        return true;
    }
    if login_eq!("cursor") && body.starts_with("Cursor Bugbot has reviewed your changes") {
        return true;
    }
    if login_eq!("copilot-pull-request-reviewer") && body.contains("Pull request overview") {
        return true;
    }
    if login_eq!("coderabbitai")
        && body.contains("<!-- This is an auto-generated comment: summarize by coderabbit.ai -->")
    {
        return true;
    }
    if login_eq!("sourcery-ai") && body.contains("<!-- Generated by sourcery-ai[bot]:") {
        return true;
    }
    if login_eq!("codacy-production")
        && (body.contains("Codacy's Analysis Summary")
            || body.contains("Coverage summary from Codacy"))
    {
        return true;
    }
    let sonar = login_eq!("sonarcloud") || login_eq!("sonarqubecloud") || login_eq!("sonarqube-cloud-us");
    if sonar && body.contains("Quality Gate") {
        return true;
    }
    false
}

fn clean_body(body: &str) -> String {
    let mut s = body.to_string();
    // Strip <!-- ... --> HTML comments (single + multi-line).
    s = strip_pattern(&s, "<!--", "-->");
    // Strip <details>...</details> blocks containing "Additional Locations".
    while let Some(pos) = s.find("<details>") {
        // Find matching </details> after this <details>.
        if let Some(end_rel) = s[pos..].find("</details>") {
            let end = pos + end_rel + "</details>".len();
            let block = &s[pos..end];
            if block
                .to_ascii_lowercase()
                .contains("additional locations")
            {
                s = format!("{}{}", &s[..pos], &s[end..]);
                continue;
            }
        }
        break;
    }
    // Strip <p>...cursor.com...</p> blocks.
    while let Some(pos) = s.find("<p>") {
        if let Some(end_rel) = s[pos..].find("</p>") {
            let end = pos + end_rel + "</p>".len();
            let block = &s[pos..end];
            if block.contains("cursor.com") {
                s = format!("{}{}", &s[..pos], &s[end..]);
                continue;
            }
        }
        break;
    }
    // Collapse 3+ newlines to 2.
    while s.contains("\n\n\n") {
        s = s.replace("\n\n\n", "\n\n");
    }
    s.trim().to_string()
}

fn strip_pattern(input: &str, open: &str, close: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;
    let bytes = input.as_bytes();
    while i < bytes.len() {
        if input[i..].starts_with(open) {
            if let Some(end_rel) = input[i + open.len()..].find(close) {
                i += open.len() + end_rel + close.len();
                continue;
            }
        }
        // Push next char (handle multi-byte safely).
        let ch_end = next_char_end(input, i);
        out.push_str(&input[i..ch_end]);
        i = ch_end;
    }
    out
}

fn next_char_end(s: &str, i: usize) -> usize {
    let bytes = s.as_bytes();
    let b = bytes[i];
    let len = if b < 0x80 {
        1
    } else if b < 0xC0 {
        1
    } else if b < 0xE0 {
        2
    } else if b < 0xF0 {
        3
    } else {
        4
    };
    (i + len).min(bytes.len())
}

// ---------------------------------------------------------------------------
// Processed comment shape (matches agent-reviews's unified format)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reply {
    pub id: i64,
    pub user: String,
    pub body: String,
    pub created_at: Option<String>,
    pub is_bot: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedComment {
    pub id: i64,
    #[serde(rename = "type")]
    pub kind: String, // "review_comment" | "issue_comment" | "review"
    pub user: String,
    pub is_bot: bool,
    pub path: Option<String>,
    pub line: Option<u32>,
    pub diff_hunk: Option<String>,
    pub body: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub url: Option<String>,
    pub replies: Vec<Reply>,
    pub has_human_reply: bool,
    pub has_any_reply: bool,
    pub is_resolved: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

fn process_comments(raw: &RawComments) -> Vec<ProcessedComment> {
    // Build replies map: comment.in_reply_to_id → [reply]
    let mut replies_map: HashMap<i64, Vec<Reply>> = HashMap::new();
    for c in &raw.review_comments {
        if let Some(parent) = c["in_reply_to_id"].as_i64() {
            let user = c["user"]["login"].as_str().unwrap_or("").to_string();
            let reply = Reply {
                id: c["id"].as_i64().unwrap_or(0),
                is_bot: is_bot(&user),
                user,
                body: clean_body(c["body"].as_str().unwrap_or("")),
                created_at: c["created_at"].as_str().map(str::to_string),
            };
            replies_map.entry(parent).or_default().push(reply);
        }
    }

    let mut out = Vec::new();

    // Inline review comments (top-level)
    for c in &raw.review_comments {
        if c["in_reply_to_id"].as_i64().is_some() {
            continue;
        }
        let user = c["user"]["login"].as_str().unwrap_or("").to_string();
        let body = c["body"].as_str().unwrap_or("");
        if is_meta_comment(&user, body) {
            continue;
        }
        let id = c["id"].as_i64().unwrap_or(0);
        let replies = replies_map.remove(&id).unwrap_or_default();
        let has_human_reply = replies.iter().any(|r| !r.is_bot);
        let has_any_reply = !replies.is_empty();
        out.push(ProcessedComment {
            id,
            kind: "review_comment".into(),
            user: user.clone(),
            is_bot: is_bot(&user),
            path: c["path"].as_str().map(str::to_string),
            line: c["line"]
                .as_u64()
                .or_else(|| c["original_line"].as_u64())
                .map(|n| n as u32),
            diff_hunk: c["diff_hunk"].as_str().map(str::to_string),
            body: clean_body(body),
            created_at: c["created_at"].as_str().map(str::to_string),
            updated_at: c["updated_at"].as_str().map(str::to_string),
            url: c["html_url"].as_str().map(str::to_string),
            replies,
            has_human_reply,
            has_any_reply,
            is_resolved: false,
            state: None,
        });
    }

    // PR-level issue comments
    for c in &raw.issue_comments {
        let user = c["user"]["login"].as_str().unwrap_or("").to_string();
        let body = c["body"].as_str().unwrap_or("");
        if is_meta_comment(&user, body) {
            continue;
        }
        out.push(ProcessedComment {
            id: c["id"].as_i64().unwrap_or(0),
            kind: "issue_comment".into(),
            user: user.clone(),
            is_bot: is_bot(&user),
            path: None,
            line: None,
            diff_hunk: None,
            body: clean_body(body),
            created_at: c["created_at"].as_str().map(str::to_string),
            updated_at: c["updated_at"].as_str().map(str::to_string),
            url: c["html_url"].as_str().map(str::to_string),
            replies: vec![],
            has_human_reply: false,
            has_any_reply: false,
            is_resolved: false,
            state: None,
        });
    }

    // Review summary bodies (humans only — bot review bodies are summaries
    // and individual findings already arrived as review_comments)
    for r in &raw.reviews {
        let user = r["user"]["login"].as_str().unwrap_or("").to_string();
        if is_bot(&user) {
            continue;
        }
        let body = r["body"].as_str().unwrap_or("");
        if body.trim().is_empty() {
            continue;
        }
        if is_meta_comment(&user, body) {
            continue;
        }
        let state = r["state"].as_str().unwrap_or("").to_string();
        let resolved = state == "APPROVED" || state == "DISMISSED";
        out.push(ProcessedComment {
            id: r["id"].as_i64().unwrap_or(0),
            kind: "review".into(),
            user: user.clone(),
            is_bot: false,
            path: None,
            line: None,
            diff_hunk: None,
            body: clean_body(body),
            created_at: r["submitted_at"].as_str().map(str::to_string),
            updated_at: r["submitted_at"].as_str().map(str::to_string),
            url: r["html_url"].as_str().map(str::to_string),
            replies: vec![],
            has_human_reply: false,
            has_any_reply: false,
            is_resolved: resolved,
            state: Some(state),
        });
    }

    // Newest first
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    out
}

fn apply_filters(comments: &[ProcessedComment], opts: &FetchOpts) -> Vec<ProcessedComment> {
    comments
        .iter()
        .filter(|c| !(opts.bots_only && !c.is_bot))
        .filter(|c| !(opts.humans_only && c.is_bot))
        .filter(|c| !(opts.unresolved && (c.is_resolved || c.has_human_reply)))
        .filter(|c| !(opts.unanswered && c.has_any_reply))
        .cloned()
        .collect()
}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------

fn render_table(comments: &[ProcessedComment], expanded: bool) {
    println!("Found {} comments", comments.len());
    if comments.is_empty() {
        return;
    }
    println!();
    for c in comments {
        let status = if c.is_resolved {
            "✓ resolved"
        } else if c.has_human_reply {
            "↩ human reply"
        } else if c.has_any_reply {
            "↩ replied"
        } else {
            "○ no reply"
        };
        let kind_label = match c.kind.as_str() {
            "review_comment" => "CODE",
            "issue_comment" => "ISSUE",
            "review" => "REVIEW",
            _ => "?",
        };
        println!(
            "[{}] {} by {} {}",
            c.id, kind_label, c.user, status
        );
        if let (Some(p), Some(l)) = (&c.path, c.line) {
            println!("  {}:{}", p, l);
        }
        if let Some(s) = &c.state {
            println!("  state: {}", s);
        }
        if expanded {
            if let Some(diff) = &c.diff_hunk {
                println!("  --- diff ---");
                for line in diff.lines() {
                    println!("  | {}", line);
                }
            }
            println!("  --- body ---");
            for line in c.body.lines() {
                println!("  | {}", line);
            }
            if !c.replies.is_empty() {
                println!("  --- replies ({}) ---", c.replies.len());
                for r in &c.replies {
                    println!("  └ [{}] {}: {}", r.id, r.user, truncate_body(&r.body, 200));
                }
            }
        } else {
            let body_preview = truncate_body(&c.body, 110);
            if !body_preview.is_empty() {
                println!("  {}", body_preview);
            }
            if !c.replies.is_empty() {
                println!("  └ {} reply(ies)", c.replies.len());
            }
        }
        println!();
    }
}

fn render_one_expanded(c: &ProcessedComment) {
    render_table(std::slice::from_ref(c), true);
}

fn truncate_body(body: &str, max: usize) -> String {
    let single = body.replace('\n', " ");
    if single.chars().count() <= max {
        single
    } else {
        let truncated: String = single.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", truncated)
    }
}

// ---------------------------------------------------------------------------
// Reply + resolve
// ---------------------------------------------------------------------------

struct PostedReply {
    id: i64,
    html_url: Option<String>,
}

fn post_reply(
    ctx: &ResolvedContext,
    parent_id: i64,
    message: &str,
) -> Result<PostedReply, Box<dyn std::error::Error>> {
    let url = format!(
        "{}/repos/{}/{}/pulls/{}/comments/{}/replies",
        ctx.base_url, ctx.owner, ctx.repo, ctx.pr, parent_id
    );
    let body = json!({ "body": message });
    let resp = ureq::post(&url)
        .header("Authorization", &format!("Bearer {}", ctx.token))
        .header("Content-Type", "application/json")
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "redpen-cli")
        .send_json(&body);

    match resp {
        Ok(r) => {
            let j: Value = r.into_body().read_json()?;
            return Ok(PostedReply {
                id: j["id"].as_i64().unwrap_or(0),
                html_url: j["html_url"].as_str().map(str::to_string),
            });
        }
        Err(ureq::Error::StatusCode(404)) => {
            // Fallback: post as issue comment with a "> Re:" prefix
            // (mirrors agent-reviews's behavior when the reply endpoint 404s
            // for non-thread comments).
            let issue_url = format!(
                "{}/repos/{}/{}/issues/{}/comments",
                ctx.base_url, ctx.owner, ctx.repo, ctx.pr
            );
            let body = json!({
                "body": format!("> Re: comment {}\n\n{}", parent_id, message)
            });
            let r = ureq::post(&issue_url)
                .header("Authorization", &format!("Bearer {}", ctx.token))
                .header("Content-Type", "application/json")
                .header("Accept", "application/vnd.github.v3+json")
                .header("User-Agent", "redpen-cli")
                .send_json(&body)?;
            let j: Value = r.into_body().read_json()?;
            Ok(PostedReply {
                id: j["id"].as_i64().unwrap_or(0),
                html_url: j["html_url"].as_str().map(str::to_string),
            })
        }
        Err(e) => Err(e.into()),
    }
}

enum ResolveOutcome {
    Resolved { thread_id: String },
    AlreadyResolved { thread_id: String },
    NotAReviewThread,
}

fn graphql_url(base_url: &str) -> String {
    // Mirror the agent-reviews logic: GHES splits REST (/api/v3) from
    // GraphQL (/api/graphql); everything else gets /graphql appended.
    if let Some(stripped) = base_url.strip_suffix("/api/v3") {
        format!("{}/api/graphql", stripped)
    } else {
        format!("{}/graphql", base_url)
    }
}

fn resolve_thread(
    ctx: &ResolvedContext,
    comment_id: i64,
) -> Result<ResolveOutcome, Box<dyn std::error::Error>> {
    let gql = graphql_url(&ctx.base_url);

    // Page through reviewThreads looking for one that contains our comment.
    let query = r#"
        query($owner:String!,$repo:String!,$pr:Int!,$cursor:String) {
          repository(owner:$owner,name:$repo) {
            pullRequest(number:$pr) {
              reviewThreads(first:100, after:$cursor) {
                pageInfo { hasNextPage endCursor }
                nodes {
                  id
                  isResolved
                  comments(first:1) { nodes { databaseId } }
                }
              }
            }
          }
        }
    "#;

    let mut cursor: Option<String> = None;
    loop {
        let body = json!({
            "query": query,
            "variables": {
                "owner": ctx.owner, "repo": ctx.repo, "pr": ctx.pr, "cursor": cursor,
            }
        });
        let resp: Value = ureq::post(&gql)
            .header("Authorization", &format!("Bearer {}", ctx.token))
            .header("Content-Type", "application/json")
            .header("User-Agent", "redpen-cli")
            .send_json(&body)?
            .into_body()
            .read_json()?;
        if let Some(errs) = resp.get("errors") {
            return Err(format!("graphql error: {}", errs).into());
        }
        let threads = &resp["data"]["repository"]["pullRequest"]["reviewThreads"];
        let nodes = threads["nodes"].as_array().cloned().unwrap_or_default();
        for t in &nodes {
            let db_ids: Vec<i64> = t["comments"]["nodes"]
                .as_array()
                .map(|a| a.iter().filter_map(|c| c["databaseId"].as_i64()).collect())
                .unwrap_or_default();
            if db_ids.contains(&comment_id) {
                let thread_id = t["id"].as_str().unwrap_or("").to_string();
                if t["isResolved"].as_bool().unwrap_or(false) {
                    return Ok(ResolveOutcome::AlreadyResolved { thread_id });
                }
                return resolve_one_thread(&gql, &ctx.token, &thread_id);
            }
        }
        let page = &threads["pageInfo"];
        if page["hasNextPage"].as_bool().unwrap_or(false) {
            cursor = page["endCursor"].as_str().map(str::to_string);
        } else {
            return Ok(ResolveOutcome::NotAReviewThread);
        }
    }
}

fn resolve_one_thread(
    gql_url: &str,
    token: &str,
    thread_id: &str,
) -> Result<ResolveOutcome, Box<dyn std::error::Error>> {
    let mutation = r#"
        mutation($threadId: ID!) {
          resolveReviewThread(input:{threadId:$threadId}) {
            thread { id isResolved }
          }
        }
    "#;
    let body = json!({
        "query": mutation,
        "variables": { "threadId": thread_id }
    });
    let resp: Value = ureq::post(gql_url)
        .header("Authorization", &format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .header("User-Agent", "redpen-cli")
        .send_json(&body)?
        .into_body()
        .read_json()?;
    if let Some(errs) = resp.get("errors") {
        return Err(format!("graphql resolve error: {}", errs).into());
    }
    if resp["data"]["resolveReviewThread"]["thread"]["isResolved"]
        .as_bool()
        .unwrap_or(false)
    {
        Ok(ResolveOutcome::Resolved {
            thread_id: thread_id.to_string(),
        })
    } else {
        // Server didn't return isResolved=true; treat as not-resolved error
        Err("resolveReviewThread did not return isResolved=true".into())
    }
}


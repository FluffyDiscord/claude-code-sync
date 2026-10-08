//! Discovery reads a transcript again only when its length or mtime changed.

mod common;

use std::fs;
use std::path::Path;

use claude_code_sync::filter::FilterConfig;
use claude_code_sync::parser::ConversationSession;
use claude_code_sync::sync::discovery::discover_sessions;
use common::ConfigEnv;
use serial_test::serial;
use tempfile::TempDir;

const FIRST_LINE: &str =
    r#"{"type":"user","uuid":"a","timestamp":"2025-01-01T00:00:00Z","message":{"text":"hello"}}"#;
const SECOND_LINE: &str =
    r#"{"type":"assistant","uuid":"b","timestamp":"2025-01-01T00:01:00Z","message":{"text":"hi"}}"#;

fn summarize_only_transcript(projects_dir: &Path) -> ConversationSession {
    let mut sessions = discover_sessions(projects_dir, &FilterConfig::default()).unwrap();
    assert_eq!(sessions.len(), 1);
    sessions.remove(0)
}

#[test]
#[serial]
fn an_unchanged_transcript_is_not_read_again_and_a_changed_one_is() {
    let _env = ConfigEnv::new();
    let projects_dir = TempDir::new().unwrap();
    let project_dir = projects_dir.path().join("-home-user-shop");
    fs::create_dir(&project_dir).unwrap();
    let transcript = project_dir.join("session.jsonl");
    fs::write(&transcript, format!("{FIRST_LINE}\n")).unwrap();

    let first = summarize_only_transcript(projects_dir.path());

    let modified = fs::metadata(&transcript).unwrap().modified().unwrap();
    let same_length_line = FIRST_LINE.replace("hello", "HELLO");
    fs::write(&transcript, format!("{same_length_line}\n")).unwrap();
    let file = fs::File::options().write(true).open(&transcript).unwrap();
    file.set_modified(modified).unwrap();
    drop(file);

    let unchanged = summarize_only_transcript(projects_dir.path());
    assert_eq!(unchanged.content_hash(), first.content_hash());

    fs::write(&transcript, format!("{FIRST_LINE}\n{SECOND_LINE}\n")).unwrap();

    let changed = summarize_only_transcript(projects_dir.path());
    assert_eq!(changed.message_count(), first.message_count() + 1);
}

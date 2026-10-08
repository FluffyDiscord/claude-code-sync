//! Machine-neutral paths: this machine's absolute locations are stored in the
//! sync repository as tokens and rendered back on the way out, so a hook
//! command or a status line still resolves on a machine whose home directory
//! or `~/.claude` location differs. Any absolute path in the file is
//! neutralized, not specific keys.
//!
//! One type owns both directions: whatever `to_machine` renders, `to_repo`
//! must undo, or two machines rewrite each other's values on every sync.

use std::path::Path;

/// Stands for this machine's `~`.
pub const HOME_TOKEN: &str = "__HOME__";
/// Stands for this machine's `~/.claude`. Tokenized before the home directory,
/// since it is configurable and may sit outside it.
pub const CLAUDE_DIR_TOKEN: &str = "__CLAUDE_DIR__";

/// This machine's absolute locations and their neutral spellings.
///
/// Locations are rendered with `/` separators on every OS: `C:/Users/me` is
/// valid inside a JSON string, resolves under Windows shells and Git Bash alike,
/// and joins cleanly with a tail stored as `__HOME__/bin/x`. A raw Windows
/// `C:\Users\me` would write `\U`, an invalid JSON escape.
///
/// The repo side recognizes both spellings a Windows file may hold —
/// `C:\\Users\\me` (escaped backslashes) and `C:/Users/me` — and stores the
/// path tail with `/`, so a path written on Windows still resolves on Linux.
#[derive(Debug, Clone, Default)]
pub struct PathTokens {
    home: Location,
    claude_dir: Location,
}

/// One absolute location: how it is rendered, and every spelling tokenized.
#[derive(Debug, Clone, Default)]
struct Location {
    rendered: String,
    spellings: Vec<String>,
}

impl Location {
    fn new(native: &str) -> Self {
        let rendered = native.replace('\\', "/");
        let mut spellings = vec![rendered.clone()];
        if native.contains('\\') {
            spellings.push(rendered.replace('/', r"\\"));
        }
        spellings.retain(|spelling| !spelling.is_empty());
        Location {
            rendered,
            spellings,
        }
    }
}

impl PathTokens {
    /// Tokens for the machine whose Claude directory is `claude_dir`.
    pub fn for_claude_dir(claude_dir: &Path) -> Self {
        let home = dirs::home_dir()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default();
        Self::for_paths(&home, &claude_dir.to_string_lossy())
    }

    /// Tokens for these two absolute locations, as written on this machine.
    fn for_paths(home: &str, claude_dir: &str) -> Self {
        PathTokens {
            home: Location::new(home),
            claude_dir: Location::new(claude_dir),
        }
    }

    /// Machine bytes -> repo bytes. Non-UTF-8 content passes through unchanged.
    pub fn to_repo(&self, bytes: &[u8]) -> Vec<u8> {
        let Ok(text) = std::str::from_utf8(bytes) else {
            return bytes.to_vec();
        };
        let mut text = text.to_string();
        for (location, token) in [
            (&self.claude_dir, CLAUDE_DIR_TOKEN),
            (&self.home, HOME_TOKEN),
        ] {
            for spelling in &location.spellings {
                text = replace_path(&text, spelling, token);
            }
        }
        text.into_bytes()
    }

    /// Repo bytes -> machine bytes. Non-UTF-8 content passes through unchanged.
    pub fn to_machine(&self, bytes: &[u8]) -> Vec<u8> {
        let Ok(text) = std::str::from_utf8(bytes) else {
            return bytes.to_vec();
        };
        let text = text.replace(CLAUDE_DIR_TOKEN, &self.claude_dir.rendered);
        let text = text.replace(HOME_TOKEN, &self.home.rendered);
        text.into_bytes()
    }
}

/// Replace `needle` with `token` only where the match is a whole path prefix:
/// `/home/user` must not eat the `/home/username` of a machine that is not this one,
/// nor the tail of an unrelated `/opt/home/user`. The tail after a replaced
/// prefix has its escaped backslash separators turned into `/`.
fn replace_path(text: &str, needle: &str, token: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(needle) {
        let before = &rest[..start];
        let after = &rest[start + needle.len()..];
        out.push_str(before);
        if continues_a_segment(before.chars().next_back())
            || continues_a_segment(after.chars().next())
        {
            out.push_str(needle);
            rest = after;
        } else {
            out.push_str(token);
            rest = push_path_tail(&mut out, after);
        }
    }
    out.push_str(rest);
    out
}

/// Copy the path continuing at the start of `text` into `out`, spelling each
/// escaped backslash separator (`\\` in the JSON text) as `/`. Returns what
/// follows the path.
fn push_path_tail<'a>(out: &mut String, text: &'a str) -> &'a str {
    let mut rest = text;
    loop {
        if let Some(after) = rest.strip_prefix(r"\\") {
            out.push('/');
            rest = after;
            continue;
        }
        let Some(character) = rest.chars().next() else {
            return rest;
        };
        if !(continues_a_segment(Some(character)) || matches!(character, '/' | '+' | '@' | '%')) {
            return rest;
        }
        out.push(character);
        rest = &rest[character.len_utf8()..];
    }
}

/// Whether a character next to a match makes it part of a longer path segment,
/// which is what tells `/home/username` and `/opt/home/user` from `/home/user` itself.
fn continues_a_segment(neighbour: Option<char>) -> bool {
    match neighbour {
        Some(character) => {
            character.is_alphanumeric() || matches!(character, '-' | '_' | '.' | '~')
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn windows() -> PathTokens {
        PathTokens::for_paths(r"C:\Users\me", r"C:\Users\me\.claude")
    }

    #[test]
    fn a_windows_backslash_path_is_stored_with_forward_slashes() {
        let live = br#"{"statusLine":{"command":"C:\\Users\\me\\.claude\\s.ps1"},"x":"C:\\Users\\me\\bin\\y"}"#;
        let stored = String::from_utf8(windows().to_repo(live)).unwrap();
        assert_eq!(
            stored,
            r#"{"statusLine":{"command":"__CLAUDE_DIR__/s.ps1"},"x":"__HOME__/bin/y"}"#
        );
    }

    #[test]
    fn a_windows_forward_slash_path_is_tokenized() {
        let live = br#"{"command":"bun \"C:/Users/me/.claude/plugins/statusline.mjs\""}"#;
        let stored = String::from_utf8(windows().to_repo(live)).unwrap();
        assert_eq!(
            stored,
            r#"{"command":"bun \"__CLAUDE_DIR__/plugins/statusline.mjs\""}"#
        );
    }

    #[test]
    fn a_windows_machine_renders_valid_json_with_forward_slashes() {
        let from_linux =
            br#"{"statusLine":{"command":"bun \"__CLAUDE_DIR__/s.mjs\" __HOME__/bin"}}"#;
        let rendered = windows().to_machine(from_linux);
        let parsed: serde_json::Value = serde_json::from_slice(&rendered).unwrap();
        assert_eq!(
            parsed["statusLine"]["command"],
            r#"bun "C:/Users/me/.claude/s.mjs" C:/Users/me/bin"#
        );
        assert_eq!(windows().to_repo(&rendered), from_linux.to_vec());
    }

    #[test]
    fn a_path_written_on_windows_resolves_on_linux() {
        let live = br#"{"command":"C:\\Users\\me\\.claude\\hooks\\h.sh"}"#;
        let stored = windows().to_repo(live);
        let on_linux = String::from_utf8(tokens().to_machine(&stored)).unwrap();
        assert_eq!(on_linux, r#"{"command":"/home/user/.claude/hooks/h.sh"}"#);
    }

    #[test]
    fn an_escaped_quote_ends_the_path_tail() {
        let live = br#"{"command":"bun \"C:\\Users\\me\\x.mjs\""}"#;
        let stored = String::from_utf8(windows().to_repo(live)).unwrap();
        assert_eq!(stored, r#"{"command":"bun \"__HOME__/x.mjs\""}"#);
    }

    fn tokens() -> PathTokens {
        PathTokens::for_paths("/home/user", "/home/user/.claude")
    }

    #[test]
    fn tokenizes_the_claude_dir_before_the_home_directory() {
        let live = br#"{"hooks":{"Stop":[{"command":"/home/user/.claude/hooks/h.sh"}]}}"#;
        let stored = String::from_utf8(tokens().to_repo(live)).unwrap();
        assert!(stored.contains("__CLAUDE_DIR__/hooks/h.sh"), "{stored}");
        assert!(!stored.contains("/home/user"));
    }

    #[test]
    fn renders_tokens_back_to_this_machine() {
        let stored = b"node __CLAUDE_DIR__/plugins/x.js and __HOME__/bin/y";
        let live = String::from_utf8(tokens().to_machine(stored)).unwrap();
        assert_eq!(
            live,
            "node /home/user/.claude/plugins/x.js and /home/user/bin/y"
        );
    }

    #[test]
    fn a_round_trip_returns_the_original_bytes() {
        let live = br#"{"statusLine":{"command":"/home/user/bin/line --dir /home/user/.claude"}}"#;
        let stored = tokens().to_repo(live);
        assert_eq!(tokens().to_machine(&stored), live.to_vec());
    }

    #[test]
    fn another_users_home_is_left_alone() {
        let live = b"/home/username/work and /home/user/work";
        let stored = String::from_utf8(tokens().to_repo(live)).unwrap();
        assert_eq!(stored, "/home/username/work and __HOME__/work");
    }

    #[test]
    fn a_path_that_merely_contains_this_home_is_left_alone() {
        let live = b"/opt/home/user/x and /home/user/x";
        let stored = String::from_utf8(tokens().to_repo(live)).unwrap();
        assert_eq!(stored, "/opt/home/user/x and __HOME__/x");
    }

    #[test]
    fn non_utf8_content_passes_through() {
        let bytes = [0xff, 0xfe, 0x00, 0x01];
        assert_eq!(tokens().to_repo(&bytes), bytes.to_vec());
        assert_eq!(tokens().to_machine(&bytes), bytes.to_vec());
    }

    #[test]
    fn an_unknown_home_directory_changes_nothing() {
        let tokens = PathTokens::for_paths("", "/srv/claude");
        let live = b"/home/user/x";
        assert_eq!(tokens.to_repo(live), live.to_vec());
    }
}

//! Native spatial drag-and-drop URI parsing and path sanitization.

use std::path::PathBuf;

/// Sanitizes dropped payloads, decoding RFC 8089 file URIs, percent-encoded bytes,
/// and shell-escaped file paths into valid OS PathBuf representations.
pub struct PathSanitizer;

impl PathSanitizer {
    /// Parses a raw drag-and-drop payload string (which may contain multiple lines
    /// and file:// URI schemes) into sanitized PathBuf instances.
    pub fn parse_drop_payload(payload: &str) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        for line in payload.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let path_str = Self::strip_file_uri(trimmed);

            // Decode into raw bytes and preserve them: filenames are arbitrary bytes on
            // unix, so a percent-encoded non-UTF-8 path must survive round-trip rather
            // than being dropped back to its still-encoded form.
            let decoded = Self::percent_decode_bytes(path_str);
            let cleaned = Self::unescape_shell_bytes(decoded);
            if !cleaned.is_empty() {
                paths.push(Self::bytes_to_pathbuf(cleaned));
            }
        }

        paths
    }

    /// Heuristic classifier: does `payload` look like a file drag-and-drop rather than
    /// ordinary pasted text? Returns true when it uses the `file://` scheme, or when every
    /// non-empty line is shaped like a filesystem path (`/…`, `~/…`, `./…`, `../…`, or a
    /// Windows drive path like `C:\…`).
    ///
    /// This is **purely syntactic** — it never touches the filesystem, so it stays
    /// deterministic and testable. A terminal delivers both a real drop and a normal paste
    /// as `Event::Paste`, so use this to decide whether to route a payload through
    /// [`parse_drop_payload`](Self::parse_drop_payload); because a path-shaped line of prose
    /// can still slip through, callers wanting certainty should confirm the parsed paths
    /// exist before acting on them.
    pub fn looks_like_drop(payload: &str) -> bool {
        let mut saw_line = false;
        for line in payload.lines() {
            let t = line.trim();
            if t.is_empty() {
                continue;
            }
            saw_line = true;
            let is_uri = t.starts_with("file://");
            let is_path = t.starts_with('/')
                || t.starts_with("~/")
                || t.starts_with("./")
                || t.starts_with("../")
                || Self::looks_like_windows_path(t);
            if !(is_uri || is_path) {
                return false;
            }
        }
        saw_line
    }

    /// `C:\…` or `C:/…` — a drive letter, a colon, then a separator.
    fn looks_like_windows_path(s: &str) -> bool {
        let b = s.as_bytes();
        b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'/' || b[2] == b'\\')
    }

    /// Strips a leading `file://` scheme and any `localhost`/empty authority per RFC 8089,
    /// yielding the bare path. `file:///a` -> `/a`, `file://localhost/a` -> `/a`,
    /// `file://host/a` -> `host/a` (kept, as a remote authority is not a local path).
    fn strip_file_uri(input: &str) -> &str {
        let Some(rest) = input.strip_prefix("file://") else {
            return input;
        };
        let path = match rest.find('/') {
            Some(0) => rest, // file:///path — empty authority
            Some(slash) => {
                if rest[..slash].eq_ignore_ascii_case("localhost") {
                    &rest[slash..]
                } else {
                    rest
                }
            }
            None => rest,
        };
        #[cfg(windows)]
        let path = path.strip_prefix('/').unwrap_or(path); // file:///C:/x -> C:/x
        path
    }

    /// Decodes percent-encoded byte sequences (e.g. `%20` -> `' '`, `%2F` -> `'/'`)
    /// using standard library without external dependencies.
    ///
    /// Lossy in the presence of non-UTF-8 bytes; prefer [`percent_decode_bytes`] on
    /// unix where filenames may not be valid UTF-8.
    ///
    /// [`percent_decode_bytes`]: Self::percent_decode_bytes
    pub fn percent_decode(input: &str) -> String {
        String::from_utf8(Self::percent_decode_bytes(input)).unwrap_or_else(|_| input.to_string())
    }

    /// Byte-exact percent decoder. Preserves arbitrary (non-UTF-8) path bytes.
    pub fn percent_decode_bytes(input: &str) -> Vec<u8> {
        let bytes = input.as_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut i = 0;

        while i < bytes.len() {
            if bytes[i] == b'%'
                && i + 2 < bytes.len()
                && let (Some(h1), Some(h2)) = ((bytes[i + 1] as char).to_digit(16), (bytes[i + 2] as char).to_digit(16))
            {
                out.push(((h1 << 4) | h2) as u8);
                i += 3;
                continue;
            }
            out.push(bytes[i]);
            i += 1;
        }

        out
    }

    /// Strips shell escape characters from file paths (e.g. `\ ` -> `' '`, `\\` -> `'\'`).
    pub fn unescape_shell_path(input: &str) -> String {
        input.replace("\\ ", " ").replace("\\\\", "\\")
    }

    /// Byte-level shell unescape: `\ ` -> ` `, `\\` -> `\`. Single left-to-right pass so
    /// escapes don't interact (unlike chained `replace`, where the second could re-scan
    /// output of the first).
    fn unescape_shell_bytes(input: Vec<u8>) -> Vec<u8> {
        let mut out = Vec::with_capacity(input.len());
        let mut i = 0;
        while i < input.len() {
            if input[i] == b'\\' && i + 1 < input.len() {
                match input[i + 1] {
                    b' ' => {
                        out.push(b' ');
                        i += 2;
                        continue;
                    }
                    b'\\' => {
                        out.push(b'\\');
                        i += 2;
                        continue;
                    }
                    _ => {}
                }
            }
            out.push(input[i]);
            i += 1;
        }
        out
    }

    #[cfg(unix)]
    fn bytes_to_pathbuf(bytes: Vec<u8>) -> PathBuf {
        use std::os::unix::ffi::OsStringExt;
        PathBuf::from(std::ffi::OsString::from_vec(bytes))
    }

    #[cfg(not(unix))]
    fn bytes_to_pathbuf(bytes: Vec<u8>) -> PathBuf {
        PathBuf::from(String::from_utf8_lossy(&bytes).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_uri_host_stripping() {
        assert_eq!(
            PathSanitizer::parse_drop_payload("file:///home/demon/a.txt"),
            vec![PathBuf::from("/home/demon/a.txt")]
        );
        // Named/localhost authority must not leak into the path.
        assert_eq!(
            PathSanitizer::parse_drop_payload("file://localhost/home/demon/a.txt"),
            vec![PathBuf::from("/home/demon/a.txt")]
        );
        // Plain (non-URI) path is passed through.
        assert_eq!(
            PathSanitizer::parse_drop_payload("/tmp/plain path"),
            vec![PathBuf::from("/tmp/plain path")]
        );
    }

    #[test]
    fn test_percent_and_shell_decode() {
        assert_eq!(
            PathSanitizer::parse_drop_payload("file:///home/a%20b/c%2Fd"),
            vec![PathBuf::from("/home/a b/c/d")]
        );
        assert_eq!(
            PathSanitizer::parse_drop_payload("/home/a\\ b"),
            vec![PathBuf::from("/home/a b")]
        );
    }

    #[test]
    fn test_looks_like_drop_classifies_payloads() {
        // Drops: file:// URIs and path-shaped lines.
        assert!(PathSanitizer::looks_like_drop("file:///home/demon/a.txt"));
        assert!(PathSanitizer::looks_like_drop("/tmp/a.txt\n/tmp/b.txt"));
        assert!(PathSanitizer::looks_like_drop("~/notes.md"));
        assert!(PathSanitizer::looks_like_drop("./rel/path"));
        assert!(PathSanitizer::looks_like_drop("C:\\Users\\me\\f.txt"));

        // Not drops: ordinary pasted prose/code, or a mix where one line isn't path-shaped.
        assert!(!PathSanitizer::looks_like_drop("hello world"));
        assert!(!PathSanitizer::looks_like_drop("fn main() { println!(\"hi\"); }"));
        assert!(!PathSanitizer::looks_like_drop("/tmp/ok.txt\nbut this line is prose"));
        assert!(!PathSanitizer::looks_like_drop("")); // empty is not a drop
        assert!(!PathSanitizer::looks_like_drop("   \n  ")); // whitespace-only is not a drop
    }

    #[cfg(unix)]
    #[test]
    fn test_non_utf8_path_preserved() {
        use std::os::unix::ffi::OsStrExt;
        // %FF is not valid UTF-8; the byte must survive into the PathBuf.
        let paths = PathSanitizer::parse_drop_payload("file:///tmp/%FFname");
        assert_eq!(paths.len(), 1);
        assert!(paths[0].as_os_str().as_bytes().contains(&0xFF));
    }
}

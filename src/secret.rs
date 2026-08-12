use crate::file::{inspect_file_with_options, ContentOptions, InspectedFile};
use crate::tree::{collect_files, Snapshot};
use regex::Regex;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretFinding {
    pub path: PathBuf,
    pub line: usize,
    pub kind: &'static str,
}

struct SecretPattern {
    kind: &'static str,
    regex: Regex,
}

pub fn scan_snapshot_for_secrets(
    snapshot: &Snapshot,
    max_size: u64,
    options: &ContentOptions,
) -> Vec<SecretFinding> {
    let mut findings = Vec::new();

    for path in collect_files(&snapshot.tree, &snapshot.root) {
        let Ok(InspectedFile::Text(text)) = inspect_file_with_options(&path, max_size, options)
        else {
            continue;
        };
        let relative = path
            .strip_prefix(&snapshot.root)
            .unwrap_or(&path)
            .to_path_buf();

        for (line_index, line) in text.lines().enumerate() {
            for pattern in secret_patterns() {
                if pattern.regex.is_match(line) {
                    findings.push(SecretFinding {
                        path: relative.clone(),
                        line: line_index + 1,
                        kind: pattern.kind,
                    });
                }
            }
        }
    }

    findings
}

fn secret_patterns() -> &'static [SecretPattern] {
    static PATTERNS: OnceLock<Vec<SecretPattern>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        [
            (
                "private key",
                r"-----BEGIN (?:[A-Z0-9 ]+ )?PRIVATE KEY-----",
            ),
            ("AWS access key", r"\b(?:AKIA|ASIA)[A-Z0-9]{16}\b"),
            (
                "GitHub token",
                r"\b(?:gh[pousr]_[A-Za-z0-9]{36,255}|github_pat_[A-Za-z0-9_]{22,255})\b",
            ),
            (
                "GitLab token",
                r"\bglpat-[0-9A-Za-z_-]{20,}\b",
            ),
            (
                "OpenAI API key",
                r"\bsk-(?:proj-|svcacct-)?[0-9A-Za-z_-]{20,}\b",
            ),
            (
                "Google API key",
                r"\bAIza[0-9A-Za-z_-]{35}\b",
            ),
            (
                "Stripe live key",
                r"\b(?:sk|rk)_live_[0-9A-Za-z]{16,}\b",
            ),
            (
                "Slack token",
                r"\bxox[baprs]-[0-9A-Za-z-]{10,}\b",
            ),
            (
                "JWT",
                r"\beyJ[0-9A-Za-z_-]{8,}\.[0-9A-Za-z_-]{8,}\.[0-9A-Za-z_-]{8,}\b",
            ),
            (
                "authorization header",
                r"(?i)\bauthorization\s*[:=]\s*(?:bearer|basic)\s+[0-9A-Za-z._~+/-]{8,}={0,2}",
            ),
            (
                "credential assignment",
                r#"(?i)\b(?:api[_-]?key|access[_-]?token|auth[_-]?token|client[_-]?secret|password|passwd|pwd|secret[_-]?key)\b\s*[:=]\s*["']?[^\s"'#,;]{8,}"#,
            ),
        ]
        .into_iter()
        .map(|(kind, pattern)| SecretPattern {
            kind,
            regex: Regex::new(pattern).expect("secret detection regex should be valid"),
        })
        .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::insert_entry;
    use std::collections::BTreeMap;
    use std::fs;

    #[test]
    fn finds_common_secrets_without_retaining_values() {
        let root = std::env::temp_dir().join(format!("ccp-secret-test-{}", std::process::id()));
        let env_path = root.join("config.env");
        let key_path = root.join("server.pem");

        fs::create_dir_all(&root).expect("test root should be created");
        fs::write(
            &env_path,
            "SAFE=value\nPASSWORD=correct-horse-battery-staple\nAWS=AKIAIOSFODNN7EXAMPLE\n",
        )
        .expect("environment file should be written");
        fs::write(
            &key_path,
            "-----BEGIN PRIVATE KEY-----\nredacted\n-----END PRIVATE KEY-----\n",
        )
        .expect("private key should be written");

        let mut tree = BTreeMap::new();
        insert_entry(&mut tree, &[String::from("config.env")], false);
        insert_entry(&mut tree, &[String::from("server.pem")], false);
        let snapshot = Snapshot { root, tree };

        let findings = scan_snapshot_for_secrets(&snapshot, 10_000, &ContentOptions::default());

        assert!(findings.contains(&SecretFinding {
            path: PathBuf::from("config.env"),
            line: 2,
            kind: "credential assignment",
        }));
        assert!(findings.contains(&SecretFinding {
            path: PathBuf::from("config.env"),
            line: 3,
            kind: "AWS access key",
        }));
        assert!(findings.contains(&SecretFinding {
            path: PathBuf::from("server.pem"),
            line: 1,
            kind: "private key",
        }));
        assert!(findings
            .iter()
            .all(|finding| !format!("{finding:?}").contains("correct-horse")));

        fs::remove_dir_all(&snapshot.root).expect("test root should be removed");
    }

    #[test]
    fn ignores_binary_and_oversized_files() {
        let root =
            std::env::temp_dir().join(format!("ccp-secret-skip-test-{}", std::process::id()));
        let binary_path = root.join("secret.bin");
        let large_path = root.join("large.txt");

        fs::create_dir_all(&root).expect("test root should be created");
        fs::write(&binary_path, b"\0PASSWORD=very-secret-value")
            .expect("binary file should be written");
        fs::write(&large_path, "PASSWORD=very-secret-value").expect("large file should be written");

        let mut tree = BTreeMap::new();
        insert_entry(&mut tree, &[String::from("secret.bin")], false);
        insert_entry(&mut tree, &[String::from("large.txt")], false);
        let snapshot = Snapshot { root, tree };

        assert!(scan_snapshot_for_secrets(&snapshot, 4, &ContentOptions::default()).is_empty());

        fs::remove_dir_all(&snapshot.root).expect("test root should be removed");
    }
}

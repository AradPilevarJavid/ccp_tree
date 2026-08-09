use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitMetadata {
    pub branch: Option<String>,
    pub commit_hash: Option<String>,
    pub remote_url: Option<String>,
}

pub fn detect_git_metadata(root: &Path) -> Option<GitMetadata> {
    if git_output(root, &["rev-parse", "--is-inside-work-tree"]).as_deref() != Some("true") {
        return None;
    }

    let branch = git_output(root, &["symbolic-ref", "--quiet", "--short", "HEAD"]);
    let commit_hash = git_output(root, &["rev-parse", "HEAD"]);
    let remote_url = git_output(root, &["remote", "get-url", "origin"]).or_else(|| {
        let remote = git_output(root, &["remote"])?
            .lines()
            .find(|line| !line.trim().is_empty())?
            .trim()
            .to_string();
        git_output(root, &["remote", "get-url", &remote])
    });

    Some(GitMetadata {
        branch,
        commit_hash,
        remote_url,
    })
}

fn git_output(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let output = String::from_utf8(output.stdout).ok()?;
    let output = output.lines().next()?.trim();
    if output.is_empty() {
        None
    } else {
        Some(output.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_ID: AtomicUsize = AtomicUsize::new(0);

    fn test_root(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "ccp-git-{name}-{}-{}",
            std::process::id(),
            TEST_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn run_git(root: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .status()
            .expect("git should be installed for repository tests");
        assert!(status.success(), "git command should succeed: {args:?}");
    }

    fn create_committed_repo(name: &str) -> std::path::PathBuf {
        let root = test_root(name);
        fs::create_dir_all(&root).expect("test root should be created");
        run_git(&root, &["init", "-q", "-b", "main"]);
        run_git(&root, &["config", "user.name", "CCP Tests"]);
        run_git(&root, &["config", "user.email", "ccp@example.com"]);
        fs::write(root.join("README.md"), "test").expect("test file should be written");
        run_git(&root, &["add", "README.md"]);
        run_git(&root, &["commit", "-q", "-m", "test"]);
        root
    }

    #[test]
    fn detects_branch_commit_and_remote_from_repository_and_subdirectory() {
        let root = create_committed_repo("metadata");
        let nested = root.join("src");
        fs::create_dir_all(&nested).expect("nested directory should be created");
        run_git(
            &root,
            &["remote", "add", "origin", "git@example.com:owner/repo.git"],
        );

        let metadata = detect_git_metadata(&nested).expect("Git metadata should be detected");

        assert_eq!(metadata.branch.as_deref(), Some("main"));
        assert_eq!(metadata.commit_hash.as_deref().map(str::len), Some(40));
        assert_eq!(
            metadata.remote_url.as_deref(),
            Some("git@example.com:owner/repo.git")
        );

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn detached_repository_keeps_commit_without_branch_or_remote() {
        let root = create_committed_repo("detached");
        run_git(&root, &["checkout", "-q", "--detach"]);

        let metadata = detect_git_metadata(&root).expect("Git metadata should be detected");

        assert_eq!(metadata.branch, None);
        assert_eq!(metadata.commit_hash.as_deref().map(str::len), Some(40));
        assert_eq!(metadata.remote_url, None);

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn repository_without_remote_keeps_branch_and_commit() {
        let root = create_committed_repo("no-remote");

        let metadata = detect_git_metadata(&root).expect("Git metadata should be detected");

        assert_eq!(metadata.branch.as_deref(), Some("main"));
        assert_eq!(metadata.commit_hash.as_deref().map(str::len), Some(40));
        assert_eq!(metadata.remote_url, None);

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn non_git_directory_has_no_metadata() {
        let root = test_root("plain");
        fs::create_dir_all(&root).expect("test root should be created");

        assert_eq!(detect_git_metadata(&root), None);

        fs::remove_dir_all(root).expect("test root should be removed");
    }
}

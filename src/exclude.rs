use anyhow::{Context, Result};
use glob::Pattern;
use std::fs;
use std::path::Path;

pub const DEFAULT_EXCLUDES: &[&str] = &[
    "target/",
    "node_modules/",
    "dist/",
    "build/",
    ".next/",
    ".nuxt/",
    ".svelte-kit/",
    ".turbo/",
    ".cache/",
    "coverage/",
    "__pycache__/",
    ".pytest_cache/",
    ".mypy_cache/",
    ".ruff_cache/",
    ".tox/",
    ".venv/",
    "venv/",
    ".gradle/",
    "cmake-build-debug/",
    "cmake-build-release/",
    "*.log",
    "logs/",
    ".log",
    "out/",
    "bin/",
    "obj/",
    "*.egg-info/",
    ".eggs/",
    ".pnp.*",
    ".yarn/",
    "vendor/",
    "Pods/",
    ".idea/",
    ".vscode/",
    "*.swp",
    "*.swo",
    ".DS_Store",
    ".nyc_output/",
    "htmlcov/",
    ".coverage",
    "test-results/",
    "playwright-report/",
    "tmp/",
    "temp/",
    ".tmp/",
    "Thumbs.db",
    "desktop.ini",
    "*.mp4",
    "*.zip",
    "*.tar.gz",
    "*.pdf",
    "public/uploads/",
    "storage/",
    "data/",
    ".env.local",
    ".env.*.local",
    ".git/",
    "Cargo.lock",
];

#[derive(Debug, Clone)]
pub struct ExcludePattern {
    raw: String,
    pattern: Pattern,
    directory_only: bool,
}

impl ExcludePattern {
    pub fn new(raw: &str) -> Result<Self> {
        let directory_only = raw.ends_with('/');
        let pattern_text = raw.trim_end_matches('/');
        let pattern = Pattern::new(pattern_text)
            .with_context(|| format!("Invalid exclusion pattern: {raw}"))?;
        Ok(Self {
            raw: pattern_text.to_string(),
            pattern,
            directory_only,
        })
    }

    pub fn matches(&self, relative: &Path, is_dir: bool) -> bool {
        if self.directory_only && !is_dir {
            return false;
        }

        let relative_text = relative.to_string_lossy().replace('\\', "/");
        if self.pattern.matches(&relative_text) {
            return true;
        }

        if self.raw.contains('/') {
            return false;
        }

        relative
            .components()
            .filter_map(|component| component.as_os_str().to_str())
            .any(|component| self.pattern.matches(component))
    }
}

pub fn load_ignore_patterns(
    root: &Path,
    excludes: &[String],
    use_mktree_ignore: bool,
) -> Result<Vec<ExcludePattern>> {
    let mut patterns = excludes.to_vec();
    if use_mktree_ignore {
        let ignore_path = root.join(".mktreeignore");
        if ignore_path.exists() {
            let content = fs::read_to_string(&ignore_path)
                .with_context(|| format!("Failed to read {}", ignore_path.display()))?;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                patterns.push(trimmed.to_string());
            }
        }
    }

    patterns
        .iter()
        .map(|pattern| ExcludePattern::new(pattern))
        .collect()
}

pub fn load_default_exclude_patterns() -> Result<Vec<ExcludePattern>> {
    DEFAULT_EXCLUDES
        .iter()
        .map(|pattern| ExcludePattern::new(pattern))
        .collect()
}

pub fn should_exclude(relative: &Path, is_dir: bool, patterns: &[ExcludePattern]) -> bool {
    patterns
        .iter()
        .any(|pattern| pattern.matches(relative, is_dir))
}

pub fn is_useless_dir_name(name: &str) -> bool {
    DEFAULT_EXCLUDES
        .iter()
        .filter(|pattern| pattern.ends_with('/') && !pattern.contains('*'))
        .map(|pattern| pattern.trim_end_matches('/'))
        .any(|pattern| pattern == name)
}

pub fn is_useless_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(is_useless_dir_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_patterns() -> Vec<ExcludePattern> {
        load_default_exclude_patterns().expect("default exclude patterns should be valid")
    }

    #[test]
    fn default_excludes_match_nested_directories() {
        let patterns = default_patterns();

        assert!(should_exclude(
            Path::new("frontend/node_modules/react/index.js"),
            true,
            &patterns
        ));
        assert!(should_exclude(
            Path::new("app/.next/cache"),
            true,
            &patterns
        ));
        assert!(should_exclude(
            Path::new("service/__pycache__"),
            true,
            &patterns
        ));
    }

    #[test]
    fn default_excludes_match_file_globs_and_exact_files() {
        let patterns = default_patterns();

        assert!(should_exclude(Path::new("debug.log"), false, &patterns));
        assert!(should_exclude(
            Path::new("src/.env.production.local"),
            false,
            &patterns
        ));
        assert!(should_exclude(Path::new("Cargo.lock"), false, &patterns));
        assert!(should_exclude(
            Path::new("docs/archive.tar.gz"),
            false,
            &patterns
        ));
    }

    #[test]
    fn directory_only_defaults_do_not_match_files_with_same_name() {
        let patterns = default_patterns();

        assert!(!should_exclude(Path::new("docs/target"), false, &patterns));
    }
}

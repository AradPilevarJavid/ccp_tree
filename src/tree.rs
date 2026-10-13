use crate::exclude::{load_default_exclude_patterns, load_ignore_patterns, should_exclude};
use anyhow::{Context, Result};
use ignore::WalkBuilder;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub is_dir: bool,
    pub children: BTreeMap<String, Entry>,
}

impl Entry {
    pub fn new(name: String, is_dir: bool) -> Self {
        Self {
            name,
            is_dir,
            children: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WalkOptions {
    pub include_hidden: bool,
    pub no_ignore: bool,
    pub include_useless: bool,
    pub exclude: Vec<String>,
    pub mktree_ignore: bool,
    pub max_size: u64,
    pub file_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub root: PathBuf,
    pub tree: BTreeMap<String, Entry>,
}

pub fn normalize_file_type(file_type: &str) -> Result<String> {
    let trimmed = file_type.trim();
    let normalized = trimmed.strip_prefix('.').unwrap_or(trimmed);
    if normalized.is_empty()
        || normalized.contains('.')
        || normalized.contains('/')
        || normalized.contains('\\')
    {
        anyhow::bail!("file type must be a single extension such as `rs` or `.rs`");
    }
    Ok(normalized.to_ascii_lowercase())
}

fn file_matches_type(path: &Path, file_type: Option<&str>) -> bool {
    match file_type {
        None => true,
        Some(file_type) => path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case(file_type)),
    }
}

pub fn insert_entry(root: &mut BTreeMap<String, Entry>, components: &[String], is_dir: bool) {
    if components.is_empty() {
        return;
    }
    let name = &components[0];
    let entry = root
        .entry(name.clone())
        .or_insert_with(|| Entry::new(name.clone(), components.len() > 1 || is_dir));
    if components.len() > 1 {
        entry.is_dir = true;
        insert_entry(&mut entry.children, &components[1..], is_dir);
    } else {
        entry.is_dir = is_dir;
    }
}

pub fn snapshot(root: &Path, options: &WalkOptions) -> Result<Snapshot> {
    let file_type = options
        .file_type
        .as_deref()
        .map(normalize_file_type)
        .transpose()?;
    if root.is_file() {
        let parent = root.parent().unwrap_or_else(|| Path::new("."));
        let name = root
            .file_name()
            .context("snapshot file root has no file name")?
            .to_string_lossy()
            .into_owned();
        let mut tree = BTreeMap::new();
        if file_matches_type(root, file_type.as_deref()) {
            insert_entry(&mut tree, &[name], false);
        }
        return Ok(Snapshot {
            root: parent.to_path_buf(),
            tree,
        });
    }

    let root = root.to_path_buf();
    let root_for_filter = root.clone();
    let include_useless = options.include_useless;
    let mut exclude_patterns = if include_useless {
        Vec::new()
    } else {
        load_default_exclude_patterns()?
    };
    if let Some(file_type) = file_type.as_deref() {
        exclude_patterns.retain(|pattern| !pattern.targets_file_type(file_type));
    }
    exclude_patterns.extend(load_ignore_patterns(
        &root,
        &options.exclude,
        options.mktree_ignore,
    )?);

    let mut builder = WalkBuilder::new(&root);
    builder
        .hidden(!options.include_hidden)
        .git_ignore(!options.no_ignore)
        .ignore(!options.no_ignore)
        .follow_links(false);

    builder.filter_entry(move |entry| {
        let path = entry.path();
        if path == root_for_filter {
            return true;
        }
        let relative = path.strip_prefix(&root_for_filter).unwrap_or(path);
        let is_dir = entry
            .file_type()
            .map(|file_type| file_type.is_dir())
            .unwrap_or(false);
        !should_exclude(relative, is_dir, &exclude_patterns)
    });

    let mut tree = BTreeMap::new();
    for result in builder.build() {
        let entry = result?;
        let path = entry.path();
        if path == root {
            continue;
        }
        let relative = path.strip_prefix(&root).unwrap_or(path);
        let components: Vec<String> = relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy().into_owned())
            .collect();
        let is_dir = entry
            .file_type()
            .map(|file_type| file_type.is_dir())
            .unwrap_or(false);
        if file_type.is_none() || (!is_dir && file_matches_type(path, file_type.as_deref())) {
            insert_entry(&mut tree, &components, is_dir);
        }
    }

    Ok(Snapshot { root, tree })
}

fn fmt_tree_entries(entries: &BTreeMap<String, Entry>, prefix: &str) -> String {
    let mut out = String::new();
    let entries_vec: Vec<&Entry> = entries.values().collect();
    let count = entries_vec.len();
    for (index, entry) in entries_vec.iter().enumerate() {
        let last_child = index == count - 1;
        let (connector, child_prefix) = if last_child {
            ("└── ", format!("{}    ", prefix))
        } else {
            ("├── ", format!("{}│   ", prefix))
        };
        let display_name = if entry.is_dir {
            format!("{}/", entry.name)
        } else {
            entry.name.clone()
        };
        out.push_str(&format!("{}{}{}\n", prefix, connector, display_name));
        if entry.is_dir && !entry.children.is_empty() {
            out.push_str(&fmt_tree_entries(&entry.children, &child_prefix));
        }
    }
    out
}

pub fn fmt_tree(entries: &BTreeMap<String, Entry>, prefix: &str) -> String {
    fmt_tree_entries(entries, prefix)
}

pub(crate) fn root_display_name(root: &Path) -> String {
    root.file_name()
        .filter(|name| !name.is_empty() && *name != "." && *name != "..")
        .map(|name| name.to_string_lossy().into_owned())
        .or_else(|| {
            std::env::current_dir().ok().and_then(|current_dir| {
                current_dir
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            })
        })
        .unwrap_or_else(|| root.display().to_string())
}

pub(crate) fn fmt_tree_with_root(root: &Path, entries: &BTreeMap<String, Entry>) -> String {
    let mut output = format!("{}/\n", root_display_name(root));
    output.push_str(&fmt_tree(entries, ""));
    output
}

pub fn collect_files(entries: &BTreeMap<String, Entry>, current_path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in entries.values() {
        let child_path = current_path.join(&entry.name);
        if entry.is_dir {
            files.extend(collect_files(&entry.children, &child_path));
        } else {
            files.push(child_path);
        }
    }
    files
}

pub(crate) fn count_dirs(entries: &BTreeMap<String, Entry>) -> usize {
    entries
        .values()
        .map(|entry| {
            if entry.is_dir {
                1 + count_dirs(&entry.children)
            } else {
                0
            }
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn tree_render_does_not_append_directory_and_file_counts() {
        let mut tree = BTreeMap::new();
        insert_entry(&mut tree, &[String::from("README.md")], false);
        insert_entry(
            &mut tree,
            &[String::from("src"), String::from("main.rs")],
            false,
        );

        let output = fmt_tree(&tree, "");

        assert!(output.contains("└── src/\n    └── main.rs\n"));
        assert!(!output.contains("directories,"));
        assert!(!output.contains("files\n\n"));
    }

    #[test]
    fn snapshot_accepts_a_file_as_the_root() {
        let root = std::env::temp_dir().join(format!("ccp-file-root-test-{}", std::process::id()));
        let notebook = root.join("analysis.ipynb");
        fs::create_dir_all(&root).expect("test root should be created");
        fs::write(&notebook, "{}").expect("notebook should be written");

        let snapshot = snapshot(
            &notebook,
            &WalkOptions {
                include_hidden: false,
                no_ignore: false,
                include_useless: false,
                exclude: Vec::new(),
                mktree_ignore: true,
                max_size: 1_000,
                file_type: None,
            },
        )
        .expect("file root should be accepted");

        assert_eq!(snapshot.root, root);
        assert!(snapshot.tree.contains_key("analysis.ipynb"));

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn snapshot_filters_files_by_final_extension_and_preserves_parents() {
        let root = std::env::temp_dir().join(format!("ccp-file-type-test-{}", std::process::id()));
        fs::create_dir_all(root.join("src/nested")).expect("test directories should be created");
        fs::write(root.join("src/main.RS"), "fn main() {}").expect("Rust file should be written");
        fs::write(root.join("src/nested/lib.rs"), "pub fn lib() {}")
            .expect("nested Rust file should be written");
        fs::write(root.join("src/main.rs.txt"), "not Rust").expect("text file should be written");
        fs::write(root.join("README"), "no extension")
            .expect("extensionless file should be written");
        fs::create_dir(root.join("folder.mp4"))
            .expect("extension-like directory should be created");

        let snapshot = snapshot(
            &root,
            &WalkOptions {
                include_hidden: false,
                no_ignore: true,
                include_useless: true,
                exclude: Vec::new(),
                mktree_ignore: false,
                max_size: 1_000,
                file_type: Some(".rS".to_string()),
            },
        )
        .expect("filtered snapshot should succeed");

        assert!(snapshot.tree.contains_key("src"));
        let src = snapshot.tree.get("src").expect("src should be retained");
        assert!(src.children.contains_key("main.RS"));
        let nested = src
            .children
            .get("nested")
            .expect("nested should be retained");
        assert!(nested.children.contains_key("lib.rs"));
        assert!(!src.children.contains_key("main.rs.txt"));
        assert!(!snapshot.tree.contains_key("README"));
        assert!(!snapshot.tree.contains_key("folder.mp4"));

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn snapshot_filters_a_nonmatching_file_root_without_error() {
        let root =
            std::env::temp_dir().join(format!("ccp-file-type-root-test-{}", std::process::id()));
        fs::create_dir_all(&root).expect("test root should be created");
        let file = root.join("notes.txt");
        fs::write(&file, "notes").expect("file should be written");

        let snapshot = snapshot(
            &file,
            &WalkOptions {
                include_hidden: false,
                no_ignore: false,
                include_useless: false,
                exclude: Vec::new(),
                mktree_ignore: true,
                max_size: 1_000,
                file_type: Some("rs".to_string()),
            },
        )
        .expect("nonmatching file root should succeed");

        assert!(snapshot.tree.is_empty());
        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn file_type_narrowly_overrides_matching_default_file_exclusion() {
        let root = std::env::temp_dir().join(format!(
            "ccp-file-type-default-exclude-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("target")).expect("target directory should be created");
        fs::write(root.join("video.mp4"), [0, 1, 2]).expect("video should be written");
        fs::write(root.join("target/nested.mp4"), [3, 4, 5])
            .expect("nested video should be written");

        let snapshot = snapshot(
            &root,
            &WalkOptions {
                include_hidden: false,
                no_ignore: false,
                include_useless: false,
                exclude: Vec::new(),
                mktree_ignore: true,
                max_size: 1_000,
                file_type: Some("mp4".to_string()),
            },
        )
        .expect("filtered snapshot should succeed");

        assert!(snapshot.tree.contains_key("video.mp4"));
        assert!(!snapshot.tree.contains_key("target"));
        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn file_type_respects_user_exclusions_and_ignore_modes() {
        let root =
            std::env::temp_dir().join(format!("ccp-file-type-ignores-test-{}", std::process::id()));
        fs::create_dir_all(root.join("target")).expect("target directory should be created");
        fs::create_dir_all(root.join(".git")).expect("Git repository marker should be created");
        fs::write(root.join(".gitignore"), "ignored_git.mp4\n")
            .expect(".gitignore should be written");
        fs::write(root.join(".ignore"), "ignored_ignore.mp4\n").expect(".ignore should be written");
        fs::write(root.join(".mktreeignore"), "ignored_mktree.mp4\n")
            .expect(".mktreeignore should be written");
        for name in [
            "kept.mp4",
            "ignored_git.mp4",
            "ignored_ignore.mp4",
            "ignored_mktree.mp4",
            "explicit.mp4",
        ] {
            fs::write(root.join(name), [1, 2, 3]).expect("fixture file should be written");
        }
        fs::write(root.join("target/generated.mp4"), [4, 5, 6])
            .expect("generated file should be written");

        let scan = |include_useless: bool, no_ignore: bool, include_hidden: bool| {
            snapshot(
                &root,
                &WalkOptions {
                    include_hidden,
                    no_ignore,
                    include_useless,
                    exclude: vec!["explicit.mp4".to_string()],
                    mktree_ignore: true,
                    max_size: 1_000,
                    file_type: Some("mp4".to_string()),
                },
            )
            .expect("filtered snapshot should succeed")
        };

        let normal = scan(false, false, false);
        assert!(normal.tree.contains_key("kept.mp4"));
        assert!(!normal.tree.contains_key("ignored_git.mp4"));
        assert!(!normal.tree.contains_key("ignored_ignore.mp4"));
        assert!(!normal.tree.contains_key("ignored_mktree.mp4"));
        assert!(!normal.tree.contains_key("explicit.mp4"));
        assert!(!normal.tree.contains_key("target"));

        let no_ignore = scan(false, true, false);
        assert!(no_ignore.tree.contains_key("kept.mp4"));
        assert!(no_ignore.tree.contains_key("ignored_git.mp4"));
        assert!(no_ignore.tree.contains_key("ignored_ignore.mp4"));
        assert!(!no_ignore.tree.contains_key("ignored_mktree.mp4"));
        assert!(!no_ignore.tree.contains_key("explicit.mp4"));
        assert!(!no_ignore.tree.contains_key("target"));

        let all = scan(true, true, true);
        assert!(all.tree.contains_key("kept.mp4"));
        assert!(all.tree.contains_key("ignored_git.mp4"));
        assert!(all.tree.contains_key("ignored_ignore.mp4"));
        assert!(!all.tree.contains_key("ignored_mktree.mp4"));
        assert!(!all.tree.contains_key("explicit.mp4"));
        let target = all
            .tree
            .get("target")
            .expect("target should be included with --all");
        assert!(target.children.contains_key("generated.mp4"));

        fs::remove_dir_all(root).expect("test root should be removed");
    }
}

use crate::exclude::{load_default_exclude_patterns, load_ignore_patterns, should_exclude};
use anyhow::Result;
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
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub root: PathBuf,
    pub tree: BTreeMap<String, Entry>,
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
    let root = root.to_path_buf();
    let root_for_filter = root.clone();
    let include_useless = options.include_useless;
    let mut exclude_patterns = if include_useless {
        Vec::new()
    } else {
        load_default_exclude_patterns()?
    };
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
        insert_entry(&mut tree, &components, is_dir);
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
}

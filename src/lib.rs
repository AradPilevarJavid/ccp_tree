use anyhow::{bail, Context, Result};
use glob::Pattern;
use ignore::WalkBuilder;
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, IsTerminal, Write};
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

#[derive(Debug, Clone)]
pub enum FileText {
    Text(String),
    Binary,
    TooLarge(u64),
}

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub name: String,
    pub is_dir: bool,
    pub content: Option<String>,
    pub children: Vec<TreeNode>,
}

#[derive(Debug, Clone)]
pub struct GenerateOptions {
    pub force: bool,
    pub dry_run: bool,
    pub verbose: bool,
    pub quiet: bool,
}

#[derive(Debug, Clone)]
pub struct GenerateEvent {
    pub action: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

pub fn load_ignore_patterns(
    root: &Path,
    excludes: &[String],
    use_mktree_ignore: bool,
) -> Result<Vec<Pattern>> {
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
        .map(|pattern| {
            Pattern::new(pattern).with_context(|| format!("Invalid exclusion pattern: {pattern}"))
        })
        .collect()
}

pub fn should_exclude(relative: &Path, patterns: &[Pattern]) -> bool {
    let rel_str = relative.to_string_lossy();
    patterns.iter().any(|pattern| pattern.matches(&rel_str))
}

pub fn is_useless_dir_name(name: &str) -> bool {
    matches!(
        name,
        "target"
            | "node_modules"
            | "dist"
            | "build"
            | ".next"
            | ".nuxt"
            | ".svelte-kit"
            | ".turbo"
            | ".cache"
            | "coverage"
            | "__pycache__"
            | ".pytest_cache"
            | ".mypy_cache"
            | ".ruff_cache"
            | ".tox"
            | ".venv"
            | "venv"
            | ".gradle"
            | "cmake-build-debug"
            | "cmake-build-release"
    )
}

pub fn is_useless_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(is_useless_dir_name)
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
    let exclude_patterns = load_ignore_patterns(&root, &options.exclude, options.mktree_ignore)?;
    let include_useless = options.include_useless;

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
        if !include_useless
            && entry
                .file_type()
                .map(|file_type| file_type.is_dir())
                .unwrap_or(false)
            && is_useless_dir(relative)
        {
            return false;
        }
        !should_exclude(relative, &exclude_patterns)
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

pub fn fmt_tree(entries: &BTreeMap<String, Entry>, prefix: &str) -> String {
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
            out.push_str(&fmt_tree(&entry.children, &child_prefix));
        }
    }
    out
}

pub fn fmt_colored_tree(entries: &BTreeMap<String, Entry>, prefix: &str) -> String {
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
            format!("\x1b[34m{}/\x1b[0m", entry.name)
        } else {
            format!("\x1b[32m{}\x1b[0m", entry.name)
        };
        out.push_str(&format!("{}{}{}\n", prefix, connector, display_name));
        if entry.is_dir && !entry.children.is_empty() {
            out.push_str(&fmt_colored_tree(&entry.children, &child_prefix));
        }
    }
    out
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

pub fn read_file_text(path: &Path, max_size: u64) -> Result<FileText> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > max_size {
        return Ok(FileText::TooLarge(max_size));
    }
    let bytes = fs::read(path)?;
    match String::from_utf8(bytes) {
        Ok(text) => Ok(FileText::Text(text)),
        Err(_) => Ok(FileText::Binary),
    }
}

pub fn file_content(path: &Path, max_size: u64) -> Result<String> {
    match read_file_text(path, max_size)? {
        FileText::Text(text) => Ok(text),
        FileText::Binary => Ok("[Binary file not shown]".to_string()),
        FileText::TooLarge(size) => Ok(format!("[File too large, > {} bytes]", size)),
    }
}

pub fn render_markdown(snapshot: &Snapshot, max_size: u64) -> String {
    let tree_str = fmt_tree(&snapshot.tree, "");
    let mut output = format!("# Project Structure\n\n```\n{}```\n", tree_str);
    output.push_str("\n# File Contents\n");
    let file_paths = collect_files(&snapshot.tree, &snapshot.root);
    for path in &file_paths {
        let relative = path.strip_prefix(&snapshot.root).unwrap_or(path);
        output.push_str(&format!("\n## {}\n\n```\n", relative.display()));
        match file_content(path, max_size) {
            Ok(content) => output.push_str(&content),
            Err(error) => output.push_str(&format!("[Error reading file: {}]", error)),
        }
        output.push_str("\n```\n");
    }
    output
}

pub fn render_tree_definition(snapshot: &Snapshot, max_size: u64, no_content: bool) -> String {
    render_tree_definition_entries(&snapshot.tree, &snapshot.root, 0, max_size, no_content)
}

fn render_tree_definition_entries(
    entries: &BTreeMap<String, Entry>,
    current_path: &Path,
    depth: usize,
    max_size: u64,
    no_content: bool,
) -> String {
    let mut out = String::new();
    let indent = "  ".repeat(depth);
    for entry in entries.values() {
        let child_path = current_path.join(&entry.name);
        if entry.is_dir {
            out.push_str(&format!("{}{}/\n", indent, entry.name));
            out.push_str(&render_tree_definition_entries(
                &entry.children,
                &child_path,
                depth + 1,
                max_size,
                no_content,
            ));
            continue;
        }

        if no_content {
            out.push_str(&format!("{}{}\n", indent, entry.name));
            continue;
        }

        match read_file_text(&child_path, max_size) {
            Ok(FileText::Text(text)) if text.is_empty() => {
                out.push_str(&format!("{}{}\n", indent, entry.name))
            }
            Ok(FileText::Text(text)) if is_single_line(&text) => {
                out.push_str(&format!("{}{}: {}\n", indent, entry.name, text));
            }
            Ok(FileText::Text(text)) => {
                out.push_str(&format!("{}{}:|\n", indent, entry.name));
                for line in text.lines() {
                    out.push_str(&format!("{}  {}\n", indent, line));
                }
                if text.ends_with('\n') {
                    out.push_str(&format!("{}  \n", indent));
                }
            }
            Ok(FileText::Binary) => {
                out.push_str(&format!("{}{}: <binary file>\n", indent, entry.name))
            }
            Ok(FileText::TooLarge(_)) => {
                out.push_str(&format!("{}{}: <file too large>\n", indent, entry.name))
            }
            Err(error) => out.push_str(&format!("{}{}: <error: {}>\n", indent, entry.name, error)),
        }
    }
    out
}

fn is_single_line(text: &str) -> bool {
    !text.contains('\n') && !text.contains('\r')
}

pub fn parse_tree_definition(input: &str) -> Result<Vec<TreeNode>> {
    let lines: Vec<&str> = input.lines().collect();
    let mut index = 0;
    parse_nodes(&lines, &mut index, 0)
}

fn parse_nodes(lines: &[&str], index: &mut usize, depth: usize) -> Result<Vec<TreeNode>> {
    let mut nodes = Vec::new();
    while *index < lines.len() {
        let line = lines[*index];
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            *index += 1;
            continue;
        }

        let current_depth = indentation_depth(line)?;
        if current_depth < depth {
            break;
        }
        if current_depth > depth {
            bail!("Unexpected indentation on line {}", *index + 1);
        }

        let trimmed = line.trim_start();
        *index += 1;
        let mut node = parse_node_header(trimmed)?;

        if node.content.as_deref() == Some("__MULTILINE__") {
            let mut content_lines = Vec::new();
            while *index < lines.len() {
                let content_line = lines[*index];
                if content_line.trim().is_empty() {
                    content_lines.push(String::new());
                    *index += 1;
                    continue;
                }
                let content_depth = indentation_depth(content_line)?;
                if content_depth <= current_depth {
                    break;
                }
                let strip_chars = ((current_depth + 1) * 2).min(content_line.len());
                let content = if content_line.len() >= strip_chars {
                    content_line[strip_chars..].to_string()
                } else {
                    String::new()
                };
                content_lines.push(content);
                *index += 1;
            }
            node.content = Some(content_lines.join("\n"));
        }

        if node.is_dir {
            node.children = parse_nodes(lines, index, depth + 1)?;
        }
        nodes.push(node);
    }
    Ok(nodes)
}

fn indentation_depth(line: &str) -> Result<usize> {
    let mut spaces = 0;
    for character in line.chars() {
        match character {
            ' ' => spaces += 1,
            '\t' => bail!("Tabs are not supported for indentation"),
            _ => break,
        }
    }
    if spaces % 2 != 0 {
        bail!("Indentation must use multiples of two spaces");
    }
    Ok(spaces / 2)
}

fn parse_node_header(header: &str) -> Result<TreeNode> {
    let (raw_name, content) = match header.split_once(':') {
        Some((name, "|")) => (name.trim(), Some("__MULTILINE__".to_string())),
        Some((name, value)) => (name.trim(), Some(value.trim_start().to_string())),
        None => (header.trim(), None),
    };

    if raw_name.is_empty() {
        bail!("Tree entry names cannot be empty");
    }
    if raw_name.contains('/') && !raw_name.ends_with('/') {
        bail!("Nested paths are not supported inside a single tree entry: {raw_name}");
    }

    let is_dir = raw_name.ends_with('/');
    let name = raw_name.trim_end_matches('/').to_string();
    Ok(TreeNode {
        name,
        is_dir,
        content,
        children: Vec::new(),
    })
}

pub fn nodes_to_entries(nodes: &[TreeNode]) -> BTreeMap<String, Entry> {
    let mut entries = BTreeMap::new();
    for node in nodes {
        entries.insert(
            node.name.clone(),
            Entry {
                name: node.name.clone(),
                is_dir: node.is_dir,
                children: nodes_to_entries(&node.children),
            },
        );
    }
    entries
}

pub fn create_tree(
    root: &Path,
    nodes: &[TreeNode],
    options: &GenerateOptions,
) -> Result<Vec<GenerateEvent>> {
    let mut events = Vec::new();
    if !options.dry_run {
        fs::create_dir_all(root).with_context(|| format!("Failed to create {}", root.display()))?;
    }
    for node in nodes {
        create_node(root, node, options, &mut events)?;
    }
    Ok(events)
}

fn create_node(
    root: &Path,
    node: &TreeNode,
    options: &GenerateOptions,
    events: &mut Vec<GenerateEvent>,
) -> Result<()> {
    let path = root.join(&node.name);
    if node.is_dir {
        if path.exists() && !path.is_dir() {
            handle_existing(&path, options)?;
            if !options.dry_run {
                remove_existing(&path)?;
            }
        }
        let exists_after_overwrite = path.exists();
        events.push(GenerateEvent {
            action: if exists_after_overwrite {
                "keep"
            } else {
                "create"
            }
            .to_string(),
            path: path.clone(),
            is_dir: true,
        });
        if !options.dry_run {
            fs::create_dir_all(&path)
                .with_context(|| format!("Failed to create {}", path.display()))?;
        }
        for child in &node.children {
            create_node(&path, child, options, events)?;
        }
        return Ok(());
    }

    let existed = path.exists();
    if existed {
        handle_existing(&path, options)?;
        if !options.dry_run {
            remove_existing(&path)?;
        }
    }
    events.push(GenerateEvent {
        action: if existed { "overwrite" } else { "create" }.to_string(),
        path: path.clone(),
        is_dir: false,
    });
    if !options.dry_run {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create {}", parent.display()))?;
        }
        fs::write(&path, node.content.as_deref().unwrap_or_default())
            .with_context(|| format!("Failed to write {}", path.display()))?;
    }
    Ok(())
}

fn handle_existing(path: &Path, options: &GenerateOptions) -> Result<()> {
    if options.force || options.dry_run {
        return Ok(());
    }
    if options.quiet || !io::stdin().is_terminal() {
        bail!(
            "{} already exists; use --force to overwrite",
            path.display()
        );
    }
    print!("{} already exists. Overwrite? [y/N] ", path.display());
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    if matches!(answer.trim().to_lowercase().as_str(), "y" | "yes") {
        Ok(())
    } else {
        bail!("Aborted by user")
    }
}

fn remove_existing(path: &Path) -> Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path).with_context(|| format!("Failed to remove {}", path.display()))
    } else {
        fs::remove_file(path).with_context(|| format!("Failed to remove {}", path.display()))
    }
}

pub fn load_template(templates_dir: &Path, name: &str) -> Result<String> {
    let candidates = [
        templates_dir.join(name),
        templates_dir.join(format!("{name}.tree")),
    ];
    for candidate in candidates {
        if candidate.exists() {
            return fs::read_to_string(&candidate)
                .with_context(|| format!("Failed to read template {}", candidate.display()));
        }
    }
    bail!(
        "Template '{name}' was not found in {}",
        templates_dir.display()
    )
}

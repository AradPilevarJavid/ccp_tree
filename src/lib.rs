use anyhow::{bail, Context, Result};
use glob::Pattern;
use ignore::WalkBuilder;
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

pub mod cli;

include!(concat!(env!("OUT_DIR"), "/builtin_templates.rs"));

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
pub struct ProjectStats {
    pub files: usize,
    pub dirs: usize,
    pub lines: usize,
    pub size: u64,
    pub estimated_tokens: usize,
}

#[derive(Debug, Clone)]
pub enum FileText {
    Text(String),
    Binary,
    TooLarge(u64),
}

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

fn root_display_name(root: &Path) -> String {
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

fn fmt_tree_with_root(root: &Path, entries: &BTreeMap<String, Entry>) -> String {
    let mut output = format!("{}/\n", root_display_name(root));
    output.push_str(&fmt_tree(entries, ""));
    output
}

fn fmt_colored_tree_entries(entries: &BTreeMap<String, Entry>, prefix: &str) -> String {
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
            out.push_str(&fmt_colored_tree_entries(&entry.children, &child_prefix));
        }
    }
    out
}

pub fn fmt_colored_tree(entries: &BTreeMap<String, Entry>, prefix: &str) -> String {
    fmt_colored_tree_entries(entries, prefix)
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

fn count_dirs(entries: &BTreeMap<String, Entry>) -> usize {
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

pub fn compute_stats(snapshot: &Snapshot, max_size: u64, max_chars: Option<u64>) -> ProjectStats {
    let file_paths = collect_files(&snapshot.tree, &snapshot.root);
    let mut lines = 0;
    let mut size = 0;

    for path in &file_paths {
        if let Ok(metadata) = fs::metadata(path) {
            size += metadata.len();
        }

        if let Ok(FileText::Text(text)) = read_file_text(path, max_size, max_chars) {
            lines += text.lines().count();
        }
    }

    ProjectStats {
        files: file_paths.len(),
        dirs: count_dirs(&snapshot.tree),
        lines,
        size,
        estimated_tokens: 0,
    }
}

/// Estimates LLM usage with the common rough heuristic of one token per four characters.
pub fn estimate_tokens(text: &str) -> usize {
    (text.chars().count() / 4).max(1)
}

pub fn read_file_text(path: &Path, max_size: u64, max_chars: Option<u64>) -> Result<FileText> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > max_size {
        return Ok(FileText::TooLarge(max_size));
    }
    let bytes = fs::read(path)?;
    match String::from_utf8(bytes) {
        Ok(text) => {
            if let Some(limit) = max_chars {
                let char_count = text.chars().count();
                if char_count > limit as usize {
                    let truncated: String = text.chars().take(limit as usize).collect();
                    return Ok(FileText::Text(format!(
                        "{}\n[truncated after {} characters]",
                        truncated, limit
                    )));
                }
            }
            Ok(FileText::Text(text))
        }
        Err(_) => Ok(FileText::Binary),
    }
}

pub fn file_content(path: &Path, max_size: u64, max_chars: Option<u64>) -> Result<String> {
    match read_file_text(path, max_size, max_chars)? {
        FileText::Text(text) => Ok(text),
        FileText::Binary => Ok("[Binary file not shown]".to_string()),
        FileText::TooLarge(size) => Ok(format!("[File too large, > {} bytes]", size)),
    }
}

fn markdown_fence_for(content: &str) -> String {
    let mut max_run = 0;
    let mut current_run = 0;
    for character in content.chars() {
        if character == '`' {
            current_run += 1;
            max_run = max_run.max(current_run);
        } else {
            current_run = 0;
        }
    }
    "`".repeat(std::cmp::max(3, max_run + 1))
}

fn format_count<T: std::fmt::Display>(value: T) -> String {
    let digits = value.to_string();
    let mut formatted = String::new();
    for (index, character) in digits.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            formatted.push(',');
        }
        formatted.push(character);
    }
    formatted.chars().rev().collect()
}

fn format_size(size: u64) -> String {
    if size < 1024 {
        let unit = if size == 1 { "byte" } else { "bytes" };
        return format!("{} {}", format_count(size), unit);
    }

    let units = ["KB", "MB", "GB", "TB"];
    let mut value = size as f64;
    let mut unit = units[0];
    for next_unit in units {
        value /= 1024.0;
        unit = next_unit;
        if value < 1024.0 {
            break;
        }
    }

    format!("{value:.1} {unit} ({} bytes)", format_count(size))
}

fn render_markdown_stats(stats: &ProjectStats) -> String {
    format!(
        "# Project Statistics\n\
         - Files: {}\n\
         - Directories: {}\n\
         - Total lines: {}\n\
         - Total size: {}\n\
         - Estimated tokens: {}\n\n",
        format_count(stats.files),
        format_count(stats.dirs),
        format_count(stats.lines),
        format_size(stats.size),
        format_count(stats.estimated_tokens),
    )
}

fn prepend_stats<F>(mut stats: ProjectStats, body: &str, render_stats: F) -> String
where
    F: Fn(&ProjectStats) -> String,
{
    for _ in 0..10 {
        let summary = render_stats(&stats);
        let output = format!("{summary}{body}");
        let estimated_tokens = estimate_tokens(&output);
        if estimated_tokens == stats.estimated_tokens {
            return output;
        }
        stats.estimated_tokens = estimated_tokens;
    }

    let summary = render_stats(&stats);
    format!("{summary}{body}")
}

pub fn render_markdown(snapshot: &Snapshot, max_size: u64, max_chars: Option<u64>) -> String {
    let tree_str = fmt_tree_with_root(&snapshot.root, &snapshot.tree);
    let tree_fence = markdown_fence_for(&tree_str);
    let mut body = format!("# Project Structure\n\n{tree_fence}\n{tree_str}{tree_fence}\n");
    body.push_str("\n# File Contents\n");
    let file_paths = collect_files(&snapshot.tree, &snapshot.root);
    for path in &file_paths {
        let relative = path.strip_prefix(&snapshot.root).unwrap_or(path);
        let content = match file_content(path, max_size, max_chars) {
            Ok(content) => content,
            Err(error) => format!("[Error reading file: {}]", error),
        };
        let fence = markdown_fence_for(&content);
        body.push_str(&format!("\n## {}\n\n{fence}\n", relative.display()));
        body.push_str(&content);
        body.push_str(&format!("\n{fence}\n"));
    }

    let stats = compute_stats(snapshot, max_size, max_chars);
    prepend_stats(stats, &body, render_markdown_stats)
}

pub fn render_raw(snapshot: &Snapshot, max_size: u64, max_chars: Option<u64>) -> String {
    let file_paths = collect_files(&snapshot.tree, &snapshot.root);
    let mut body = String::new();
    for (index, path) in file_paths.iter().enumerate() {
        let relative = path.strip_prefix(&snapshot.root).unwrap_or(path);
        let content = match file_content(path, max_size, max_chars) {
            Ok(content) => content,
            Err(error) => format!("[Error reading file: {}]", error),
        };
        body.push_str(&format!("==== {} ====\n", relative.display()));
        body.push_str(&content);
        if !content.ends_with('\n') {
            body.push('\n');
        }
        if index + 1 < file_paths.len() {
            body.push('\n');
        }
    }

    body
}

pub fn render_structure(snapshot: &Snapshot, max_size: u64, max_chars: Option<u64>) -> String {
    let tree_str = fmt_tree_with_root(&snapshot.root, &snapshot.tree);
    let fence = markdown_fence_for(&tree_str);
    let body = format!("# Project Structure\n\n{fence}\n{tree_str}{fence}\n");
    let stats = compute_stats(snapshot, max_size, max_chars);
    prepend_stats(stats, &body, render_markdown_stats)
}

pub fn render_tree_definition(snapshot: &Snapshot, max_size: u64, no_content: bool, max_chars: Option<u64>) -> String {
    render_tree_definition_entries(&snapshot.tree, &snapshot.root, 0, max_size, no_content, max_chars)
}

fn render_tree_definition_entries(
    entries: &BTreeMap<String, Entry>,
    current_path: &Path,
    depth: usize,
    max_size: u64,
    no_content: bool,
    max_chars: Option<u64>,
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
                max_chars,
            ));
            continue;
        }

        if no_content {
            out.push_str(&format!("{}{}\n", indent, entry.name));
            continue;
        }

        match read_file_text(&child_path, max_size, max_chars) {
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

    let builtin_name = name.strip_suffix(".tree").unwrap_or(name);
    if let Some((_, template)) = BUILTIN_TEMPLATES
        .iter()
        .find(|(template_name, _)| *template_name == builtin_name)
    {
        return Ok((*template).to_string());
    }

    let builtin_templates = if BUILTIN_TEMPLATES.is_empty() {
        "none".to_string()
    } else {
        BUILTIN_TEMPLATES
            .iter()
            .map(|(template_name, _)| *template_name)
            .collect::<Vec<_>>()
            .join(", ")
    };

    bail!(
        "Template '{name}' was not found in {} or built-in templates ({builtin_templates})",
        templates_dir.display()
    )
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

    #[test]
    fn structure_render_omits_file_contents_section() {
        let mut tree = BTreeMap::new();
        insert_entry(
            &mut tree,
            &[String::from("src"), String::from("main.rs")],
            false,
        );
        let snapshot = Snapshot {
            root: PathBuf::from("example-project"),
            tree,
        };

        let output = render_structure(&snapshot, 1_000);

        assert!(output.starts_with("# Project Statistics"));
        assert!(output.contains("# Project Structure"));
        assert!(output.contains("example-project/\n"));
        assert!(!output.contains("# File Contents"));
    }

    #[test]
    fn compute_stats_counts_files_directories_text_lines_and_size() {
        let root = std::env::temp_dir().join(format!("ccp-stats-test-{}", std::process::id()));
        let src_dir = root.join("src");
        let readme_path = root.join("README.md");
        let main_path = src_dir.join("main.rs");
        let image_path = root.join("image.bin");

        fs::create_dir_all(&src_dir).expect("test src dir should be created");
        fs::write(&readme_path, "one\ntwo\n").expect("readme should be written");
        fs::write(&main_path, "fn main() {}").expect("main should be written");
        fs::write(&image_path, [0, 159, 146, 150]).expect("binary should be written");

        let mut tree = BTreeMap::new();
        insert_entry(&mut tree, &[String::from("README.md")], false);
        insert_entry(&mut tree, &[String::from("image.bin")], false);
        insert_entry(
            &mut tree,
            &[String::from("src"), String::from("main.rs")],
            false,
        );
        let snapshot = Snapshot { root, tree };

        let stats = compute_stats(&snapshot, 1_000);

        assert_eq!(stats.files, 3);
        assert_eq!(stats.dirs, 1);
        assert_eq!(stats.lines, 3);
        assert_eq!(stats.size, 24);
        assert_eq!(stats.estimated_tokens, 0);

        fs::remove_dir_all(&snapshot.root).expect("test root should be removed");
    }

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
    fn markdown_fence_is_longer_than_content_backtick_runs() {
        assert_eq!(markdown_fence_for("no fences"), "```");
        assert_eq!(markdown_fence_for("```rust\nfn main() {}\n```"), "````");
        assert_eq!(markdown_fence_for("````\ninner\n````"), "`````");
    }

    #[test]
    fn markdown_render_uses_adaptive_fences_for_file_contents() {
        let root =
            std::env::temp_dir().join(format!("ccp-markdown-fence-test-{}", std::process::id()));
        let readme_path = root.join("README.md");

        fs::create_dir_all(&root).expect("test root should be created");
        fs::write(&readme_path, "before\n```rust\nfn main() {}\n```\nafter")
            .expect("test file should be written");

        let mut tree = BTreeMap::new();
        insert_entry(&mut tree, &[String::from("README.md")], false);
        let snapshot = Snapshot { root, tree };

        let output = render_markdown(&snapshot, 1_000);

        assert!(output.starts_with("# Project Statistics"));
        assert!(output.contains("- Estimated tokens: "));
        assert!(output.contains("## README.md\n\n````\nbefore\n```rust"));
        assert!(output.contains("```\nafter\n````\n"));

        fs::remove_dir_all(&snapshot.root).expect("test root should be removed");
    }

    #[test]
    fn raw_render_outputs_only_delimited_file_contents_in_order() {
        let root = std::env::temp_dir().join(format!("ccp-raw-test-{}", std::process::id()));
        let src_dir = root.join("src");
        let readme_path = root.join("README.md");
        let main_path = src_dir.join("main.rs");

        fs::create_dir_all(&src_dir).expect("test src dir should be created");
        fs::write(&readme_path, "readme\n").expect("readme should be written");
        fs::write(&main_path, "fn main() {}").expect("main should be written");

        let mut tree = BTreeMap::new();
        insert_entry(
            &mut tree,
            &[String::from("src"), String::from("main.rs")],
            false,
        );
        insert_entry(&mut tree, &[String::from("README.md")], false);
        let snapshot = Snapshot { root, tree };

        let output = render_raw(&snapshot, 1_000);

        assert_eq!(
            output,
            "==== README.md ====\nreadme\n\n==== src/main.rs ====\nfn main() {}\n"
        );
        assert!(!output.contains("Project Statistics"));
        assert!(!output.contains("# Project Structure"));
        assert!(!output.contains("```"));

        fs::remove_dir_all(&snapshot.root).expect("test root should be removed");
    }

    #[test]
    fn tree_definition_render_does_not_include_project_statistics() {
        let root =
            std::env::temp_dir().join(format!("ccp-tree-definition-test-{}", std::process::id()));
        let readme_path = root.join("README.md");

        fs::create_dir_all(&root).expect("test root should be created");
        fs::write(&readme_path, "readme").expect("readme should be written");

        let mut tree = BTreeMap::new();
        insert_entry(&mut tree, &[String::from("README.md")], false);
        let snapshot = Snapshot { root, tree };

        let output = render_tree_definition(&snapshot, 1_000, false);

        assert_eq!(output, "README.md: readme\n");
        assert!(!output.contains("Project Statistics"));
        assert!(!output.contains("Estimated tokens"));

        fs::remove_dir_all(&snapshot.root).expect("test root should be removed");
    }

    #[test]
    fn load_template_falls_back_to_builtin_templates() {
        let template = load_template(Path::new("missing-template-dir"), "python")
            .expect("python should be available as a built-in template");

        assert!(template.contains("main.py"));
        assert!(template.contains("Hello from ccp"));
    }
}

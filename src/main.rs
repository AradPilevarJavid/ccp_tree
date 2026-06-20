use anyhow::{Context, Result};
use clap::Parser;
use ignore::WalkBuilder;
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// CLI arguments
// ---------------------------------------------------------------------------
#[derive(Parser)]
#[command(name = "copy-project")]
#[command(about = "Copy project structure and file contents (like copy4AI)")]
struct Args {
    /// Root directory to scan (defaults to current directory)
    #[arg(default_value = ".")]
    root: PathBuf,

    /// Write output to this file instead of stdout
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Copy the result to the system clipboard (requires the 'clipboard' feature)
    #[cfg(feature = "clipboard")]
    #[arg(long)]
    clipboard: bool,

    /// Include hidden files and directories (those starting with a dot)
    #[arg(long)]
    include_hidden: bool,

    /// Do not respect .gitignore / .ignore files
    #[arg(long)]
    no_ignore: bool,

    /// Glob patterns to exclude (can be repeated)
    #[arg(short = 'e', long = "exclude")]
    exclude: Vec<String>,

    /// Maximum file size in bytes (larger files are skipped)
    #[arg(long, default_value = "1048576")]
    max_size: u64,
}

// ---------------------------------------------------------------------------
// Tree representation
// ---------------------------------------------------------------------------
#[derive(Debug)]
struct Entry {
    name: String,
    is_dir: bool,
    children: BTreeMap<String, Entry>,
}

impl Entry {
    fn new(name: String, is_dir: bool) -> Self {
        Self {
            name,
            is_dir,
            children: BTreeMap::new(),
        }
    }
}

/// Insert a relative path into the tree. `components` are the path segments.
fn insert_entry(root: &mut BTreeMap<String, Entry>, components: &[String], is_dir: bool) {
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
        // leaf – if we arrived here, the last component determines type
        entry.is_dir = is_dir;
    }
}

// ---------------------------------------------------------------------------
// Tree formatting
// ---------------------------------------------------------------------------
fn fmt_tree(entries: &BTreeMap<String, Entry>, prefix: &str, is_last: bool) -> String {
    let mut out = String::new();
    let entries_vec: Vec<&Entry> = entries.values().collect();
    let count = entries_vec.len();
    for (i, entry) in entries_vec.iter().enumerate() {
        let last_child = i == count - 1;
        let connector = if is_last && i == 0 {
            if last_child {
                "└── "
            } else {
                "├── "
            }
        } else {
            if last_child {
                "└── "
            } else {
                "├── "
            }
        };
        // Use the prefix logic from the classic tree algorithm
        let current_prefix = if is_last && i == 0 {
            prefix.to_string()
        } else {
            format!("{}{}", prefix, if is_last { "    " } else { "│   " })
        };

        // Actually, correct algorithm:
        let (connector, children_prefix) = if last_child {
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
            let child_prefix = if last_child {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };
            out.push_str(&fmt_tree(&entry.children, &child_prefix, last_child));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// File content retrieval
// ---------------------------------------------------------------------------
fn file_content(path: &Path, max_size: u64) -> Result<String> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > max_size {
        return Ok(format!("[File too large, > {} bytes]", max_size));
    }
    let bytes = fs::read(path)?;
    // Check if it looks like text (valid UTF-8)
    match String::from_utf8(bytes) {
        Ok(s) => Ok(s),
        Err(_) => Ok("[Binary file not shown]".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Collect file paths (depth-first, same order as the tree)
// ---------------------------------------------------------------------------
fn collect_files(entries: &BTreeMap<String, Entry>, current_path: &Path) -> Vec<PathBuf> {
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

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
fn main() -> Result<()> {
    let args = Args::parse();

    // Build the walker
    let mut builder = WalkBuilder::new(&args.root);
    builder
        .hidden(!args.include_hidden)   // skip hidden unless explicitly included
        .git_ignore(!args.no_ignore)
        .ignore(!args.no_ignore)
        .max_filesize(args.max_size)
        .follow_links(false);

    // Add custom exclusion patterns as override ignores
    for pattern in &args.exclude {
        builder.add_custom_ignore_filename(&pattern);
        // Or better: we can use filter_entry to apply glob matching.
        // The above adds a custom ignore *file* which is not what we want.
        // So we'll use filter_entry with a glob check instead.
        // Let's redo: remove the add_custom_ignore_filename line, and
        // apply the exclusion in the loop below after collecting entries.
        // For simplicity, we'll manually filter while walking.
    }

    // We'll walk manually and build our tree, applying glob exclusion later.
    // However, the ignore crate handles standard ignores. We'll use filter_entry
    // for custom globs.
    let exclude_patterns: Vec<glob::Pattern> = args
        .exclude
        .iter()
        .map(|p| glob::Pattern::new(p))
        .collect::<Result<Vec<_>, _>>()
        .context("Invalid exclusion pattern")?;

    // Override filter_entry to apply our custom glob patterns
    builder.filter_entry(move |entry| {
        let path = entry.path();
        // Keep if it's the root
        if path == args.root {
            return true;
        }
        // Relative path for matching
        let relative = path.strip_prefix(&args.root).unwrap_or(path);
        let rel_str = relative.to_string_lossy();
        for pat in &exclude_patterns {
            if pat.matches(&rel_str) {
                return false;
            }
        }
        true
    });

    let walker = builder.build();

    // Build the tree
    let mut tree_root: BTreeMap<String, Entry> = BTreeMap::new();
    for result in walker {
        let entry = result?;
        let path = entry.path();
        if path == args.root {
            continue; // skip the root itself
        }
        let relative = path.strip_prefix(&args.root).unwrap();
        let components: Vec<String> = relative
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        insert_entry(&mut tree_root, &components, is_dir);
    }

    // Produce output string
    let tree_str = fmt_tree(&tree_root, "", false);
    let mut output = format!("# Project Structure\n\n```\n{}```\n", tree_str);

    // File contents section
    output.push_str("\n# File Contents\n");
    let file_paths = collect_files(&tree_root, &args.root);
    for path in &file_paths {
        let relative = path.strip_prefix(&args.root).unwrap_or(path);
        let rel_display = relative.display();
        output.push_str(&format!("\n## {}\n\n```\n", rel_display));
        match file_content(path, args.max_size) {
            Ok(content) => {
                output.push_str(&content);
            }
            Err(e) => {
                output.push_str(&format!("[Error reading file: {}]", e));
            }
        }
        output.push_str("\n```\n");
    }

    // Output
    #[cfg(feature = "clipboard")]
    if args.clipboard {
        let mut clipboard = arboard::Clipboard::new()
            .context("Failed to access clipboard")?;
        clipboard.set_text(&output)
            .context("Failed to set clipboard text")?;
        println!("Output copied to clipboard.");
        return Ok(());
    }

    // Write to file or stdout
    if let Some(out_path) = args.output {
        fs::write(&out_path, &output)
            .context("Failed to write output file")?;
    } else {
        io::stdout()
            .write_all(output.as_bytes())
            .context("Failed to write to stdout")?;
    }

    Ok(())
}
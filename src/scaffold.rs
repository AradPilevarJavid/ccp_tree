use crate::parser::TreeNode;
use anyhow::{bail, Context, Result};
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

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

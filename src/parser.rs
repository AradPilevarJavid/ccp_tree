use crate::tree::Entry;
use anyhow::{bail, Result};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub name: String,
    pub is_dir: bool,
    pub content: Option<String>,
    pub children: Vec<TreeNode>,
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

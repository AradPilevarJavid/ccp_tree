use crate::file::{is_single_line, read_file_text_with_options, ContentOptions, FileText};
use crate::tree::{Entry, Snapshot};
use std::collections::BTreeMap;
use std::path::Path;

pub fn render_tree_definition(
    snapshot: &Snapshot,
    max_size: u64,
    no_content: bool,
    max_chars: Option<u64>,
) -> String {
    render_tree_definition_with_options(
        snapshot,
        max_size,
        no_content,
        &ContentOptions {
            max_chars,
            ..ContentOptions::default()
        },
    )
}

pub fn render_tree_definition_with_options(
    snapshot: &Snapshot,
    max_size: u64,
    no_content: bool,
    options: &ContentOptions,
) -> String {
    render_tree_definition_entries(
        &snapshot.tree,
        &snapshot.root,
        0,
        max_size,
        no_content,
        options,
    )
}

fn render_tree_definition_entries(
    entries: &BTreeMap<String, Entry>,
    current_path: &Path,
    depth: usize,
    max_size: u64,
    no_content: bool,
    options: &ContentOptions,
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
                options,
            ));
            continue;
        }

        if no_content {
            out.push_str(&format!("{}{}\n", indent, entry.name));
            continue;
        }

        match read_file_text_with_options(&child_path, max_size, options) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::insert_entry;
    use std::collections::BTreeMap;
    use std::fs;

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

        let output = render_tree_definition(&snapshot, 1_000, false, None);

        assert_eq!(output, "README.md: readme\n");
        assert!(!output.contains("Project Statistics"));
        assert!(!output.contains("Estimated tokens"));

        fs::remove_dir_all(&snapshot.root).expect("test root should be removed");
    }
}

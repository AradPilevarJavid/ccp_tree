use crate::file::{
    format_bytes, format_metadata, inspect_file_with_options, is_single_line, ContentOptions,
    InspectedFile,
};
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

        match inspect_file_with_options(&child_path, max_size, options) {
            Ok(InspectedFile::Text(text)) if text.is_empty() => {
                out.push_str(&format!("{}{}\n", indent, entry.name))
            }
            Ok(InspectedFile::Text(text)) if is_single_line(&text) => {
                out.push_str(&format!("{}{}: {}\n", indent, entry.name, text));
            }
            Ok(InspectedFile::Text(text)) => {
                out.push_str(&format!("{}{}:|\n", indent, entry.name));
                for line in text.lines() {
                    out.push_str(&format!("{}  {}\n", indent, line));
                }
                if text.ends_with('\n') {
                    out.push_str(&format!("{}  \n", indent));
                }
            }
            Ok(InspectedFile::Binary(metadata)) => out.push_str(&format!(
                "{}{}: <binary file; {}>\n",
                indent,
                entry.name,
                format_metadata(&metadata),
            )),
            Ok(InspectedFile::TooLarge { metadata, limit }) => out.push_str(&format!(
                "{}{}: <file too large; {}; limit: {}>\n",
                indent,
                entry.name,
                format_metadata(&metadata),
                format_bytes(limit),
            )),
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
        assert!(!output.contains("Tokens (o200k_base)"));

        fs::remove_dir_all(&snapshot.root).expect("test root should be removed");
    }

    #[test]
    fn tree_definition_render_includes_binary_metadata() {
        let root =
            std::env::temp_dir().join(format!("ccp-tree-binary-test-{}", std::process::id()));
        let image_path = root.join("image.data");
        let png = [
            0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, b'I', b'H',
            b'D', b'R',
        ];

        fs::create_dir_all(&root).expect("test root should be created");
        fs::write(&image_path, png).expect("test image should be written");

        let mut tree = BTreeMap::new();
        insert_entry(&mut tree, &[String::from("image.data")], false);
        let snapshot = Snapshot { root, tree };

        let output = render_tree_definition(&snapshot, 1_000, false, None);

        assert_eq!(
            output,
            "image.data: <binary file; MIME: image/png; size: 16 bytes; detected extension: png>\n"
        );

        fs::remove_dir_all(&snapshot.root).expect("test root should be removed");
    }
}

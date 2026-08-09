use crate::file::{file_content_with_options, markdown_fence_for, ContentOptions};
use crate::stats::{
    compute_stats_with_options, estimate_tokens, format_count, format_size, ProjectStats,
};
use crate::tree::{collect_files, fmt_tree_with_root, Snapshot};

fn render_markdown_stats(stats: &ProjectStats) -> String {
    format!(
        "# Project Statistics\n\
         - Files: {}\n\
         - Directories: {}\n\
         - Total lines: {}\n\
         - Total size: {}\n\
         - Tokens (o200k_base): {}\n\n",
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
    render_markdown_with_options(
        snapshot,
        max_size,
        &ContentOptions {
            max_chars,
            ..ContentOptions::default()
        },
    )
}

pub fn render_markdown_with_options(
    snapshot: &Snapshot,
    max_size: u64,
    options: &ContentOptions,
) -> String {
    let tree_str = fmt_tree_with_root(&snapshot.root, &snapshot.tree);
    let tree_fence = markdown_fence_for(&tree_str);
    let mut body = format!("# Project Structure\n\n{tree_fence}\n{tree_str}{tree_fence}\n");
    body.push_str("\n# File Contents\n");
    let file_paths = collect_files(&snapshot.tree, &snapshot.root);
    for path in &file_paths {
        let relative = path.strip_prefix(&snapshot.root).unwrap_or(path);
        let content = match file_content_with_options(path, max_size, options) {
            Ok(content) => content,
            Err(error) => format!("[Error reading file: {}]", error),
        };
        let fence = markdown_fence_for(&content);
        body.push_str(&format!("\n## {}\n\n{fence}\n", relative.display()));
        body.push_str(&content);
        body.push_str(&format!("\n{fence}\n"));
    }

    let stats = compute_stats_with_options(snapshot, max_size, options);
    prepend_stats(stats, &body, render_markdown_stats)
}

pub fn render_raw(snapshot: &Snapshot, max_size: u64, max_chars: Option<u64>) -> String {
    render_raw_with_options(
        snapshot,
        max_size,
        &ContentOptions {
            max_chars,
            ..ContentOptions::default()
        },
    )
}

pub fn render_raw_with_options(
    snapshot: &Snapshot,
    max_size: u64,
    options: &ContentOptions,
) -> String {
    let file_paths = collect_files(&snapshot.tree, &snapshot.root);
    let mut body = String::new();
    for (index, path) in file_paths.iter().enumerate() {
        let relative = path.strip_prefix(&snapshot.root).unwrap_or(path);
        let content = match file_content_with_options(path, max_size, options) {
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
    render_structure_with_options(
        snapshot,
        max_size,
        &ContentOptions {
            max_chars,
            ..ContentOptions::default()
        },
    )
}

pub fn render_structure_with_options(
    snapshot: &Snapshot,
    max_size: u64,
    options: &ContentOptions,
) -> String {
    let tree_str = fmt_tree_with_root(&snapshot.root, &snapshot.tree);
    let fence = markdown_fence_for(&tree_str);
    let body = format!("# Project Structure\n\n{fence}\n{tree_str}{fence}\n");
    let stats = compute_stats_with_options(snapshot, max_size, options);
    prepend_stats(stats, &body, render_markdown_stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{insert_entry, Snapshot};
    use std::collections::BTreeMap;
    use std::fs;

    #[test]
    fn structure_render_omits_file_contents_section() {
        let mut tree = BTreeMap::new();
        insert_entry(
            &mut tree,
            &[String::from("src"), String::from("main.rs")],
            false,
        );
        let snapshot = Snapshot {
            root: "example-project".into(),
            tree,
        };

        let output = render_structure(&snapshot, 1_000, None);

        assert!(output.starts_with("# Project Statistics"));
        assert!(output.contains("# Project Structure"));
        assert!(output.contains("example-project/\n"));
        assert!(!output.contains("# File Contents"));
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

        let output = render_markdown(&snapshot, 1_000, None);

        assert!(output.starts_with("# Project Statistics"));
        let token_count = output
            .lines()
            .find_map(|line| line.strip_prefix("- Tokens (o200k_base): "))
            .expect("statistics should contain an o200k token count")
            .replace(',', "")
            .parse::<usize>()
            .expect("token count should be a number");
        assert_eq!(token_count, estimate_tokens(&output));
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

        let output = render_raw(&snapshot, 1_000, None);

        assert_eq!(
            output,
            "==== README.md ====\nreadme\n\n==== src/main.rs ====\nfn main() {}\n"
        );
        assert!(!output.contains("Project Statistics"));
        assert!(!output.contains("# Project Structure"));
        assert!(!output.contains("```"));

        fs::remove_dir_all(&snapshot.root).expect("test root should be removed");
    }
}

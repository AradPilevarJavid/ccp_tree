use crate::file::{read_file_text_with_options, ContentOptions, FileText};
use crate::tree::{collect_files, count_dirs, Snapshot};
use std::fs;

#[derive(Debug, Clone)]
pub struct ProjectStats {
    pub files: usize,
    pub dirs: usize,
    pub lines: usize,
    pub size: u64,
    pub estimated_tokens: usize,
}

pub fn compute_stats(snapshot: &Snapshot, max_size: u64, max_chars: Option<u64>) -> ProjectStats {
    compute_stats_with_options(
        snapshot,
        max_size,
        &ContentOptions {
            max_chars,
            ..ContentOptions::default()
        },
    )
}

pub fn compute_stats_with_options(
    snapshot: &Snapshot,
    max_size: u64,
    options: &ContentOptions,
) -> ProjectStats {
    let file_paths = collect_files(&snapshot.tree, &snapshot.root);
    let mut lines = 0;
    let mut size = 0;

    for path in &file_paths {
        if let Ok(metadata) = fs::metadata(path) {
            size += metadata.len();
        }

        if let Ok(FileText::Text(text)) = read_file_text_with_options(path, max_size, options) {
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

pub fn estimate_tokens(text: &str) -> usize {
    (text.chars().count() / 4).max(1)
}

pub(crate) fn format_count<T: std::fmt::Display>(value: T) -> String {
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

pub(crate) fn format_size(size: u64) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{insert_entry, Snapshot};
    use std::collections::BTreeMap;
    use std::path::PathBuf;

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
        let snapshot = Snapshot {
            root: PathBuf::from(root),
            tree,
        };

        let stats = compute_stats(&snapshot, 1_000, None);

        assert_eq!(stats.files, 3);
        assert_eq!(stats.dirs, 1);
        assert_eq!(stats.lines, 3);
        assert_eq!(stats.size, 24);
        assert_eq!(stats.estimated_tokens, 0);

        fs::remove_dir_all(&snapshot.root).expect("test root should be removed");
    }
}

use crate::tree::Entry;
use std::collections::BTreeMap;

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

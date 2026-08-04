pub mod cli;
pub mod exclude;
pub mod file;
pub mod parser;
pub mod render;
pub mod scaffold;
pub mod stats;
pub mod template;
pub mod tree;

#[cfg(feature = "clipboard")]
pub mod clipboard;

#[cfg(feature = "clipboard")]
pub use clipboard::set_clipboard;
pub use exclude::{
    is_useless_dir, is_useless_dir_name, load_default_exclude_patterns, load_ignore_patterns,
    should_exclude, ExcludePattern, DEFAULT_EXCLUDES,
};
pub use file::{
    file_content, file_content_with_options, read_file_text, read_file_text_with_options,
    ContentOptions, FileText,
};
pub use parser::{nodes_to_entries, parse_tree_definition, TreeNode};
pub use render::{
    fmt_colored_tree, render_markdown, render_markdown_with_options, render_raw,
    render_raw_with_options, render_structure, render_structure_with_options,
    render_tree_definition, render_tree_definition_with_options,
};
pub use scaffold::{create_tree, GenerateEvent, GenerateOptions};
pub use stats::{compute_stats, compute_stats_with_options, estimate_tokens, ProjectStats};
pub use template::load_template;
pub use tree::{collect_files, fmt_tree, insert_entry, snapshot, Entry, Snapshot, WalkOptions};

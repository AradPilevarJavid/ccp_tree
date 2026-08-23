pub mod cli;
pub mod config;
pub mod exclude;
pub mod file;
pub mod git;
pub mod parser;
pub mod render;
pub mod scaffold;
pub mod secret;
pub mod stats;
pub mod template;
pub mod tree;

#[cfg(feature = "clipboard")]
pub mod clipboard;

#[cfg(feature = "clipboard")]
pub use clipboard::set_clipboard;
pub use config::{apply_config, load_config_files, load_default_config, Config, CONFIG_FILE_NAME};
pub use exclude::{
    is_useless_dir, is_useless_dir_name, load_default_exclude_patterns, load_ignore_patterns,
    should_exclude, ExcludePattern, DEFAULT_EXCLUDES,
};
pub use file::{
    file_content, file_content_with_options, inspect_file, inspect_file_with_options,
    read_file_text, read_file_text_with_options, ContentOptions, FileMetadata, FileText,
    InspectedFile, MimeDetection,
};
pub use git::{detect_git_metadata, GitMetadata};
pub use parser::{nodes_to_entries, parse_tree_definition, TreeNode};
pub use render::{
    fmt_colored_tree, render_markdown, render_markdown_with_options, render_raw,
    render_raw_with_options, render_structure, render_structure_with_options,
    render_tree_definition, render_tree_definition_with_options,
};
pub use scaffold::{create_tree, GenerateEvent, GenerateOptions};
pub use secret::{scan_snapshot_for_secrets, SecretFinding};
pub use stats::{compute_stats, compute_stats_with_options, estimate_tokens, ProjectStats};
pub use template::{list_templates, load_template, AvailableTemplate, TemplateSource};
pub use tree::{collect_files, fmt_tree, insert_entry, snapshot, Entry, Snapshot, WalkOptions};

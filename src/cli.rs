use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "ccp")]
#[command(version)]
#[command(about = "Snapshot, scaffold, and blueprint projects")]
#[command(long_about = "\
Create AI-friendly project snapshots, inspect directory structures, produce \
reusable .tree blueprints, and scaffold projects from those blueprints.")]
#[command(after_long_help = r#"
EXAMPLES:
  ccp
      Snapshot the current directory as Markdown.

  ccp ./my-project --structure
      Print the project tree and statistics without file contents.

  ccp ./my-project --raw --max-chars 4000
      Emit delimited raw contents, truncating each file to 4,000 characters.

  ccp reverse ./my-project -o my-project.tree
      Save a reusable .tree blueprint.

  ccp create ./new-project --template rust
      Create a project from a built-in template.

  ccp generate ./new-project --input project.tree
      Create files from a .tree definition.

TREE FORMAT:
  Directories end with '/'. File content follows ':'; multiline content uses ':|'
  and is indented by two additional spaces.

  src/
    main.rs:|
      fn main() {
          println!(\"Hello\");
      }

SEE ALSO:
  ccp-generate(1), ccp-create(1), ccp-reverse(1)"#)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Root directory to scan (defaults to current directory)
    #[arg(default_value = ".")]
    pub root: PathBuf,

    /// Write output to this file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Copy the result to the system clipboard (requires the 'clipboard' feature)
    #[cfg(feature = "clipboard")]
    #[arg(long, short = 'c')]
    pub clipboard: bool,

    /// Include hidden files and directories (those starting with a dot)
    #[arg(long)]
    pub include_hidden: bool,

    /// Do not respect .gitignore / .ignore files
    #[arg(long)]
    pub no_ignore: bool,

    /// Include generated/cache folders that are skipped by default
    #[arg(short = 'a', long = "all")]
    pub all: bool,

    /// Glob patterns to exclude (can be repeated)
    #[arg(short = 'e', long = "exclude")]
    pub exclude: Vec<String>,

    /// Maximum file size in bytes (larger files are skipped)
    #[arg(long, default_value = "1048576")]
    pub max_size: u64,

    /// Limit the number of characters read from each file (for AI context windows)
    #[arg(long)]
    pub max_chars: Option<u64>,

    /// Include only the first number of lines from each file
    #[arg(long, conflicts_with = "tail")]
    pub head: Option<usize>,

    /// Include only the last number of lines from each file
    #[arg(long, conflicts_with = "head")]
    pub tail: Option<usize>,

    /// Apply character limits from the end of each file
    #[arg(short = 'r', requires = "max_chars")]
    pub from_end: bool,

    /// Count o200k_base tokens and exit
    #[arg(short = 't', long = "tokens")]
    pub tokens: bool,

    /// Omit file contents from reverse .tree output
    #[arg(long)]
    pub no_content: bool,

    /// Output only the project structure, without file contents
    #[arg(long, short = 's')]
    pub structure: bool,

    /// Output a reusable .tree definition instead of Markdown
    #[arg(long)]
    pub reverse: bool,

    /// Output raw concatenated file contents (no Markdown, no tree)
    #[arg(long)]
    pub raw: bool,

    /// Preview filesystem operations or scan output only
    #[arg(long)]
    pub dry_run: bool,

    /// Print more progress details
    #[arg(long, short)]
    pub verbose: bool,

    /// Suppress non-essential messages
    #[arg(long, short)]
    pub quiet: bool,

    /// Disable warnings for potential credentials in exported file contents
    #[arg(long)]
    pub no_secret_scan: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create files and directories from an indented .tree definition
    Generate(GenerateCommand),
    /// Alias for generate
    Create(GenerateCommand),
    /// Output a reusable .tree definition for a directory
    Reverse(ReverseCommand),
}

#[derive(Parser)]
pub struct GenerateCommand {
    /// Target directory to create into
    #[arg(default_value = ".")]
    pub root: PathBuf,

    /// Read structure from a .tree file
    #[arg(short, long)]
    pub input: Option<PathBuf>,

    /// Load a .tree file from the templates directory
    #[arg(long)]
    pub template: Option<String>,

    /// Directory containing .tree templates
    #[arg(long, default_value = "templates")]
    pub templates_dir: PathBuf,

    /// Inline .tree structure text
    #[arg(long)]
    pub inline: Option<String>,

    /// Overwrite existing files without prompting
    #[arg(long)]
    pub force: bool,

    /// Preview only, without writing files
    #[arg(long)]
    pub dry_run: bool,

    /// Print more progress details
    #[arg(long, short)]
    pub verbose: bool,

    /// Suppress non-essential messages and decline overwrite prompts
    #[arg(long, short)]
    pub quiet: bool,
}

#[derive(Parser)]
pub struct ReverseCommand {
    /// Root directory to scan
    #[arg(default_value = ".")]
    pub root: PathBuf,

    /// Write output to this file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Copy the result to the system clipboard (requires the 'clipboard' feature)
    #[cfg(feature = "clipboard")]
    #[arg(long, short = 'c')]
    pub clipboard: bool,

    /// Include hidden files and directories
    #[arg(long)]
    pub include_hidden: bool,

    /// Do not respect .gitignore / .ignore files
    #[arg(long)]
    pub no_ignore: bool,

    /// Include generated/cache folders that are skipped by default
    #[arg(short = 'a', long = "all")]
    pub all: bool,

    /// Glob patterns to exclude (can be repeated)
    #[arg(short = 'e', long = "exclude")]
    pub exclude: Vec<String>,

    /// Maximum file size in bytes
    #[arg(long, default_value = "1048576")]
    pub max_size: u64,

    /// Limit the number of characters read from each file (for AI context windows)
    #[arg(long)]
    pub max_chars: Option<u64>,

    /// Include only the first number of lines from each file
    #[arg(long, conflicts_with = "tail")]
    pub head: Option<usize>,

    /// Include only the last number of lines from each file
    #[arg(long, conflicts_with = "head")]
    pub tail: Option<usize>,

    /// Apply character limits from the end of each file
    #[arg(short = 'r', requires = "max_chars")]
    pub from_end: bool,

    /// Count o200k_base tokens and exit
    #[arg(short = 't', long = "tokens")]
    pub tokens: bool,

    /// Omit file contents
    #[arg(long)]
    pub no_content: bool,

    /// Preview tree in color before emitting .tree output
    #[arg(long)]
    pub dry_run: bool,

    /// Print more progress details
    #[arg(long, short)]
    pub verbose: bool,

    /// Suppress non-essential messages
    #[arg(long, short)]
    pub quiet: bool,

    /// Disable warnings for potential credentials in exported file contents
    #[arg(long)]
    pub no_secret_scan: bool,
}

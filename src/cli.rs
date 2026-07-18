use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "ccp")]
#[command(version)]
#[command(about = "Snapshot, scaffold, and blueprint projects")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[arg(default_value = ".")]
    pub root: PathBuf,

    #[arg(short, long)]
    pub output: Option<PathBuf>,

    #[cfg(feature = "clipboard")]
    #[arg(long, short = 'c')]
    pub clipboard: bool,

    #[arg(long)]
    pub include_hidden: bool,

    #[arg(long)]
    pub no_ignore: bool,

    #[arg(short = 'a', long = "all")]
    pub all: bool,

    #[arg(short = 'e', long = "exclude")]
    pub exclude: Vec<String>,

    #[arg(long, default_value = "1048576")]
    pub max_size: u64,

    #[arg(long)]
    pub no_content: bool,

    #[arg(long, short = 's')]
    pub structure: bool,

    #[arg(long)]
    pub reverse: bool,

    #[arg(long, short = 'r')]
    pub raw: bool,

    #[arg(long)]
    pub dry_run: bool,

    #[arg(long, short)]
    pub verbose: bool,

    #[arg(long, short)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum Command {
    Generate(GenerateCommand),
    Create(GenerateCommand),
    Reverse(ReverseCommand),
}

#[derive(Parser)]
pub struct GenerateCommand {
    #[arg(default_value = ".")]
    pub root: PathBuf,
    #[arg(short, long)]
    pub input: Option<PathBuf>,
    #[arg(long)]
    pub template: Option<String>,
    #[arg(long, default_value = "templates")]
    pub templates_dir: PathBuf,
    #[arg(long)]
    pub inline: Option<String>,
    #[arg(long)]
    pub force: bool,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long, short)]
    pub verbose: bool,
    #[arg(long, short)]
    pub quiet: bool,
}

#[derive(Parser)]
pub struct ReverseCommand {
    #[arg(default_value = ".")]
    pub root: PathBuf,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    #[cfg(feature = "clipboard")]
    #[arg(long, short = 'c')]
    pub clipboard: bool,
    #[arg(long)]
    pub include_hidden: bool,
    #[arg(long)]
    pub no_ignore: bool,
    #[arg(short = 'a', long = "all")]
    pub all: bool,
    #[arg(short = 'e', long = "exclude")]
    pub exclude: Vec<String>,
    #[arg(long, default_value = "1048576")]
    pub max_size: u64,
    #[arg(long)]
    pub no_content: bool,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long, short)]
    pub verbose: bool,
    #[arg(long, short)]
    pub quiet: bool,
}

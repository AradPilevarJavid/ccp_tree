use anstream::println as aprintln;
use anyhow::{Context, Result};
use ccp_tree::{
    create_tree, fmt_colored_tree, load_template, nodes_to_entries, parse_tree_definition,
    render_markdown, render_raw, render_raw_structure, render_structure, render_tree_definition,
    snapshot, GenerateOptions, Snapshot, WalkOptions,
};
use clap::{Parser, Subcommand};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "ccp")]
#[command(version)]
#[command(about = "Snapshot, scaffold, and blueprint projects")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Root directory to scan (defaults to current directory)
    #[arg(default_value = ".")]
    root: PathBuf,

    /// Write output to this file instead of stdout
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Copy the result to the system clipboard (requires the 'clipboard' feature)
    #[cfg(feature = "clipboard")]
    #[arg(long, short = 'c')]
    clipboard: bool,

    /// Include hidden files and directories (those starting with a dot)
    #[arg(long)]
    include_hidden: bool,

    /// Do not respect .gitignore / .ignore files
    #[arg(long)]
    no_ignore: bool,

    /// Include generated/cache folders that are skipped by default
    #[arg(short = 'a', long = "all")]
    all: bool,

    /// Glob patterns to exclude (can be repeated)
    #[arg(short = 'e', long = "exclude")]
    exclude: Vec<String>,

    /// Maximum file size in bytes (larger files are skipped)
    #[arg(long, default_value = "1048576")]
    max_size: u64,

    /// Omit file contents from reverse .tree output
    #[arg(long)]
    no_content: bool,

    /// Output only the project structure, without file contents
    #[arg(long, short = 's')]
    structure: bool,

    /// Output a reusable .tree definition instead of Markdown
    #[arg(long)]
    reverse: bool,

    /// Output raw concatenated file contents (no Markdown, no tree)
    #[arg(long, short = 'r')]
    raw: bool,

    /// Preview filesystem operations or scan output only
    #[arg(long)]
    dry_run: bool,

    /// Print more progress details
    #[arg(long, short)]
    verbose: bool,

    /// Suppress non-essential messages
    #[arg(long, short)]
    quiet: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Create files and directories from an indented .tree definition
    Generate(GenerateCommand),
    /// Alias for generate
    Create(GenerateCommand),
    /// Output a reusable .tree definition for a directory
    Reverse(ReverseCommand),
}

#[derive(Parser)]
struct GenerateCommand {
    /// Target directory to create into
    #[arg(default_value = ".")]
    root: PathBuf,

    /// Read structure from a .tree file
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Load a .tree file from the templates directory
    #[arg(long)]
    template: Option<String>,

    /// Directory containing .tree templates
    #[arg(long, default_value = "templates")]
    templates_dir: PathBuf,

    /// Inline .tree structure text
    #[arg(long)]
    inline: Option<String>,

    /// Overwrite existing files without prompting
    #[arg(long)]
    force: bool,

    /// Preview only, without writing files
    #[arg(long)]
    dry_run: bool,

    /// Print more progress details
    #[arg(long, short)]
    verbose: bool,

    /// Suppress non-essential messages and decline overwrite prompts
    #[arg(long, short)]
    quiet: bool,
}

#[derive(Parser)]
struct ReverseCommand {
    /// Root directory to scan
    #[arg(default_value = ".")]
    root: PathBuf,

    /// Write output to this file instead of stdout
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Copy the result to the system clipboard (requires the 'clipboard' feature)
    #[cfg(feature = "clipboard")]
    #[arg(long, short = 'c')]
    clipboard: bool,

    /// Include hidden files and directories
    #[arg(long)]
    include_hidden: bool,

    /// Do not respect .gitignore / .ignore files
    #[arg(long)]
    no_ignore: bool,

    /// Include generated/cache folders that are skipped by default
    #[arg(short = 'a', long = "all")]
    all: bool,

    /// Glob patterns to exclude (can be repeated)
    #[arg(short = 'e', long = "exclude")]
    exclude: Vec<String>,

    /// Maximum file size in bytes
    #[arg(long, default_value = "1048576")]
    max_size: u64,

    /// Omit file contents
    #[arg(long)]
    no_content: bool,

    /// Preview tree in color before emitting .tree output
    #[arg(long)]
    dry_run: bool,

    /// Print more progress details
    #[arg(long, short)]
    verbose: bool,

    /// Suppress non-essential messages
    #[arg(long, short)]
    quiet: bool,
}

#[cfg(feature = "clipboard")]
fn set_clipboard(text: &str) -> Result<()> {
    use std::io::Write;

    // On Linux, try wl-copy (Wayland) and xclip (X11) first.
    // These force the active selection, bypassing KDE's clipboard history.
    if cfg!(target_os = "linux") {
        if let Ok(mut child) = std::process::Command::new("wl-copy")
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(text.as_bytes())?;
            }
            let status = child.wait()?;
            if status.success() {
                return Ok(());
            }
        }
        if let Ok(mut child) = std::process::Command::new("xclip")
            .args(["-selection", "clipboard", "-i"])
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(text.as_bytes())?;
            }
            let status = child.wait()?;
            if status.success() {
                return Ok(());
            }
        }
    }
    // Fallback to arboard for non-Linux or if neither tool is installed
    let mut clipboard = arboard::Clipboard::new().context("Failed to access clipboard")?;
    clipboard
        .set_text(text)
        .context("Failed to set clipboard")?;
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Generate(command)) | Some(Command::Create(command)) => run_generate(command),
        Some(Command::Reverse(command)) => run_reverse(command),
        None => run_copy(cli),
    }
}

fn run_copy(cli: Cli) -> Result<()> {
    let options = WalkOptions {
        include_hidden: cli.include_hidden,
        no_ignore: cli.no_ignore,
        include_useless: cli.all,
        exclude: cli.exclude,
        mktree_ignore: true,
        max_size: cli.max_size,
    };
    let scan = snapshot(&cli.root, &options)?;

    if cli.dry_run {
        aprintln!("{}", fmt_colored_tree(&scan.tree, ""));
        return Ok(());
    }

    let output = if cli.raw && cli.structure {
        render_raw_structure(&scan, cli.max_size)
    } else if cli.raw {
        render_raw(&scan, cli.max_size)
    } else if cli.reverse {
        render_tree_definition(&scan, cli.max_size, cli.no_content)
    } else if cli.structure {
        render_structure(&scan, cli.max_size)
    } else {
        render_markdown(&scan, cli.max_size)
    };

    #[cfg(feature = "clipboard")]
    if cli.clipboard {
        set_clipboard(&output)?;
        if !cli.quiet {
            let message = if cli.raw {
                if cli.all {
                    "Full raw project snapshot copied to clipboard."
                } else {
                    "Raw project snapshot copied to clipboard."
                }
            } else if cli.reverse {
                if cli.all {
                    "Tree definition (all files) copied to clipboard."
                } else {
                    "Tree definition copied to clipboard."
                }
            } else if cli.structure {
                if cli.all {
                    "Full project structure copied to clipboard."
                } else {
                    "Project structure copied to clipboard."
                }
            } else {
                if cli.all {
                    "Full project snapshot copied to clipboard."
                } else {
                    "Project snapshot copied to clipboard."
                }
            };
            println!("{}", message);
        }
        return Ok(());
    }
    if cli.reverse && !cli.raw {
        let output_path = cli
            .output
            .unwrap_or_else(|| default_tree_output_path(&cli.root));
        return write_output(Some(output_path), &output);
    }

    write_output(cli.output, &output)
}

fn run_reverse(command: ReverseCommand) -> Result<()> {
    let options = WalkOptions {
        include_hidden: command.include_hidden,
        no_ignore: command.no_ignore,
        include_useless: command.all,
        exclude: command.exclude,
        mktree_ignore: true,
        max_size: command.max_size,
    };

    let scan = snapshot(&command.root, &options)?;
    if command.dry_run && !command.quiet {
        aprintln!("{}", fmt_colored_tree(&scan.tree, ""));
    }
    if command.verbose && !command.quiet {
        eprintln!("Scanned {}", command.root.display());
    }

    let output = render_tree_definition(&scan, command.max_size, command.no_content);

    #[cfg(feature = "clipboard")]
    if command.clipboard {
        set_clipboard(&output)?;
        if !command.quiet {
            let message = if command.all {
                "Tree definition (all files) copied to clipboard."
            } else {
                "Tree definition copied to clipboard."
            };
            println!("{}", message);
        }
        return Ok(());
    }

    let output_path = command
        .output
        .unwrap_or_else(|| default_tree_output_path(&command.root));
    write_output(Some(output_path), &output)
}

fn default_tree_output_path(root: &Path) -> PathBuf {
    let name = root
        .file_name()
        .filter(|name| !name.is_empty() && *name != "." && *name != "..")
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "ccp".to_string());

    PathBuf::from(format!("{name}.tree"))
}

fn run_generate(command: GenerateCommand) -> Result<()> {
    let input = load_generate_input(&command)?;
    let nodes = parse_tree_definition(&input)?;
    let options = GenerateOptions {
        force: command.force,
        dry_run: command.dry_run,
        verbose: command.verbose,
        quiet: command.quiet,
    };

    if command.dry_run && !command.quiet {
        let preview = Snapshot {
            root: command.root.clone(),
            tree: nodes_to_entries(&nodes),
        };
        aprintln!("{}", fmt_colored_tree(&preview.tree, ""));
    }

    let events = create_tree(&command.root, &nodes, &options)?;
    if (command.verbose || command.dry_run) && !command.quiet {
        for event in events {
            let suffix = if event.is_dir { "/" } else { "" };
            eprintln!("{} {}{}", event.action, event.path.display(), suffix);
        }
    }
    Ok(())
}

fn load_generate_input(command: &GenerateCommand) -> Result<String> {
    let provided = command.input.is_some() as u8
        + command.template.is_some() as u8
        + command.inline.is_some() as u8;
    if provided > 1 {
        anyhow::bail!("Use only one of --input, --template, or --inline");
    }

    if let Some(inline) = &command.inline {
        return Ok(inline.replace("\\n", "\n"));
    }
    if let Some(template) = &command.template {
        return load_template(&command.templates_dir, template);
    }
    if let Some(input) = &command.input {
        return fs::read_to_string(input)
            .with_context(|| format!("Failed to read {}", input.display()));
    }

    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .context("Failed to read .tree definition from stdin")?;
    Ok(buffer)
}

fn write_output(output_path: Option<PathBuf>, output: &str) -> Result<()> {
    if let Some(path) = output_path {
        fs::write(&path, output).with_context(|| format!("Failed to write {}", path.display()))?;
    } else {
        use std::io::Write;

        io::stdout()
            .write_all(output.as_bytes())
            .context("Failed to write to stdout")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_grouped_raw_structure_short_flags() {
        let cli = Cli::try_parse_from(["ccp", "-rs"]).expect("-rs should parse");

        assert!(cli.raw);
        assert!(cli.structure);
        assert!(!cli.reverse);
    }

    #[test]
    fn parses_structure_with_raw_long_flag() {
        let cli = Cli::try_parse_from(["ccp", "-s", "--raw"]).expect("-s --raw should parse");

        assert!(cli.raw);
        assert!(cli.structure);
        assert!(!cli.reverse);
    }
}

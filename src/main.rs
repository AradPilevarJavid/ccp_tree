use anstream::println as aprintln;
use anyhow::{Context, Result};
use ccp_tree::{
    cli::{Cli, Command, GenerateCommand, ReverseCommand},
    create_tree, fmt_colored_tree, load_template, nodes_to_entries, parse_tree_definition,
    render_markdown, render_raw, render_structure, render_tree_definition, snapshot,
    GenerateOptions, Snapshot, WalkOptions,
};
use clap::Parser;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

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
    validate_copy_options(&cli)?;

    let options = WalkOptions {
        include_hidden: cli.include_hidden || cli.all,
        no_ignore: cli.no_ignore || cli.all,
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

    let output = if cli.raw {
        render_raw(&scan, cli.max_size, cli.max_chars)
    } else if cli.reverse {
        render_tree_definition(&scan, cli.max_size, cli.no_content, cli.max_chars)
    } else if cli.structure {
        render_structure(&scan, cli.max_size, cli.max_chars)
    } else {
        render_markdown(&scan, cli.max_size, cli.max_chars)
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

fn validate_copy_options(cli: &Cli) -> Result<()> {
    if cli.raw && cli.structure {
        anyhow::bail!(
            "Options -r (raw content only) and -s (structure with statistics) cannot be used together."
        );
    }

    Ok(())
}

fn run_reverse(command: ReverseCommand) -> Result<()> {
    let options = WalkOptions {
        include_hidden: command.include_hidden || command.all,
        no_ignore: command.no_ignore || command.all,
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

    let output = render_tree_definition(&scan, command.max_size, command.no_content, command.max_chars);

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
    fn raw_and_structure_together_error() {
        let cli = Cli::try_parse_from(["ccp", "-rs"]).expect("-rs should parse");

        let error = validate_copy_options(&cli).expect_err("-rs should fail validation");

        assert_eq!(
            error.to_string(),
            "Options -r (raw content only) and -s (structure with statistics) cannot be used together."
        );
    }

    #[test]
    fn structure_with_raw_long_flag_errors() {
        let cli = Cli::try_parse_from(["ccp", "-s", "--raw"]).expect("-s --raw should parse");

        let error = validate_copy_options(&cli).expect_err("-s --raw should fail validation");

        assert_eq!(
            error.to_string(),
            "Options -r (raw content only) and -s (structure with statistics) cannot be used together."
        );
    }
}

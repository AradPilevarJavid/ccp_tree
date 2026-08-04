use anstream::println as aprintln;
use anyhow::{Context, Result};
use ccp_tree::{
    cli::{Cli, Command, GenerateCommand, ReverseCommand},
    create_tree, fmt_colored_tree, load_template, nodes_to_entries, parse_tree_definition,
    render_markdown_with_options, render_raw_with_options, render_structure_with_options,
    render_tree_definition_with_options, snapshot, ContentOptions, GenerateOptions, Snapshot,
    WalkOptions,
};
use clap::Parser;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

#[cfg(feature = "clipboard")]
use ccp_tree::set_clipboard;

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

    let content_options = ContentOptions {
        max_chars: cli.max_chars,
        head: cli.head,
        tail: cli.tail,
        from_end: cli.from_end,
    };
    let output = if cli.raw {
        render_raw_with_options(&scan, cli.max_size, &content_options)
    } else if cli.reverse {
        render_tree_definition_with_options(&scan, cli.max_size, cli.no_content, &content_options)
    } else if cli.structure {
        render_structure_with_options(&scan, cli.max_size, &content_options)
    } else {
        render_markdown_with_options(&scan, cli.max_size, &content_options)
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
        anyhow::bail!("Options --raw and -s (structure with statistics) cannot be used together.");
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

    let content_options = ContentOptions {
        max_chars: command.max_chars,
        head: command.head,
        tail: command.tail,
        from_end: command.from_end,
    };
    let output = render_tree_definition_with_options(
        &scan,
        command.max_size,
        command.no_content,
        &content_options,
    );

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
        let cli = Cli::try_parse_from(["ccp", "--raw", "-s"]).expect("--raw -s should parse");

        let error = validate_copy_options(&cli).expect_err("--raw -s should fail validation");

        assert_eq!(
            error.to_string(),
            "Options --raw and -s (structure with statistics) cannot be used together."
        );
    }

    #[test]
    fn structure_with_raw_long_flag_errors() {
        let cli = Cli::try_parse_from(["ccp", "-s", "--raw"]).expect("-s --raw should parse");

        let error = validate_copy_options(&cli).expect_err("-s --raw should fail validation");

        assert_eq!(
            error.to_string(),
            "Options --raw and -s (structure with statistics) cannot be used together."
        );
    }

    #[cfg(feature = "clipboard")]
    #[test]
    fn clipboard_and_reverse_direction_short_flags_combine() {
        let cli =
            Cli::try_parse_from(["ccp", "-cr", "--max-chars", "120"]).expect("-cr should parse");

        assert!(cli.clipboard);
        assert!(cli.from_end);
        assert_eq!(cli.max_chars, Some(120));
        assert!(!cli.raw);
    }

    #[test]
    fn head_and_tail_conflict() {
        let result = Cli::try_parse_from(["ccp", "--head", "10", "--tail", "10"]);

        assert!(result.is_err());
    }

    #[test]
    fn reverse_direction_requires_max_chars() {
        let result = Cli::try_parse_from(["ccp", "-r"]);

        assert!(result.is_err());
    }
}

use anstream::println as aprintln;
use anyhow::{Context, Result};
use ccp_tree::{
    apply_config,
    cli::{Cli, Command, GenerateCommand, ReverseCommand, TemplatesCommand},
    create_tree, estimate_tokens, fmt_colored_tree, list_templates, load_template,
    nodes_to_entries, parse_tree_definition, render_markdown_with_options, render_raw_with_options,
    render_structure_with_options, render_tree_definition_with_options, scan_snapshot_for_secrets,
    snapshot, ContentOptions, GenerateOptions, SecretFinding, Snapshot, WalkOptions,
};
#[cfg(test)]
use clap::Parser;
use clap::{
    Command as ClapCommand, CommandFactory, FromArgMatches,
};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

#[cfg(feature = "clipboard")]
use ccp_tree::set_clipboard;

const COLOR_OPTION: &str = "\x1b[36m";
const COLOR_RESET: &str = "\x1b[0m";

fn main() -> Result<()> {
    if let Some(command) = help_command_from_args() {
        print_compact_help(command);
        return Ok(());
    }
    let matches = compact_help_command(Cli::command()).get_matches();
    let mut cli = Cli::from_arg_matches(&matches)?;
    let config = ccp_tree::load_default_config()?;
    apply_config(&mut cli, &matches, &config)?;
    match cli.command {
        Some(Command::Generate(command)) | Some(Command::Create(command)) => run_generate(command),
        Some(Command::Reverse(command)) => run_reverse(command),
        Some(Command::Templates(command)) => run_templates(command),
        None => run_copy(cli),
    }
}

fn help_command_from_args() -> Option<ClapCommand> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let help_index = args.iter().position(|arg| arg == "--help" || arg == "-h")?;
    let mut command = Cli::command();

    for arg in args[..help_index]
        .iter()
        .filter(|arg| !arg.starts_with('-'))
    {
        let subcommand = command
            .find_subcommand_mut(arg)
            .map(|subcommand| subcommand.clone())?;
        command = subcommand;
    }

    Some(command)
}

fn print_compact_help(mut command: ClapCommand) {
    command = compact_help_command(command);
    let rendered = command.render_help().to_string();
    for line in rendered.lines() {
        if let Some((label, description)) = line.trim_start().split_once("  ") {
            if label.starts_with('-') || label.starts_with('[') {
                println!(
                    "  {COLOR_OPTION}{}{COLOR_RESET}: {}",
                    label.trim_end(),
                    description.trim()
                );
                continue;
            }
        }
        println!("{line}");
    }
}

/// Keep interactive help dense without changing the CLI definition used to
/// generate the full man pages.
fn compact_help_command(command: ClapCommand) -> ClapCommand {
    command
        .next_line_help(false)
        .term_width(200)
}

fn run_templates(command: TemplatesCommand) -> Result<()> {
    for template in list_templates(&command.templates_dir)? {
        match template.source {
            ccp_tree::TemplateSource::Builtin => println!("{} (built-in)", template.name),
            ccp_tree::TemplateSource::Custom(path) => {
                println!("{} (custom: {})", template.name, path.display())
            }
        }
    }
    Ok(())
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

    if cli.dry_run && !cli.tokens {
        aprintln!("{}", fmt_colored_tree(&scan.tree, ""));
        return Ok(());
    }

    let content_options = ContentOptions {
        max_chars: cli.max_chars,
        head: cli.head,
        tail: cli.tail,
        from_end: cli.from_end,
    };
    let skips_content_export = cli.tokens || cli.structure || cli.reverse && cli.no_content;
    if !cli.no_secret_scan && !skips_content_export {
        warn_about_secrets(&scan, cli.max_size, &content_options);
    }
    let output = if cli.raw {
        render_raw_with_options(&scan, cli.max_size, &content_options)
    } else if cli.reverse {
        render_tree_definition_with_options(&scan, cli.max_size, cli.no_content, &content_options)
    } else if cli.structure {
        render_structure_with_options(&scan, cli.max_size, &content_options)
    } else {
        render_markdown_with_options(&scan, cli.max_size, &content_options)
    };

    if cli.tokens {
        println!("Tokens (o200k_base): {}", estimate_tokens(&output));
        return Ok(());
    }

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
    if command.dry_run && !command.quiet && !command.tokens {
        aprintln!("{}", fmt_colored_tree(&scan.tree, ""));
    }
    if command.verbose && !command.quiet && !command.tokens {
        eprintln!("Scanned {}", command.root.display());
    }

    let content_options = ContentOptions {
        max_chars: command.max_chars,
        head: command.head,
        tail: command.tail,
        from_end: command.from_end,
    };
    if !command.no_secret_scan && !command.tokens && !command.no_content {
        warn_about_secrets(&scan, command.max_size, &content_options);
    }
    let output = render_tree_definition_with_options(
        &scan,
        command.max_size,
        command.no_content,
        &content_options,
    );

    if command.tokens {
        println!("Tokens (o200k_base): {}", estimate_tokens(&output));
        return Ok(());
    }

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

fn warn_about_secrets(snapshot: &Snapshot, max_size: u64, options: &ContentOptions) {
    let findings = scan_snapshot_for_secrets(snapshot, max_size, options);
    if findings.is_empty() {
        return;
    }

    eprintln!(
        "Warning: {} potential secret{} detected in content about to be exported:",
        findings.len(),
        if findings.len() == 1 { "" } else { "s" }
    );
    for SecretFinding { path, line, kind } in findings {
        eprintln!("  {}:{} — {}", path.display(), line, kind);
    }
    eprintln!(
        "Review the files or exclude them with --exclude. Use --no-secret-scan to disable this warning."
    );
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
        assert!(!cli.no_clipboard);
        assert!(cli.from_end);
        assert_eq!(cli.max_chars, Some(120));
        assert!(!cli.raw);
    }

    #[cfg(feature = "clipboard")]
    #[test]
    fn no_clipboard_flag_parses() {
        let cli =
            Cli::try_parse_from(["ccp", "--no-clipboard"]).expect("--no-clipboard should parse");

        assert!(cli.no_clipboard);
        assert!(!cli.clipboard);
    }

    #[test]
    fn head_and_tail_conflict() {
        let result = Cli::try_parse_from(["ccp", "--head", "10", "--tail", "10"]);

        assert!(result.is_err());
    }

    #[test]
    fn reverse_direction_requires_max_chars() {
        let matches = Cli::command()
            .try_get_matches_from(["ccp", "-r"])
            .expect("-r should parse before config is applied");
        let mut cli = Cli::from_arg_matches(&matches).expect("CLI should be constructed");

        let error = apply_config(&mut cli, &matches, &ccp_tree::Config::default())
            .expect_err("-r without max_chars should fail");

        assert!(error.to_string().contains("'from_end'"));
    }

    #[test]
    fn tokens_short_flag_parses_for_snapshot() {
        let cli = Cli::try_parse_from(["ccp", "-t"]).expect("-t should parse");

        assert!(cli.tokens);
    }

    #[test]
    fn tokens_long_flag_parses_for_reverse_subcommand() {
        let cli =
            Cli::try_parse_from(["ccp", "reverse", "--tokens"]).expect("--tokens should parse");

        let Some(Command::Reverse(command)) = cli.command else {
            panic!("reverse command should parse");
        };
        assert!(command.tokens);
    }

    #[test]
    fn templates_subcommand_accepts_custom_directory() {
        let cli =
            Cli::try_parse_from(["ccp", "templates", "--templates-dir", "./custom-templates"])
                .expect("templates command should parse");

        let Some(Command::Templates(command)) = cli.command else {
            panic!("templates command should parse");
        };
        assert_eq!(command.templates_dir, PathBuf::from("./custom-templates"));
    }
}

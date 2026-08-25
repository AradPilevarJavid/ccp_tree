use ccp_tree::cli::Cli;
use clap::{CommandFactory, Parser};
use clap_mangen::Man;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

const MANUAL_SECTIONS: &str = r#"
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
          println!("Hello");
      }

CONFIGURATION:
  ccp reads ./.ccprc and ~/.ccprc as TOML. Global settings take priority over
  local settings, and explicit command-line arguments take priority over both.

SEE ALSO:
  ccp-generate(1), ccp-create(1), ccp-reverse(1), ccp-templates(1)"#;

#[derive(Debug, Parser)]
#[command(
    name = "ccp-mangen",
    version,
    about = "Generate ccp manual pages from the CLI definition"
)]
struct Args {
    /// Directory where ccp(1) and its subcommand pages are written
    #[arg(short, long, default_value = "target/man", value_name = "DIR")]
    output_dir: PathBuf,

    /// Write only the main ccp(1) page to standard output
    #[arg(long, conflicts_with = "output_dir")]
    stdout: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run(Args::parse(), &mut io::stdout())
}

fn run(args: Args, stdout: &mut dyn Write) -> Result<(), Box<dyn std::error::Error>> {
    // Keep the executable's `--help` concise while placing the complete
    // reference sections in the generated manual page.
    let command = Cli::command().name("ccp").after_long_help(MANUAL_SECTIONS);

    if args.stdout {
        render_combined_manual(command, stdout)?;
        return Ok(());
    }

    fs::create_dir_all(&args.output_dir)?;
    clap_mangen::generate_to(command.clone(), &args.output_dir)?;

    // Keep standalone pages for direct lookup, but make `man ccp` a complete
    // manual by embedding each subcommand's page after the main page.
    let mut combined = Vec::new();
    render_combined_manual(command, &mut combined)?;
    fs::write(args.output_dir.join("ccp.1"), combined)?;

    writeln!(
        stdout,
        "Generated manual pages in {}",
        args.output_dir.display()
    )?;
    Ok(())
}

fn render_combined_manual(
    command: clap::Command,
    output: &mut dyn Write,
) -> Result<(), Box<dyn std::error::Error>> {
    Man::new(command.clone()).render(output)?;

    for subcommand in command
        .get_subcommands()
        .filter(|subcommand| subcommand.get_name() != "help")
    {
        let name = match subcommand.get_name() {
            "generate" => "ccp-generate",
            "create" => "ccp-create",
            "reverse" => "ccp-reverse",
            "templates" => "ccp-templates",
            _ => continue,
        };
        let mut rendered = Vec::new();
        Man::new(subcommand.clone().name(name)).render(&mut rendered)?;
        let body = String::from_utf8(rendered)?
            .lines()
            .filter(|line| {
                !line.starts_with(".TH ")
                    && !line.starts_with(".ie ")
                    && !line.starts_with(".el ")
            })
            .collect::<Vec<_>>()
            .join("\n");
        writeln!(output, "\n.SH SUBCOMMAND MANUAL: {}\n{}", name, body)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_target_man() {
        let args = Args::try_parse_from(["ccp-mangen"]).expect("arguments should parse");

        assert_eq!(args.output_dir, PathBuf::from("target/man"));
        assert!(!args.stdout);
    }

    #[test]
    fn stdout_renders_the_main_manual() {
        let args =
            Args::try_parse_from(["ccp-mangen", "--stdout"]).expect("arguments should parse");
        let mut output = Vec::new();

        run(args, &mut output).expect("manual should render");
        let output = String::from_utf8(output).expect("manual should be UTF-8");

        assert!(output.contains(".TH ccp 1"));
        assert!(output.contains(".SH OPTIONS"));
        assert!(output.contains(".SH EXTRA"));
        assert!(output.contains("EXAMPLES:"));
        assert!(output.contains(".SH SUBCOMMAND MANUAL: ccp-generate"));
        assert!(output.contains("Create files and directories from an indented .tree definition"));
    }
}

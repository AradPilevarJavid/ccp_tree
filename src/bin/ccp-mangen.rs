use ccp_tree::cli::Cli;
use clap::{CommandFactory, Parser};
use clap_mangen::Man;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

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
    let command = Cli::command().name("ccp");

    if args.stdout {
        Man::new(command).render(stdout)?;
        return Ok(());
    }

    fs::create_dir_all(&args.output_dir)?;
    clap_mangen::generate_to(command, &args.output_dir)?;

    writeln!(
        stdout,
        "Generated manual pages in {}",
        args.output_dir.display()
    )?;
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
    }
}

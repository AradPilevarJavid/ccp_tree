use ccp_tree::cli::Cli;
use clap::CommandFactory;
use clap_mangen::Man;
use std::env;
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = env::args().nth(1).unwrap_or_else(|| ".".to_string());
    let mut command = Cli::command();
    command = command.name("ccp");
    let man = Man::new(command);
    let mut buffer = Vec::new();
    man.render(&mut buffer)?;
    fs::write(Path::new(&out_dir).join("ccp.1"), buffer)?;
    Ok(())
}

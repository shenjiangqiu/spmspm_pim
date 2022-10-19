use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use clap_complete::Shell;

#[derive(Parser, Debug)]
#[command(author, about, version)]
pub struct Cli {
    /// subcommand
    #[clap(subcommand)]
    pub subcmd: Operation,
}

#[derive(Debug, Subcommand)]
pub enum Operation {
    /// run the simulator
    Run(RunArgs),
    /// generate the shell completion script
    Completion(CompArgs),
}
#[derive(Debug, Args)]
pub struct RunArgs {
    /// the config file path
    pub config: PathBuf,
}
#[derive(Debug, Args)]
pub struct CompArgs {
    /// the shell type
    pub shell: Shell,
}

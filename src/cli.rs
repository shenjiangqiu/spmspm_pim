use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
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
    /// analyze the result
    Analyze(AnalyzeArgs),
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
#[derive(Debug, Args)]
pub struct AnalyzeArgs {
    /// the type of analysis
    pub analyze: AnalyzeType,
    /// the config file path
    pub config: PathBuf,
}
#[derive(Debug, Clone, ValueEnum)]
pub enum AnalyzeType {
    /// run all tests
    All,
    /// run overlap test
    Overlap,
    /// run sequential test
    Sequential,
    /// run window schedule test
    Window,
    /// run split spmm test
    SplitSpmm,
}

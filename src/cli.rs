//! The command line interface of the simulator.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

/// the command line interface of the simulator
#[derive(Parser, Debug)]
#[command(author, about, version)]
pub struct Cli {
    /// subcommand
    #[clap(subcommand)]
    pub subcmd: Operation,
}

/// the subcommands of the simulator
#[derive(Debug, Subcommand)]
pub enum Operation {
    /// run the simulator
    Run(RunArgs),
    /// generate the shell completion script
    Completion(CompArgs),
    /// analyze the result
    Analyze(AnalyzeArgs),
}

/// the arguments of the run subcommand
#[derive(Debug, Args)]
pub struct RunArgs {
    /// the config file path
    pub config: PathBuf,
}

/// the arguments of the completion subcommand
#[derive(Debug, Args)]
pub struct CompArgs {
    /// the shell type
    pub shell: Shell,
}

/// the arguments of the analyze subcommand
#[derive(Debug, Args)]
pub struct AnalyzeArgs {
    /// the type of analysis
    pub analyze: AnalyzeType,
    /// the config file path
    pub config: PathBuf,
}

/// the type of analysis
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
    /// run gearbox test
    Gearbox,
}

//! The command line interface of the simulator.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

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
    /// analyze the result
    Analyze(AnalyzeArgs),
}

/// the arguments of the run subcommand
#[derive(Debug, Args)]
pub struct RunArgs {
    /// the config file path
    pub config: PathBuf,
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
    /// nnz
    Nnz,
    /// nnz using native algorithm
    NnzNative,
}

#[derive(Parser, Debug)]
#[command(name = "draw", about = "draw the speed up of spmm and gearbox")]
pub struct DrawCli {
    /// the path of the split spmm result
    pub split_result: Option<PathBuf>,
    /// the path of gearbox result
    pub gearbox_result: Option<PathBuf>,
    /// the output path of png,
    pub output: Option<PathBuf>,
}

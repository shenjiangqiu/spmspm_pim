//! The command line interface of the simulator.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Clone, ValueEnum)]
pub enum LogType {
    File,
    Stderr,
}
/// the command line interface of the simulator
#[derive(Parser, Debug)]
#[command(author, about, version)]
pub struct Cli {
    /// the log type
    #[clap(short, long)]
    pub log_type: Option<LogType>,
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
    /// draw the graphs
    Draw(DrawCli),
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

    /// run gearbox test
    GearboxParallel,

    /// nnz
    Nnz,
    /// nnz using native algorithm
    NnzNative,

    /// nnz and draw graph
    NnzDraw,
    /// the original gearbox SPMV
    GearboxOrigin,
    /// usign gearbox to perform spmm
    GearboxOriginAll,
    /// usign gearbox to perform spmm
    GearboxOriginAllV2,
    /// count the overflow
    GearboxOriginAllV2OverFlow,
    /// analyze the overflow overhead and tsv traffic
    GearboxOverflowTraffic,
    /// the channel traffic stats
    AnalyzeChannel,
    /// analyze the refined gearbox
    AnalyzeRefinedGearbox,
    /// analyze the refined gearbox with dispatching and overflow
    AnalyzeRefinedGearboxDispatchOverflow,
    /// the distribution
    AnalyzeRefinedDistribution,
    /// with mapping,
    AnalyzeRefinedNewMapping,
    /// the bank trace
    AnalyzeBankTrace,
    /// not only the max bank, but all values
    AnalyzeBankTraceAll,
    /// the new mapping: real col mapping
    AnalyzeRefinedNewMappingCycle,
    /// the mapping for real one-hot jump
    AnalyzeRealOneHotJump,
    ///
    NewAnalysis,
}

#[derive(Debug, Subcommand)]
pub enum DrawType {
    /// draw the speed up of spmm and gearbox
    SpeedUp(SpeedUpArgs),
    ///
    Split(ExecResult),
    Empty(ExecResult),
    Cycle(ExecResult),
    Gearbox(ExecResult),
    GearboxOld(ExecResult),
    GearBoxAll(ExecResult),
    GearBoxAllMultiConf(ExecResult),
    GearBoxV2(ExecResult),
    GearBoxOverflow(ExecResult),
    GearBoxTsvTraffic(ExecResult),
    TsvAndOverflow(ExecResult),
    Channel(ExecResult),
    Refined(ExecResult),
    RefinedDispatchOverflow(ExecResult),
    RefinedHybrid(HybridResult),
    RefinedDistribution(ExecResult),
    ShowCycle(ExecResult),
    ShowAvgMax(ExecResult),
    ShowDetailedAvgMax(ExecResult),
    DrawTrace(ExecResult),
    DrawTraceSplit(ExecResult),
    DrawTraceSplitAll(ExecResult),
    DrawSimpleCycle(ExecResult),
}

#[derive(Debug, Args)]
pub struct DrawCli {
    #[clap(subcommand)]
    pub subcmd: DrawType,
}

#[derive(Parser, Debug)]
#[command(name = "stopspm", about = "stop spmm experiment")]
pub struct StopCli {
    /// the port of the server
    #[clap(short, long)]
    pub port: Option<u16>,
    /// the path of the file storing the port
    #[clap(short, long)]
    pub file_path: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct SpeedUpArgs {
    /// the path of the split spmm result
    #[clap(short, long)]
    pub split_result: Option<PathBuf>,
    /// the path of gearbox result
    #[clap(short, long)]
    pub gearbox_result: Option<PathBuf>,
    /// the output path of png,
    #[clap(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct ExecResult {
    /// the path of the split spmm result
    #[clap(short, long)]
    pub result_file: Option<PathBuf>,
    /// the output path of png,
    #[clap(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct HybridResult {
    /// the path of the split spmm result
    #[clap(short, long)]
    pub pre_result: Option<PathBuf>,
    /// the path of the split spmm result
    #[clap(short, long)]
    pub new_result: Option<PathBuf>,
    /// the output path of png,
    #[clap(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Parser, Debug)]
#[command(name = "generate_matrix_graph", about = "stop spmm experiment")]
pub struct GenerateGraphCli {
    /// the port of the server
    #[clap(short, long)]
    pub graph: PathBuf,
    #[clap(short, long)]
    pub max_size: Option<usize>,
    #[clap(short, long)]
    pub output: Option<PathBuf>,
}

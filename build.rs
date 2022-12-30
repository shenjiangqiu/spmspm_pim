use std::ffi::OsString;

use clap::Command;
use clap::CommandFactory;
use clap_complete::{Generator, Shell};
#[path = "src/cli.rs"]
mod cli;

fn print_completions<G: Generator>(gen: G, cmd: &mut Command, outdir: impl Into<OsString>) {
    clap_complete::generate_to(gen, cmd, cmd.get_name().to_string(), outdir).unwrap();
}
fn generate_complete_scripts(cmd: &mut Command) {
    print_completions(Shell::Zsh, cmd, "completion_scripts");
    print_completions(Shell::Bash, cmd, "completion_scripts");
    print_completions(Shell::Fish, cmd, "completion_scripts");
    print_completions(Shell::PowerShell, cmd, "completion_scripts");
    print_completions(Shell::Elvish, cmd, "completion_scripts");
}

fn main() {
    println!("cargo:rerun-if-changed=src/cli.rs");
    let mut cmd = cli::Cli::command();
    generate_complete_scripts(&mut cmd);
    let mut cmd = cli::DrawCli::command();
    generate_complete_scripts(&mut cmd);
}

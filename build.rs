use std::ffi::OsString;
use std::fs;

use clap::Command;
use clap::CommandFactory;
use clap_complete::{Generator, Shell};
#[path = "src/cli.rs"]
mod cli;
fn print_completions<G: Generator>(gen: G, cmd: &mut Command, outdir: impl Into<OsString>) {
    clap_complete::generate_to(gen, cmd, cmd.get_name().to_string(), outdir).unwrap();
}
fn main() {
    let env_dir = std::env::var_os("OUT_DIR");
    let outdir = match env_dir {
        None => {
            println!("No OUT_DIR defined to store completion files.");
            std::process::exit(1);
        }
        Some(outdir) => outdir,
    };
    fs::create_dir_all(&outdir).unwrap();

    let mut cmd = cli::Cli::command();
    print_completions(Shell::Zsh, &mut cmd, "completion_scripts");
    print_completions(Shell::Bash, &mut cmd, "completion_scripts");
    print_completions(Shell::Fish, &mut cmd, "completion_scripts");
    print_completions(Shell::PowerShell, &mut cmd, "completion_scripts");
    print_completions(Shell::Elvish, &mut cmd, "completion_scripts");
}

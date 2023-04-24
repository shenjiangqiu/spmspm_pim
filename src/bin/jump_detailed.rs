use std::fs::File;

use spmspm_pim::analysis::remap_analyze::real_jump::RealJumpResult;

fn main() -> eyre::Result<()> {
    //[normal, ideal, from_source, my, smart]
    println!("jumptypes: normal ideal from_source my smart");
    let weighted_result: Vec<RealJumpResult> =
        serde_json::from_reader(File::open("results/realjump/weighted.json")?)?;
    println!("weighted:");
    print_result(weighted_result);
    let samebank_result: Vec<RealJumpResult> =
        serde_json::from_reader(File::open("results/realjump/same_bank.json")?)?;
    println!("samebank:");
    print_result(samebank_result);
    Ok(())
}

fn print_result(weighted_result: Vec<RealJumpResult>) {
    for r in weighted_result {
        println!();

        println!();
        print!("row_cycles: ");
        for row_cycle in r.row_cycles.into_split_iter() {
            print!("{} {} ", row_cycle.0, row_cycle.1);
        }
        println!();
        print!("evil_row_cycles: ");
        for evil_cycles in r.evil_row_cycles.into_split_iter() {
            print!("{} {} ", evil_cycles.0, evil_cycles.1);
        }
        println!();
        print!("local col_cycles: ");
        for col_cycle in r.local_dense_col_cycles.into_split_iter() {
            print!("{} {} ", col_cycle.0, col_cycle.1);
        }
        println!();
        print!("remote col_cycles: ");
        for col_cycle in r.remote_dense_col_cycles.into_split_iter() {
            print!("{} {} ", col_cycle.0, col_cycle.1);
        }
        println!();
        print!("dispatching: ");
        for _ in 0..5 {
            print!(
                "{} {} {}",
                r.dispatcher_reading_cycle, r.dispatcher_reading_cycle, r.dispatcher_reading_cycle
            );
        }
        println!();
    }
}

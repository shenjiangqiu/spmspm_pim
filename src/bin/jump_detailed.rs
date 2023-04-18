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
        print!("real_cycle: ");
        for real_cycle in r.real_cycle {
            print!("{} ", real_cycle);
        }

        println!();
        print!("row_cycles: ");
        for row_cycle in r.row_cycles.into_split_iter() {
            print!(
                "{} {} {} ",
                row_cycle.oepn_row, row_cycle.one_jump, row_cycle.muliple_jump
            );
        }
        println!();
        print!("evil_row_cycles: ");
        for evil_cycles in r.evil_row_cycles.into_split_iter() {
            print!(
                "{} {} {} ",
                evil_cycles.oepn_row, evil_cycles.one_jump, evil_cycles.muliple_jump
            );
        }
        println!();
        print!("col_cycles: ");
        for col_cycle in r.col_cycles.into_split_iter() {
            print!(
                "{} {} {} ",
                col_cycle.oepn_row, col_cycle.one_jump, col_cycle.muliple_jump
            );
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

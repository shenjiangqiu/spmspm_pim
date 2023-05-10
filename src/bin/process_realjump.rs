// use spmspm_pim::{analysis::remap_analyze::real_jump::RealJumpResult, tools::file_server};

// fn main() -> eyre::Result<()> {
//     //[normal, ideal, from_source, my, smart]
//     println!("jumptypes: normal ideal from_source my smart");
//     let weighted_result: Vec<RealJumpResult> =
//         serde_json::from_reader(file_server::file_reader("results/realjump/weighted.json")?)?;
//     println!("weighted:");
//     print_result(weighted_result);
//     let samebank_result: Vec<RealJumpResult> =
//         serde_json::from_reader(file_server::file_reader("results/realjump/same_bank.json")?)?;
//     println!("samebank:");
//     print_result(samebank_result);
//     Ok(())
// }

// fn print_result(weighted_result: Vec<RealJumpResult>) {
//     for r in weighted_result {
//         println!();

//         println!();
//         print!("row_cycles: ");
//         for row_cycle in r.row_cycles {
//             print!("{} ", row_cycle);
//         }
//         println!();
//         print!("evil_row_cycles: ");
//         for evil_cycles in r.evil_row_cycles {
//             print!("{} ", evil_cycles);
//         }
//         println!();
//         print!("local col_cycles: ");
//         for col_cycle in r.local_dense_col_cycles {
//             print!("{} ", col_cycle);
//         }
//         println!();
//         print!("remote col_cycles: ");
//         for col_cycle in r.remote_dense_col_cycles {
//             print!("{} ", col_cycle);
//         }
//         println!();
//         print!("dispatching: ");
//         for _ in 0..5 {
//             print!("{} ", r.dispatcher_reading_cycle);
//         }
//         println!();
//     }
// }
fn main() {}

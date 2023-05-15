#![allow(unused, clippy::too_many_arguments)]
use std::{collections::BTreeMap, path::PathBuf};

use clap::Parser;
use itertools::{izip, Itertools};
use spmspm_pim::{
    analysis::remap_analyze::{
        jump::{MyJumpOpt, NormalJumpCycle},
        real_jump::RealJumpResult,
        row_cycle::*,
    },
    pim::configv2::MappingType,
    tools::file_server,
};

mod common;

#[derive(Parser)]
struct Cli {
    path: PathBuf,
}
fn main() -> eyre::Result<()> {
    let cli = Cli::parse();
    //results/realjump/real_jump_sensitive_fix_row_open.json
    let result: common::RealJumpResultMap =
        serde_json::from_reader(file_server::file_reader(&cli.path.to_string_lossy()).unwrap())
            .unwrap();

    print_fn(
        &result,
        || {
            println!("total_cycles");
        },
        || {
            println!("total_cycles_end\n\n");
        },
        |g| {
            println!("graph: {}", g);
        },
        |g| {
            println!("graph_end: {}", g);
        },
        |_g, m| {
            println!("map: {:?}", m);
        },
        |_g, m| {
            println!("map_end: {:?}\n\n", m);
        },
        |graph_name, mapping_type, single_result| {
            let remote_write = single_result.remote_dense_col_cycles;
            let normal_256 = remote_write.normal_jump_cycle_256;
            let normal_32 = remote_write.normal_jump_cycle_32;
            let NormalJumpCycle {
                jump_one_cycle,
                jump_multiple_cycle,
                total_jumps_all,
                total_jumps_covered_by_row_open,
                jumps_not_covered_when_no_row_open,
                jumps_not_covered_when_more_shift,
            } = normal_256;
            println!(
                "normal_256: jump_one_cycle: {:?}, jump_multiple_cycle: {:?}, total_jumps_all: {:?}, total_jumps_covered_by_row_open: {:?}, jumps_not_covered_when_no_row_open: {:?}, jumps_not_covered_when_more_shift: {:?}",
                jump_one_cycle, jump_multiple_cycle, total_jumps_all, total_jumps_covered_by_row_open, jumps_not_covered_when_no_row_open, jumps_not_covered_when_more_shift
            );
            let NormalJumpCycle {
                jump_one_cycle,
                jump_multiple_cycle,
                total_jumps_all,
                total_jumps_covered_by_row_open,
                jumps_not_covered_when_no_row_open,
                jumps_not_covered_when_more_shift,
            } = normal_32;
            println!(
                "normal_32: jump_one_cycle: {:?}, jump_multiple_cycle: {:?}, total_jumps_all: {:?}, total_jumps_covered_by_row_open: {:?}, jumps_not_covered_when_no_row_open: {:?}, jumps_not_covered_when_more_shift: {:?}",
                jump_one_cycle, jump_multiple_cycle, total_jumps_all, total_jumps_covered_by_row_open, jumps_not_covered_when_no_row_open, jumps_not_covered_when_more_shift
            );

            let my_jump256: MyJumpOpt<16, 256> = remote_write.my_jump_opt_16_256;
            let MyJumpOpt {
                multi_jump_cycle,
                one_jump_cycle,
                opt_saved_times,
                opt_saved_cycles,
                all_cycle_hist_0,
                all_cycle_hist_1_2,
                all_cycle_hist_3_4,
                all_cycle_hist_5_8,
                all_cycle_hist_9_and_more,
                row_cycle_total,
                total_accesses,
                row_hits,
                row_misses,
                gloabl_row_accesses,
                global_row_hits,
                global_row_miss,
                global_row_cycles,
            } = my_jump256;
            println!(
                "my_jump256: multi_jump_cycle: {:?}, one_jump_cycle: {:?}, opt_saved_times: {:?}, opt_saved_cycles: {:?}, all_cycle_hist_0: {:?}, all_cycle_hist_1_2: {:?}, all_cycle_hist_3_4: {:?}, all_cycle_hist_5_8: {:?}, all_cycle_hist_9_and_more: {:?}",
                multi_jump_cycle, one_jump_cycle, opt_saved_times, opt_saved_cycles, all_cycle_hist_0, all_cycle_hist_1_2, all_cycle_hist_3_4, all_cycle_hist_5_8, all_cycle_hist_9_and_more
            );
            let my_jump_32 = remote_write.my_jump_opt_16_32;
            let MyJumpOpt {
                multi_jump_cycle,
                one_jump_cycle,
                opt_saved_times,
                opt_saved_cycles,
                all_cycle_hist_0,
                all_cycle_hist_1_2,
                all_cycle_hist_3_4,
                all_cycle_hist_5_8,
                all_cycle_hist_9_and_more,
                row_cycle_total,
                total_accesses,
                row_hits,
                row_misses,
                gloabl_row_accesses,
                global_row_hits,
                global_row_miss,
                global_row_cycles,
            } = my_jump_32;
            println!(
                "my_jump_32 multi_jump_cycle: {:?}, one_jump_cycle: {:?}, opt_saved_times: {:?}, opt_saved_cycles: {:?}, all_cycle_hist_0: {:?}, all_cycle_hist_1_2: {:?}, all_cycle_hist_3_4: {:?}, all_cycle_hist_5_8: {:?}, all_cycle_hist_9_and_more: {:?}",
                multi_jump_cycle, one_jump_cycle, opt_saved_times, opt_saved_cycles, all_cycle_hist_0, all_cycle_hist_1_2, all_cycle_hist_3_4, all_cycle_hist_5_8, all_cycle_hist_9_and_more
            );

            println!("\n\n");
        },
    );

    Ok(())
}
fn print_fn<'a>(
    result: &'a common::RealJumpResultMap,
    start: impl FnOnce(),
    end: impl FnOnce(),
    mut graph_start: impl FnMut(&'a str),
    mut graph_end: impl FnMut(&'a str),
    mut map_start: impl FnMut(&'a str, &MappingType),
    mut map_end: impl FnMut(&'a str, &MappingType),
    mut gap_fn: impl FnMut(&'a str, &'a MappingType, &'a RealJumpResult),
) {
    start();
    for (graph, graph_result) in result {
        graph_start(graph);
        for (map, map_result) in graph_result {
            map_start(graph, map);
            gap_fn(graph, map, map_result);
            map_end(graph, map);
        }
        graph_end(graph);
    }
    end();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test() {
        let my_map: BTreeMap<_, _> = [("hello".to_string(), 1), ("world".to_string(), 2)]
            .into_iter()
            .collect();
        println!("{:?}", my_map.get("hello"));
    }
}

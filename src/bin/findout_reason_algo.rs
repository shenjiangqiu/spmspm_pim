#![allow(unused, clippy::too_many_arguments, clippy::type_complexity)]

use std::{collections::BTreeMap, path::PathBuf};

use clap::Parser;
use itertools::{izip, Itertools};
use spmspm_pim::{
    analysis::remap_analyze::{
        action::TotalAction,
        real_jump::{AllAlgorithomResults, RealJumpResult},
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
type TreeAlgo = [TwoLocalRemote; 3];
fn main() -> eyre::Result<()> {
    let cli = Cli::parse();
    //results/realjump/real_jump_sensitive_fix_row_open.json
    let result: common::AllJumpResultMap =
        serde_json::from_reader(file_server::file_reader(&cli.path.to_string_lossy()).unwrap())
            .unwrap();

    let mut total_cycle: BTreeMap<
        String,
        BTreeMap<&MappingType, BTreeMap<AllJumpCyclesTypes, Vec<(&str, (usize, usize, usize))>>>,
    > = BTreeMap::new();
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
            let AllAlgorithomResults {
                bfs,
                page_rank,
                spmm,
            } = single_result;
            let bfs_break_down = break_algorithm(bfs);
            let page_rank_break_down = break_algorithm(page_rank);
            let spmm_break_down = break_algorithm(spmm);
            let all: TreeAlgo = [bfs_break_down, page_rank_break_down, spmm_break_down];
            for algo in all.into_iter().enumerate() {
                let (algo_name, local_remote): (usize, TwoLocalRemote) = algo;
                for local_or_remote in local_remote.into_iter().enumerate() {
                    let (local_or_remote, normal_opt): (usize, TwoNormalOpt) = local_or_remote;
                    for normal_opt in normal_opt.into_iter().enumerate() {
                        let (normal_or_opt, select_type): (usize, TwoSelect) = normal_opt;
                        for select_type in select_type.into_iter().enumerate() {
                            let (select_type, walker_size): (usize, FourWalker) = select_type;
                            print!(
                                "algo-{} lo-re-{} no-op-{} sel-{} ",
                                algo_name, local_or_remote, normal_or_opt, select_type
                            );
                            for walker_size in walker_size.into_iter().enumerate() {
                                let (walker_size, cycle): (usize, TreeCycle) = walker_size;
                                print!(
                                    "walker_size-{}, {} {} {} {:.2}",
                                    walker_size,
                                    cycle[0],
                                    cycle[1],
                                    cycle[2],
                                    cycle[2] as f32 / (cycle[0] + cycle[1] + cycle[2]) as f32
                                );
                            }
                            println!();
                        }
                    }
                }
            }
        },
    );

    Ok(())
}
type TreeCycle = [usize; 3];
type FourWalker = [TreeCycle; 4];
type TwoSelect = [FourWalker; 2];
type TwoNormalOpt = [TwoSelect; 2];
type TwoLocalRemote = [TwoNormalOpt; 2];
/// 2-local_remote, 2-normal_opt, 2-select_type, 4-walker_size, 3-cycle
fn break_algorithm(bfs: &RealJumpResult) -> TwoLocalRemote {
    let local = bfs.row_cycles;
    let local_update_break_down = break_down(local);
    let remote = bfs.remote_dense_col_cycles;
    let remote_update_break_down = break_down(remote);
    [local_update_break_down, remote_update_break_down]
}
/// 2-normal_opt, 2-select_type, 4-walker_size, 3-cycle
fn break_down(local: AllJumpCycles) -> TwoNormalOpt {
    let local_normal_256 = local.normal_jump_cycle_256;
    let local_normal_128 = local.normal_jump_cycle_128;
    let local_normal_64 = local.normal_jump_cycle_64;
    let local_normal_32 = local.normal_jump_cycle_32;
    let local_break_down = [
        [
            local_normal_256.jump_multiple_cycle,
            local_normal_256.jump_one_cycle,
            0,
        ],
        [
            local_normal_128.jump_multiple_cycle,
            local_normal_128.jump_one_cycle,
            0,
        ],
        [
            local_normal_64.jump_multiple_cycle,
            local_normal_64.jump_one_cycle,
            0,
        ],
        [
            local_normal_32.jump_multiple_cycle,
            local_normal_32.jump_one_cycle,
            0,
        ],
    ];
    let local_selective_256 = local.normal_jump_cycle_selective_256;
    let local_selective_128 = local.normal_jump_cycle_selective_128;
    let local_selective_64 = local.normal_jump_cycle_selective_64;
    let local_selective_32 = local.normal_jump_cycle_selective_32;
    let local_selective_break_down = [
        [
            local_selective_256.jump_multiple_cycle,
            local_selective_256.jump_one_cycle,
            local_selective_256.extra_scan_cycles,
        ],
        [
            local_selective_128.jump_multiple_cycle,
            local_selective_128.jump_one_cycle,
            local_selective_256.extra_scan_cycles,
        ],
        [
            local_selective_64.jump_multiple_cycle,
            local_selective_64.jump_one_cycle,
            local_selective_256.extra_scan_cycles,
        ],
        [
            local_selective_32.jump_multiple_cycle,
            local_selective_32.jump_one_cycle,
            local_selective_256.extra_scan_cycles,
        ],
    ];
    let total_normal = [local_break_down, local_selective_break_down];
    let local_opt_256 = local.my_jump_opt_4_256;
    let local_opt_128 = local.my_jump_opt_4_128;
    let local_opt_64 = local.my_jump_opt_4_64;
    let local_opt_32 = local.my_jump_opt_4_32;
    let local_opt_break_down = [
        [
            local_opt_256.multi_jump_cycle,
            local_opt_256.one_jump_cycle,
            0,
        ],
        [
            local_opt_128.multi_jump_cycle,
            local_opt_128.one_jump_cycle,
            0,
        ],
        [
            local_opt_64.multi_jump_cycle,
            local_opt_64.one_jump_cycle,
            0,
        ],
        [
            local_opt_32.multi_jump_cycle,
            local_opt_32.one_jump_cycle,
            0,
        ],
    ];
    let local_opt_selective_256 = local.my_jump_opt_selective_4_256;
    let local_opt_selective_128 = local.my_jump_opt_selective_4_128;
    let local_opt_selective_64 = local.my_jump_opt_selective_4_64;
    let local_opt_selective_32 = local.my_jump_opt_selective_4_32;
    let local_opt_selective_break_down = [
        [
            local_opt_selective_256.multi_jump_cycle,
            local_opt_selective_256.one_jump_cycle,
            local_opt_selective_256.extra_scan_cycle,
        ],
        [
            local_opt_selective_128.multi_jump_cycle,
            local_opt_selective_128.one_jump_cycle,
            local_opt_selective_256.extra_scan_cycle,
        ],
        [
            local_opt_selective_64.multi_jump_cycle,
            local_opt_selective_64.one_jump_cycle,
            local_opt_selective_256.extra_scan_cycle,
        ],
        [
            local_opt_selective_32.multi_jump_cycle,
            local_opt_selective_32.one_jump_cycle,
            local_opt_selective_256.extra_scan_cycle,
        ],
    ];
    let total_opt = [local_opt_break_down, local_opt_selective_break_down];

    [total_normal, total_opt]
}

fn print_single_algorithm<'a>(
    algo_name: String,
    single_result: &RealJumpResult,
    total_cycle: &mut BTreeMap<
        String,
        BTreeMap<
            &'a MappingType,
            BTreeMap<AllJumpCyclesTypes, Vec<(&'a str, (usize, usize, usize))>>,
        >,
    >,
    mapping_type: &'a MappingType,
    graph_name: &'a str,
) {
    let mut remote_write_total_action = TotalAction::default();

    single_result
        .remote_dense_col_cycles
        .apply(&mut remote_write_total_action);
    let remote_write_cycle = remote_write_total_action.total;
    for (index, remote_write, real_local) in izip!(
        AllJumpCyclesTypes::default(),
        remote_write_cycle,
        &single_result.real_local_cycle
    ) {
        // this is ideal
        let dispatching = single_result.dispatcher_reading_cycle;
        let total = (*real_local, dispatching, remote_write);

        let jump_type = index;
        let total_cycle = total_cycle
            .entry(algo_name.clone())
            .or_default()
            .entry(mapping_type)
            .or_default()
            .entry(jump_type)
            .or_default();
        total_cycle.push((graph_name, total));
    }
    println!("\n\n");
}

fn print_fn<'a>(
    result: &'a common::AllJumpResultMap,
    start: impl FnOnce(),
    end: impl FnOnce(),
    mut graph_start: impl FnMut(&'a str),
    mut graph_end: impl FnMut(&'a str),
    mut map_start: impl FnMut(&'a str, &MappingType),
    mut map_end: impl FnMut(&'a str, &MappingType),
    mut gap_fn: impl FnMut(&'a str, &'a MappingType, &'a AllAlgorithomResults),
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

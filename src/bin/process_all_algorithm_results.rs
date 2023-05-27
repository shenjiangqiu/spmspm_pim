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
            // println!("total_cycles");
        },
        || {
            // println!("total_cycles_end\n\n");
        },
        |g| {
            // println!("graph: {}", g);
        },
        |g| {
            // println!("graph_end: {}", g);
        },
        |_g, m| {
            // println!("map: {:?}", m);
        },
        |_g, m| {
            // println!("map_end: {:?}\n\n", m);
        },
        |graph_name, mapping_type, single_result| {
            let AllAlgorithomResults {
                bfs,
                page_rank,
                spmm,
            } = single_result;
            print_single_algorithm(
                "bfs".to_string(),
                bfs,
                &mut total_cycle,
                mapping_type,
                graph_name,
            );
            print_single_algorithm(
                "page_rank".to_string(),
                page_rank,
                &mut total_cycle,
                mapping_type,
                graph_name,
            );
            print_single_algorithm(
                "spmm".to_string(),
                spmm,
                &mut total_cycle,
                mapping_type,
                graph_name,
            );
        },
    );

    for (algo_name, result) in total_cycle {
        for (mapping_type, mapping_result) in result {
            for (jump_type, jump_result) in mapping_result {
                let st = jump_result
                    .iter()
                    .map(|(graph_name, (real, disp, remote))| {
                        // let total = real + disp + remote;
                        // let real_percent = *real as f64 / total as f64;
                        // let disp_percent = *disp as f64 / total as f64;
                        // let remote_percent = *remote as f64 / total as f64;
                        // format!(" | {} {} {}", real_percent, disp_percent, remote_percent)
                        let total = real + disp + remote;
                        format!(" | | | {graph_name} {} {} {} {}", real, disp, remote, total)
                    })
                    .join(" ");
                println!("{algo_name}-{:?}-{:?}: {}", mapping_type, jump_type, st);
            }
        }
    }

    Ok(())
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

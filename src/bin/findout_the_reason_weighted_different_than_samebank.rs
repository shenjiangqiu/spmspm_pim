use std::{collections::BTreeMap, path::PathBuf};

use clap::Parser;
use itertools::{izip, Itertools};
use spmspm_pim::{
    analysis::remap_analyze::{real_jump::RealJumpResult, row_cycle::*},
    pim::configv2::MappingType,
    tools::file_server,
};

mod common;
struct TotalAction {
    total: Vec<usize>,
}
impl RowCycleAction for TotalAction {
    fn apply<T: JumpCycle + UpdatableJumpCycle + AddableJumpCycle>(&mut self, item: &T) {
        self.total.push(item.total());
    }
}
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

    let mut total_cycle: BTreeMap<&MappingType, BTreeMap<AllJumpCyclesTypes, Vec<(&str, usize)>>> =
        BTreeMap::new();
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
            let mut remote_write_total_action = TotalAction { total: vec![] };
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
                let total = real_local + dispatching + remote_write;

                let jump_type = index.into();
                let total_cycle = total_cycle
                    .entry(mapping_type)
                    .or_default()
                    .entry(jump_type)
                    .or_default();
                total_cycle.push((graph_name, total));
            }
            println!("\n\n");
        },
    );

    total_cycle
        .iter()
        .for_each(|(mapping_type, mapping_result)| {
            mapping_result.iter().for_each(|(jump_type, jump_result)| {
                let st = jump_result.iter().map(|x| x.1).join(" ");
                println!("{:?}-{:?}: {}", mapping_type, jump_type, st);
            });
        });

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

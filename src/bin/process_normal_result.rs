use std::collections::BTreeMap;

use itertools::Itertools;
use spmspm_pim::{analysis::remap_analyze::real_jump::RealJumpResult, pim::configv2::MappingType};

mod common;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum JumpType {
    Normal,
    Ideal,
    My(usize),
}
fn main() -> eyre::Result<()> {
    //results/realjump/real_jump_sensitive_fix_row_open.json
    let result: common::RealJumpResultMap = serde_json::from_str(include_str!(
        "../../results/realjump/add_normal_and_opt.json"
    ))?;
    // first get the normal mapping
    for (graph, graph_result) in result
        .iter()
        .map(|(g, r)| (g, r.get(&MappingType::SameBank).unwrap()))
    {
        println!("graph: {}", graph);
        println!(
            "total_cycles_local_col: {:?}",
            graph_result
                .local_dense_col_cycles
                .normal_jump_cycle
                .cover_rate()
        );
        println!(
            "total_cycles_remote_col: {:?}",
            graph_result
                .remote_dense_col_cycles
                .normal_jump_cycle
                .cover_rate()
        );
        println!(
            "total_cycles_row: {:?}",
            graph_result.row_cycles.normal_jump_cycle.cover_rate()
        );
    }
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

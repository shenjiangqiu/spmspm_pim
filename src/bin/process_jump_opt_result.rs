use std::collections::BTreeMap;

use itertools::Itertools;
use spmspm_pim::{
    analysis::remap_analyze::{real_jump::RealJumpResult, row_cycle::JumpTypes},
    pim::configv2::MappingType,
};

mod common;

fn main() -> eyre::Result<()> {
    //results/realjump/real_jump_sensitive_fix_row_open.json
    let result: common::RealJumpResultMap = serde_json::from_str(include_str!(
        "../../results/realjump/add_normal_and_opt.json"
    ))?;

    let mut total_cycle: BTreeMap<&MappingType, BTreeMap<JumpTypes, Vec<(&str, usize)>>> =
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
            for (index, ((((_row, _evil_row), _local_write), remote_write), real_local)) in
                single_result
                    .row_cycles
                    .into_iter()
                    .zip(single_result.evil_row_cycles.into_iter())
                    .zip(single_result.local_dense_col_cycles.into_iter())
                    .zip(single_result.remote_dense_col_cycles.into_iter())
                    .zip(single_result.real_local_cycle.into_iter())
                    .enumerate()
            {
                // this is ideal
                let dispatching = single_result.dispatcher_reading_cycle;
                let total = real_local + dispatching + remote_write;

                let jump_type = index.into();
                let total_cycle = total_cycle
                    .entry(mapping_type)
                    .or_insert_with(BTreeMap::new)
                    .entry(jump_type)
                    .or_insert_with(Vec::new);
                total_cycle.push((graph_name, total));
            }
            println!("\n\n");
        },
    );
    // println!("total_cycle: {:?}", total_cycle);
    let same_bank_normal = total_cycle
        .get(&MappingType::SameBank)
        .unwrap()
        .get(&JumpTypes::Normal)
        .unwrap();
    let same_bank_ideal = total_cycle
        .get(&MappingType::SameBank)
        .unwrap()
        .get(&JumpTypes::Ideal)
        .unwrap();
    let same_bank_my_opt = total_cycle
        .get(&MappingType::SameBank)
        .unwrap()
        .get(&JumpTypes::My16Opt)
        .unwrap();
    let same_bank_my_no_opt = total_cycle
        .get(&MappingType::SameBank)
        .unwrap()
        .get(&JumpTypes::My16)
        .unwrap();
    let same_bank_my_no_overhead = total_cycle
        .get(&MappingType::SameBank)
        .unwrap()
        .get(&JumpTypes::My16NoOverhead)
        .unwrap();
    // weighted
    let weighted_bank_normal = total_cycle
        .get(&MappingType::SameBankWeightedMapping)
        .unwrap()
        .get(&JumpTypes::Normal)
        .unwrap();
    let weighted_bank_ideal = total_cycle
        .get(&MappingType::SameBankWeightedMapping)
        .unwrap()
        .get(&JumpTypes::Ideal)
        .unwrap();
    let weighted_bank_my_opt = total_cycle
        .get(&MappingType::SameBankWeightedMapping)
        .unwrap()
        .get(&JumpTypes::My16Opt)
        .unwrap();
    let weighted_bank_my_no_opt = total_cycle
        .get(&MappingType::SameBankWeightedMapping)
        .unwrap()
        .get(&JumpTypes::My16)
        .unwrap();
    let weighted_bank_my_no_overhead = total_cycle
        .get(&MappingType::SameBankWeightedMapping)
        .unwrap()
        .get(&JumpTypes::My16NoOverhead)
        .unwrap();

    let st = same_bank_normal
        .iter()
        .map(|x| x.0.split('/').last().unwrap())
        .join(" ");
    println!("graphs: {}", st);
    let st = same_bank_normal.iter().map(|x| x.1).join(" ");
    println!("same_bank_normal: {}", st);
    let st = same_bank_ideal.iter().map(|x| x.1).join(" ");
    println!("same_bank_ideal: {}", st);
    let st = same_bank_my_opt.iter().map(|x| x.1).join(" ");
    println!("same_bank_my_opt: \n{}", st);
    let st = same_bank_my_no_opt.iter().map(|x| x.1).join(" ");
    println!("same_bank_my_no_opt: \n{}", st);
    let st = same_bank_my_no_overhead.iter().map(|x| x.1).join(" ");
    println!("same_bank_my_no_overhead: \n{}", st);

    let st = weighted_bank_normal.iter().map(|x| x.1).join(" ");
    println!("weighted_bank_normal: {}", st);
    let st = weighted_bank_ideal.iter().map(|x| x.1).join(" ");
    println!("weighted_bank_ideal: {}", st);
    let st = weighted_bank_my_opt.iter().map(|x| x.1).join(" ");
    println!("weighted_bank_my_opt: \n{}", st);
    let st = weighted_bank_my_no_opt.iter().map(|x| x.1).join(" ");
    println!("weighted_bank_my_no_opt: \n{}", st);
    let st = weighted_bank_my_no_overhead.iter().map(|x| x.1).join(" ");
    println!("weighted_bank_my_no_overhead: \n{}", st);

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

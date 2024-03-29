// use std::collections::BTreeMap;

// use itertools::Itertools;
// use spmspm_pim::{analysis::remap_analyze::real_jump::RealJumpResult, pim::configv2::MappingType};

// mod common;
// #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// enum JumpType {
//     Normal,
//     Ideal,
//     My(usize),
// }
// fn main() -> eyre::Result<()> {
//     //results/realjump/real_jump_sensitive_fix_row_open.json
//     let result: common::RealJumpResultMap = serde_json::from_str(include_str!(
//         "../../results/realjump/add_normal_and_opt.json"
//     ))?;

//     let mut total_cycle: BTreeMap<&MappingType, BTreeMap<JumpType, Vec<(&str, usize)>>> =
//         BTreeMap::new();
//     print_fn(
//         &result,
//         || {
//             println!("total_cycles");
//         },
//         || {
//             println!("total_cycles_end\n\n");
//         },
//         |g| {
//             println!("graph: {}", g);
//         },
//         |g| {
//             println!("graph_end: {}", g);
//         },
//         |_g, m| {
//             println!("map: {:?}", m);
//         },
//         |_g, m| {
//             println!("map_end: {:?}\n\n", m);
//         },
//         |graph_name, mapping_type, single_result| {
//             for (index, ((((_row, _evil_row), _local_write), remote_write), real_local)) in
//                 single_result
//                     .row_cycles
//                     .into_iter()
//                     .zip(single_result.evil_row_cycles.into_iter())
//                     .zip(single_result.local_dense_col_cycles.into_iter())
//                     .zip(single_result.remote_dense_col_cycles.into_iter())
//                     .zip(single_result.real_local_cycle.into_iter())
//                     .enumerate()
//             {
//                 // this is ideal
//                 let dispatching = single_result.dispatcher_reading_cycle;
//                 let total = real_local + dispatching + remote_write;

//                 let jump_type = match index {
//                     0 => JumpType::Normal,
//                     1 => JumpType::Ideal,
//                     3 => JumpType::My(16),
//                     4 => JumpType::My(32),
//                     5 => JumpType::My(64),
//                     _ => continue,
//                 };
//                 let total_cycle = total_cycle
//                     .entry(mapping_type)
//                     .or_insert_with(BTreeMap::new)
//                     .entry(jump_type)
//                     .or_insert_with(Vec::new);
//                 total_cycle.push((graph_name, total));
//             }
//             println!("\n\n");
//         },
//     );
//     // println!("total_cycle: {:?}", total_cycle);
//     let same_bank_normal = total_cycle
//         .get(&MappingType::SameBank)
//         .unwrap()
//         .get(&JumpType::Normal)
//         .unwrap();
//     let same_bank_ideal = total_cycle
//         .get(&MappingType::SameBank)
//         .unwrap()
//         .get(&JumpType::Ideal)
//         .unwrap();
//     let same_bank_my = total_cycle
//         .get(&MappingType::SameBank)
//         .unwrap()
//         .iter()
//         .filter(|x| match x.0 {
//             JumpType::My(_) => true,
//             _ => false,
//         });
//     let weighted_bank_normal = total_cycle
//         .get(&MappingType::SameBankWeightedMapping)
//         .unwrap()
//         .get(&JumpType::Normal)
//         .unwrap();
//     let weighted_bank_ideal = total_cycle
//         .get(&MappingType::SameBankWeightedMapping)
//         .unwrap()
//         .get(&JumpType::Ideal)
//         .unwrap();
//     let weighted_bank_my = total_cycle
//         .get(&MappingType::SameBankWeightedMapping)
//         .unwrap()
//         .iter()
//         .filter(|x| match x.0 {
//             JumpType::My(_) => true,
//             _ => false,
//         });
//     let st = same_bank_normal
//         .iter()
//         .map(|x| x.0.split('/').last().unwrap())
//         .join(" ");
//     println!("graphs: {}", st);
//     let st = same_bank_normal.iter().map(|x| x.1).join(" ");
//     println!("same_bank_normal: {}", st);
//     let st = same_bank_ideal.iter().map(|x| x.1).join(" ");
//     println!("same_bank_ideal: {}", st);
//     let st = same_bank_my
//         .map(|x| format!("{:?}: {}", x.0, x.1.iter().map(|x| x.1).join(" ")))
//         .join("\n");
//     println!("same_bank_my: \n{}", st);

//     let st = weighted_bank_normal.iter().map(|x| x.1).join(" ");
//     println!("weighted_bank_normal: {}", st);
//     let st = weighted_bank_ideal.iter().map(|x| x.1).join(" ");
//     println!("weighted_bank_ideal: {}", st);
//     let st = weighted_bank_my
//         .map(|x| format!("{:?}: {}", x.0, x.1.iter().map(|x| x.1).join(" ")))
//         .join("\n");
//     println!("weighted_bank_my: \n{}", st);

//     Ok(())
// }

// fn print_fn<'a>(
//     result: &'a common::RealJumpResultMap,
//     start: impl FnOnce(),
//     end: impl FnOnce(),
//     mut graph_start: impl FnMut(&'a str),
//     mut graph_end: impl FnMut(&'a str),
//     mut map_start: impl FnMut(&'a str, &MappingType),
//     mut map_end: impl FnMut(&'a str, &MappingType),
//     mut gap_fn: impl FnMut(&'a str, &'a MappingType, &'a RealJumpResult),
// ) {
//     start();
//     for (graph, graph_result) in result {
//         graph_start(graph);
//         for (map, map_result) in graph_result {
//             map_start(graph, map);
//             gap_fn(graph, map, map_result);
//             map_end(graph, map);
//         }
//         graph_end(graph);
//     }
//     end();
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     #[test]
//     fn test() {
//         let my_map: BTreeMap<_, _> = [("hello".to_string(), 1), ("world".to_string(), 2)]
//             .into_iter()
//             .collect();
//         println!("{:?}", my_map.get("hello"));
//     }
// }
fn main() {}

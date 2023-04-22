use std::collections::BTreeMap;

use spmspm_pim::{analysis::remap_analyze::real_jump::RealJumpResult, pim::configv2::MappingType};

mod common;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum NormalOrMy {
    Normal,
    My,
}
fn main() -> eyre::Result<()> {
    let result: common::RealJumpResultMap = serde_json::from_str(include_str!(
        "../../results/realjump/real_jump_sensitive_husky5.json"
    ))?;
    // println!("{:?}", result);
    //first print the total_cycles
    let mut last_normal_local = 0;
    let mut last_normal_dispatching = 0;
    let mut last_normal_remote = 0;
    let mut last_total = 0;
    let mut last_ideal_local = 0;
    let mut last_ideal_dispatching = 0;
    let mut last_ideal_remote = 0;
    let mut last_ideal_total = 0;

    let mut local_speed_up = vec![];
    let mut dispaching_speed_up = vec![];
    let mut remote_speed_up = vec![];
    let mut total_speed_up = vec![];

    let mut ideal_to_my_local_speed_up = vec![];
    let mut ideal_to_my_dispaching_speed_up = vec![];
    let mut ideal_to_my_remote_speed_up = vec![];
    let mut ideal_to_my_total_speed_up = vec![];

    let mut last_normal_mapping_total = 0;

    let mut compare_same_bank_to_same_bank = vec![];
    let mut compare_weighted_to_same_bank = vec![];
    let mut total_cycle: BTreeMap<MappingType, BTreeMap<NormalOrMy, Vec<usize>>> = BTreeMap::new();
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
        |_g, _m, gp, r| {
            print!("gap: {}  ", gp);
            for (index, ((((row, evil_row), local_write), remote_write), real_local)) in r
                .row_cycles
                .into_iter()
                .zip(r.evil_row_cycles.into_iter())
                .zip(r.local_dense_col_cycles.into_iter())
                .zip(r.remote_dense_col_cycles.into_iter())
                .zip(r.real_local_cycle.into_iter())
                .enumerate()
            {
                // this is ideal
                let dispatching = r.dispatcher_reading_cycle;
                let total = real_local + dispatching + remote_write;
                let real_local_rate = real_local as f32 / total as f32;
                let dispatching_rate = dispatching as f32 / total as f32;
                let remote_write_rate = remote_write as f32 / total as f32;

                if index == 0 {
                    last_normal_local = real_local;
                    last_normal_dispatching = dispatching;
                    last_normal_remote = remote_write;
                    last_total = total;
                    total_cycle
                        .entry(_m.clone())
                        .or_default()
                        .entry(NormalOrMy::Normal)
                        .or_default()
                        .push(total);
                    if _m == &MappingType::SameBank {
                        last_normal_mapping_total = total;
                    }
                    print!("normal: {real_local} {dispatching} {remote_write} {total} {real_local_rate} {dispatching_rate} {remote_write_rate} " );
                } else if index == 1 {
                    last_ideal_local = real_local;
                    last_ideal_dispatching = dispatching;
                    last_ideal_remote = remote_write;
                    last_ideal_total = total;
                } else if index == 3 {
                    total_cycle
                        .entry(_m.clone())
                        .or_default()
                        .entry(NormalOrMy::My)
                        .or_default()
                        .push(total);
                    local_speed_up.push(last_normal_local as f32 / real_local as f32);

                    dispaching_speed_up.push(last_normal_dispatching as f32 / dispatching as f32);
                    remote_speed_up.push(last_normal_remote as f32 / remote_write as f32);
                    total_speed_up.push(last_total as f32 / total as f32);

                    ideal_to_my_local_speed_up.push(real_local as f32 / last_ideal_local as f32);
                    ideal_to_my_dispaching_speed_up
                        .push(dispatching as f32 / last_ideal_dispatching as f32);
                    ideal_to_my_remote_speed_up
                        .push(remote_write as f32 / last_ideal_remote as f32);
                    ideal_to_my_total_speed_up.push(total as f32 / last_ideal_total as f32);

                    print!("my: {real_local} {dispatching} {remote_write} {total} {real_local_rate} {dispatching_rate} {remote_write_rate} " );
                    if _m == &MappingType::SameBank {
                        compare_same_bank_to_same_bank
                            .push(total as f32 / last_normal_mapping_total as f32);
                    } else {
                        compare_weighted_to_same_bank
                            .push(total as f32 / last_normal_mapping_total as f32);
                    }
                }
            }
            println!("\n\n");
        },
    );
    println!("local_speed_up: {:?}", local_speed_up);
    println!("dispaching_speed_up: {:?}", dispaching_speed_up);
    println!("remote_speed_up: {:?}", remote_speed_up);
    println!("total_speed_up: {:?}", total_speed_up);
    let average_local_speed_up = local_speed_up.iter().sum::<f32>() / local_speed_up.len() as f32;
    let average_dispaching_speed_up =
        dispaching_speed_up.iter().sum::<f32>() / dispaching_speed_up.len() as f32;
    let average_remote_speed_up =
        remote_speed_up.iter().sum::<f32>() / remote_speed_up.len() as f32;
    let average_total_speed_up = total_speed_up.iter().sum::<f32>() / total_speed_up.len() as f32;
    println!("average_local_speed_up: {}", average_local_speed_up);
    println!(
        "average_dispaching_speed_up: {}",
        average_dispaching_speed_up
    );
    println!("average_remote_speed_up: {}", average_remote_speed_up);
    println!("average_total_speed_up: {}", average_total_speed_up);

    println!(
        "ideal_to_my_local_speed_up: {:?}",
        ideal_to_my_local_speed_up
    );
    println!(
        "ideal_to_my_dispaching_speed_up: {:?}",
        ideal_to_my_dispaching_speed_up
    );
    println!(
        "ideal_to_my_remote_speed_up: {:?}",
        ideal_to_my_remote_speed_up
    );
    println!(
        "ideal_to_my_total_speed_up: {:?}",
        ideal_to_my_total_speed_up
    );
    println!(
        "average_ideal_to_my_total_speedup: {}",
        ideal_to_my_total_speed_up.iter().sum::<f32>() / ideal_to_my_total_speed_up.len() as f32
    );

    println!(
        "compare_same_bank_to_same_bank: {:?}",
        compare_same_bank_to_same_bank
    );
    println!(
        "compare_weighted_to_same_bank: {:?}",
        compare_weighted_to_same_bank
    );
    println!(
        "average_compare_same_bank_to_same_bank: {}",
        compare_same_bank_to_same_bank.iter().sum::<f32>()
            / compare_same_bank_to_same_bank.len() as f32
    );
    println!(
        "average_compare_weighted_to_same_bank: {}",
        compare_weighted_to_same_bank.iter().sum::<f32>()
            / compare_weighted_to_same_bank.len() as f32
    );
    let normal_same_bank = total_cycle
        .get(&MappingType::SameBank)
        .unwrap()
        .get(&NormalOrMy::Normal)
        .unwrap();
    let my_same_bank = total_cycle
        .get(&MappingType::SameBank)
        .unwrap()
        .get(&NormalOrMy::My)
        .unwrap();
    let normal_weighted = total_cycle
        .get(&MappingType::SameBankWeightedMapping)
        .unwrap()
        .get(&NormalOrMy::Normal)
        .unwrap();
    let my_weighted = total_cycle
        .get(&MappingType::SameBankWeightedMapping)
        .unwrap()
        .get(&NormalOrMy::My)
        .unwrap();
    // the averages
    let normal_same_bank_average =
        normal_same_bank.iter().sum::<usize>() as f32 / normal_same_bank.len() as f32;
    let my_same_bank_average =
        my_same_bank.iter().sum::<usize>() as f32 / my_same_bank.len() as f32;
    let normal_weighted_average =
        normal_weighted.iter().sum::<usize>() as f32 / normal_weighted.len() as f32;
    let my_weighted_average = my_weighted.iter().sum::<usize>() as f32 / my_weighted.len() as f32;
    println!("normal_same_bank_average: {}", normal_same_bank_average);
    println!("my_same_bank_average: {}", my_same_bank_average);
    println!("normal_weighted_average: {}", normal_weighted_average);
    println!("my_weighted_average: {}", my_weighted_average);

    Ok(())
}

fn print_fn(
    result: &common::RealJumpResultMap,
    start: impl FnOnce(),
    end: impl FnOnce(),
    mut graph_start: impl FnMut(&str),
    mut graph_end: impl FnMut(&str),
    mut map_start: impl FnMut(&str, &MappingType),
    mut map_end: impl FnMut(&str, &MappingType),
    mut gap_fn: impl FnMut(&str, &MappingType, &usize, &RealJumpResult),
) {
    start();
    for (graph, graph_result) in result {
        graph_start(graph);
        for (map, map_result) in graph_result {
            map_start(graph, map);
            for (gap, gap_result) in map_result {
                gap_fn(graph, map, gap, gap_result);
            }
            map_end(graph, map);
        }
        graph_end(graph);
    }
    end();
}

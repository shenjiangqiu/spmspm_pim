use spmspm_pim::{analysis::remap_analyze::real_jump::RealJumpResult, pim::configv2::MappingType};

mod common;
fn main() -> eyre::Result<()> {
    let result: common::RealJumpResultMap =
        serde_json::from_str(include_str!("../../output/real_jump_sensitive.json"))?;
    // println!("{:?}", result);
    //first print the total_cycles
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
            println!("map_end: {:?}", m);
        },
        |_g, _m, gp, r| {
            print!("gap: {}  ", gp);
            for (((row, evil_row), local_write), remote_write) in r
                .row_cycles
                .into_iter()
                .zip(r.evil_row_cycles.into_iter())
                .zip(r.local_dense_col_cycles.into_iter())
                .zip(r.remote_dense_col_cycles.into_iter())
            {
                print!(
                    "row: {}  evil_row: {}  local_write: {} , dispatching: {}  remote_write: {} , ",
                    row, evil_row, local_write, r.dispatcher_reading_cycle, remote_write
                );
            }
            println!("");
        },
    );

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

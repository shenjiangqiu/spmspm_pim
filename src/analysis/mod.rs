//! # the analysis module
//! show the key timing and bandwidth
//!

use crate::pim::config::Config;
pub mod analyze_channel;
pub mod analyze_gearbox;
pub mod analyze_gearbox_origin;
pub mod analyze_gearbox_origin_all;
pub mod analyze_gearbox_origin_all_v2;
pub mod analyze_gearbox_origin_all_v2_overflow;
pub mod analyze_gearbox_parallel;
pub(crate) mod analyze_nnz;
pub mod analyze_nnz_gearbox;
pub(crate) mod analyze_nnz_native;
pub mod analyze_refined_gearbox;
pub mod analyze_split_spmm;
pub mod compute_merger_cycle;
pub mod event;
pub mod mergered_stream;
pub mod overlap;
pub mod partition;
pub mod schedule_window;
pub mod sequential_event_sim;
pub mod split;

pub mod analyze_gearbox_overflow_and_traffic;
pub fn print_all_stats(config: &Config) {
    let single_task_overlap_stat = overlap::compute_single_task_overlap_stat(config);
    for stat in single_task_overlap_stat {
        println!("graph: {}", stat.graph);
        stat.print();
    }
    let lock_task_overlap_stat = sequential_event_sim::compute_lock_task_overlap_stat(config);
    for stat in lock_task_overlap_stat {
        println!("graph: {}", stat.graph);
        stat.print();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pim::config::Config;
    #[test]
    fn test_print_all_stat() {
        let config: Config =
            toml::from_str(std::fs::read_to_string("ddr4.toml").unwrap().as_str()).unwrap();
        print_all_stats(&config);
    }
}

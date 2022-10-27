//! # the analysis module
//! show the key timing and bandwidth
//!

use crate::pim::config::Config;

pub mod overlap;
pub mod sequential_event_sim;

pub fn print_all_stats(config: &Config) {
    let single_task_overlap_stat = overlap::compute_single_task_overlap_stat(config);
    single_task_overlap_stat.print();
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

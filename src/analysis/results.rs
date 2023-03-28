use serde::{Deserialize, Serialize};

use super::traits::{get_mean_std_max, ReportStats};

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct SubArrayResult {
    /// total local cycle
    pub cycle: usize,
    /// for normal rows
    pub local_row_open_cycle: usize,
    /// row hit for read
    pub local_row_read_cycle: usize,
    /// row hit for write
    pub local_row_write_cycle: usize,
    /// cycle for compuation
    pub comp_cycle: usize,

    /// for evil row read/write miss
    pub local_row_open_cycle_evil: usize,
    /// row hit for read
    pub local_row_read_cycle_evil: usize,
    /// row hit for write
    pub local_row_write_cycle_evil: usize,

    // for remote rows that read by local subarray
    pub remote_row_read_cycle: usize,

    // remote result write by target subarray
    pub remote_row_write_cycle: usize,
    /// remote total cycle
    pub cycle_remote: usize,
}

impl ReportStats for SubArrayResult {
    fn report_stats(data_vec: &[Self]) -> std::collections::BTreeMap<String, (f64, f64, usize)> {
        let mut map = std::collections::BTreeMap::new();
        map.insert("cycle".to_string(), get_mean_std_max(data_vec, |x| x.cycle));
        map.insert(
            "local_row_open_cycle".to_string(),
            get_mean_std_max(data_vec, |x| x.local_row_open_cycle),
        );
        map.insert(
            "local_row_read_cycle".to_string(),
            get_mean_std_max(data_vec, |x| x.local_row_read_cycle),
        );
        map.insert(
            "local_row_write_cycle".to_string(),
            get_mean_std_max(data_vec, |x| x.local_row_write_cycle),
        );
        map.insert(
            "comp_cycle".to_string(),
            get_mean_std_max(data_vec, |x| x.comp_cycle),
        );
        map.insert(
            "local_row_open_cycle_evil".to_string(),
            get_mean_std_max(data_vec, |x| x.local_row_open_cycle_evil),
        );
        map.insert(
            "local_row_read_cycle_evil".to_string(),
            get_mean_std_max(data_vec, |x| x.local_row_read_cycle_evil),
        );
        map.insert(
            "local_row_write_cycle_evil".to_string(),
            get_mean_std_max(data_vec, |x| x.local_row_write_cycle_evil),
        );
        map.insert(
            "remote_row_read_cycle".to_string(),
            get_mean_std_max(data_vec, |x| x.remote_row_read_cycle),
        );
        map.insert(
            "remote_row_write_cycle".to_string(),
            get_mean_std_max(data_vec, |x| x.remote_row_write_cycle),
        );
        map.insert(
            "cycle_remote".to_string(),
            get_mean_std_max(data_vec, |x| x.cycle_remote),
        );
        map
    }
}
impl SubArrayResult {
    pub fn accumulate(&self, other: &mut SubArrayResult) {
        other.cycle += self.cycle;
        other.local_row_open_cycle += self.local_row_open_cycle;
        other.local_row_read_cycle += self.local_row_read_cycle;
        other.local_row_write_cycle += self.local_row_write_cycle;
        other.comp_cycle += self.comp_cycle;
        other.local_row_open_cycle_evil += self.local_row_open_cycle_evil;
        other.local_row_read_cycle_evil += self.local_row_read_cycle_evil;
        other.local_row_write_cycle_evil += self.local_row_write_cycle_evil;
        other.remote_row_read_cycle += self.remote_row_read_cycle;
        other.remote_row_write_cycle += self.remote_row_write_cycle;
        other.cycle_remote += self.cycle_remote;
    }
    pub fn reset(&mut self) {
        self.cycle = 0;
        self.local_row_open_cycle = 0;
        self.local_row_read_cycle = 0;
        self.local_row_write_cycle = 0;
        self.comp_cycle = 0;
        self.local_row_open_cycle_evil = 0;
        self.local_row_read_cycle_evil = 0;
        self.local_row_write_cycle_evil = 0;
        self.remote_row_read_cycle = 0;
        self.remote_row_write_cycle = 0;
        self.cycle_remote = 0;
    }
}

use std::collections::BTreeMap;

use statrs::statistics::Statistics;

pub trait ReportStats: Sized {
    fn report_stats(data_vec: &[Self]) -> BTreeMap<String, (f64, f64, usize)>;
}
pub fn get_mean_std_max<T>(data: &[T], mut mapper: impl FnMut(&T) -> usize) -> (f64, f64, usize) {
    let mean = data.into_iter().map(&mut mapper).map(|x| x as f64).mean();
    let std = data
        .into_iter()
        .map(&mut mapper)
        .map(|x| x as f64)
        .std_dev();
    let max = data.into_iter().map(&mut mapper).max().unwrap();
    (mean, std, max)
}

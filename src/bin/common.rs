#![allow(dead_code)]
use std::collections::BTreeMap;

use spmspm_pim::{analysis::remap_analyze::real_jump::RealJumpResult, pim::configv2::MappingType};

pub type RealJumpResultMap = BTreeMap<String, BTreeMap<MappingType, RealJumpResult>>;

fn main() {}

#[cfg(test)]
mod tests {
    fn test_iter_mut() {
        let mut a = vec![1, 2, 3];
        a.iter_mut().for_each(|x| *x += 1);
    }
}

#![allow(dead_code)]

use std::collections::BTreeMap;

use spmspm_pim::{analysis::remap_analyze::real_jump::RealJumpResult, pim::configv2::MappingType};

pub type RealJumpResultMap =
    BTreeMap<String, BTreeMap<MappingType, BTreeMap<usize, RealJumpResult>>>;

fn main() {}

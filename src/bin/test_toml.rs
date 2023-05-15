use std::{fs, hint::black_box};

use spmspm_pim::pim::configv2::ConfigV2;

fn main() {
    let current_time = std::time::Instant::now();
    let config_str =
        black_box(fs::read_to_string("configs/real_jump_same_bank-1-16.toml").unwrap());
    for _i in 0..10000 {
        let config_str = black_box(&config_str);
        let config: ConfigV2 = toml::from_str(config_str).unwrap();
        black_box(config);
    }
    println!("time: {:?}", current_time.elapsed());

    let current_time = std::time::Instant::now();

    for _i in 0..10000 {
        let config_str = include_str!("../../configs/real_jump_same_bank-1-16.toml");
        let _config: ConfigV2 = toml::from_str(config_str).unwrap();
        // black_box(config);
    }
    println!("time: {:?}", current_time.elapsed());
}

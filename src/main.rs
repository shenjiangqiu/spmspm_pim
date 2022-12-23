use std::env;

use eyre::Result;
use spmspm_pim::main_inner;

fn main() -> Result<()> {
    main_inner(env::args())
}

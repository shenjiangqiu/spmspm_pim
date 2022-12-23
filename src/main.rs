use std::env::args;

use eyre::Result;
use spmspm_pim::main_inner;

fn main() -> Result<()> {
    main_inner(args())
}

use std::env;

use eyre::Result;
use spmspm_pim::main_inner;

fn main() -> Result<()> {
    main_inner(env::args())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
        let path = PathBuf::from("../");
        println!("{:?}", path.parent());
    }
}

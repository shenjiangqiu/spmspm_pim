use std::env;

use eyre::Result;
use spmspm_pim::main_inner;

fn main() -> Result<()> {
    main_inner(env::args())
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Mutex};

    use rayon::prelude::{IntoParallelIterator, ParallelIterator};

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
        let path = PathBuf::from("../");
        println!("{:?}", path.parent());
    }

    // trait Window {
    //     type Item<'a>
    //     where
    //         Self: 'a;
    //     fn next(&mut self) -> Option<Self::Item<'_>>;
    // }

    // struct WindowMut<'a> {
    //     array: &'a mut [u8],
    //     current: usize,
    // }
    // impl<'a> WindowMut<'a> {
    //     fn new(array: &'a mut [u8], current: usize) -> Self {
    //         Self { array, current }
    //     }
    // }
    // impl<'a> Window for WindowMut<'a> {
    //     type Item<'b> = &'b mut [u8] where Self: 'b;

    //     fn next(&mut self) -> Option<Self::Item<'_>> {
    //         if self.current < self.array.len() {
    //             let out = Some(&mut self.array[self.current..self.current + 2]);
    //             self.current += 1;
    //             out
    //         } else {
    //             None
    //         }
    //     }
    // }

    #[test]
    fn multi_thread() {
        let a = Mutex::new(0);
        (0..10).into_par_iter().for_each(|_| {
            let mut a = a.lock().unwrap();
            *a += 1;
        });
        println!("{:?}", a);
        assert_eq!(*a.lock().unwrap(), 10);
    }
}

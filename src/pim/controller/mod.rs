pub trait Controller {}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};
    #[test]
    fn test() {
        thread::scope(|s| {
            for i in 1..100 {
                println!("i: {}", i);
                thread::sleep(Duration::from_secs(1));
                for i in 0..10000 {
                    s.spawn(move || {
                        println!("{}", i);
                    });
                }
            }

            thread::sleep(Duration::from_secs(10));
            for i in 0..10000 {
                s.spawn(move || {
                    println!("{}", i);
                });
            }
            thread::sleep(Duration::from_secs(10));
            for i in 0..10000 {
                s.spawn(move || {
                    println!("{}", i);
                });
            }
            thread::sleep(Duration::from_secs(10));
        });
    }
}

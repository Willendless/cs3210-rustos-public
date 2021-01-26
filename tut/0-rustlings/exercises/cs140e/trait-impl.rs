// FIXME: Make me pass! Diff budget: 25 lines.

<<<<<<< HEAD
#[derive(Debug)]
=======
// I AM NOT DONE

>>>>>>> skeleton/lab2
enum Duration {
    MilliSeconds(u64),
    Seconds(u32),
    Minutes(u16)
}

// What traits does `Duration` need to implement?

use std::cmp::Ordering;

fn to_milliseconds(d: &Duration) -> u64 {
    match d {
        Duration::MilliSeconds(a) => *a,
        Duration::Seconds(a) => *a as u64 * 1000,
        Duration::Minutes(a) => *a as u64 * 60_000,
    }
}

impl PartialEq for Duration {
    fn eq(&self, other: &Self) -> bool {
        if to_milliseconds(self) == to_milliseconds(other) {
            true
        } else {
            false
        }
    }
}

impl PartialOrd for Duration {
    fn partial_cmp(&self, other: &Duration) -> Option<Ordering> {
        let a = to_milliseconds(self);
        let b = to_milliseconds(other);
        Some(a.cmp(&b))
    }
}

#[test]
fn traits() {
    assert_eq!(Seconds(120), Minutes(2));
    assert_eq!(Seconds(420), Minutes(7));
    assert_eq!(MilliSeconds(420000), Minutes(7));
    assert_eq!(MilliSeconds(43000), Seconds(43));
}

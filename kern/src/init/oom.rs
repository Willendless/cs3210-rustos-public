use core::alloc::Layout;
use crate::console::{kprintln};

#[alloc_error_handler]
pub fn oom(_layout: Layout) -> ! {
    loop {
        kprintln!("oom")
    }
    panic!("OOM");
}

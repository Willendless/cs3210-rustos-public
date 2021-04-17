use core::alloc::Layout;
use crate::console::{kprintln};

#[alloc_error_handler]
pub fn oom(_layout: Layout) -> ! {
    error!("oom");
    loop {
    }
}

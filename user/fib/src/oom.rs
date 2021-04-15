use core::alloc::Layout;

#[alloc_error_handler]
pub fn oom(_layout: Layout) -> ! {
    kernel_api::syscall::exit();
}

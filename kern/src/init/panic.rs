use core::panic::PanicInfo;
use crate::console::kprintln;
use pi::gpio::Gpio;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    kprintln!("          The pi is overdone.");
    kprintln!("------------------ PANIC -------------------");
    kprintln!("");

    if let Some(location) = _info.location() {
        kprintln!("FILE: {}", location.file());
        kprintln!("LINE: {}", location.line());
        kprintln!("COL: {}", location.column());
        kprintln!("");
    } else {
        kprintln!("failed to get location information...");
    }

    kprintln!("Error: {:#?}", _info.payload().downcast_ref::<&str>());

    let led = Gpio::new(16);
    let mut led = led.into_output();
    led.clear();

    loop {

    }
}

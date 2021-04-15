use core::panic::PanicInfo;
use crate::console::kprintln;
use pi::gpio::Gpio;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kprintln!("          The pi is overdone.");
    kprintln!("------------------ PANIC -------------------");
    kprintln!("");

    if let Some(location) = info.location() {
        kprintln!("FILE: {}", location.file());
        kprintln!("LINE: {}", location.line());
        kprintln!("COL: {}", location.column());
        kprintln!("");
    } else {
        kprintln!("failed to get location information...");
    }

    if let Some(s) = info.payload().downcast_ref::<&str>() {
        kprintln!("Error: {:?}", s);
    } else {
        kprintln!("Error: no info");
    }

    let led = Gpio::new(16);
    let mut led = led.into_output();
    led.clear();

    loop {
    }
}

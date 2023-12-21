#![no_std]
#![no_main]

use core::fmt::Write;
use cortex_m_rt::entry;
use panic_halt as _;
mod init;

#[entry]
fn main() -> ! {
    let mut car = init::CarPins::new();
    writeln!(car.display, "init").unwrap();
    loop {
        // car.display.clear().unwrap();
        car.go_with_openmv();
    }
}

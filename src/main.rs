#![no_std]
#![no_main]

use core::fmt::Write;
use cortex_m_rt::entry;
// use embedded_hal::blocking::delay::DelayMs;
use panic_halt as _;
mod init;

#[entry]
fn main() -> ! {
    let mut car = init::CarPins::new();
    writeln!(car.display, "init").unwrap();
    loop {
        // car.read();
        // car.delay.delay_ms(1000 as u16);
        car.go_with_openmv();
    }
}

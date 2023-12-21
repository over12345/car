#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;
mod init;

#[entry]
fn main() -> ! {
    let mut car = init::CarPins::new();
    let _ = car.display.clear();
    loop {
        car = car.read();
    }
}

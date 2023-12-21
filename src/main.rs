#![no_std]
#![no_main]

// use core::fmt::Write;

use cortex_m_rt::{entry, exception, ExceptionFrame};
use panic_halt as _;

mod init;

#[entry]
fn main() -> ! {
    let mut car = init::CarPins::new();
    loop {
        car = car.read();
    }
}

#[exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    panic!("{:#?}", ef);
}

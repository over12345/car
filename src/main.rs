#![no_std]
#![no_main]

use cortex_m_rt::{entry, exception, ExceptionFrame};
use panic_halt as _;

mod init;

#[entry]
fn main() -> ! {
    let mut pins = init::CarPins::new();
    pins.flashln("init compelt");
    loop {}
}

#[exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    panic!("{:#?}", ef);
}

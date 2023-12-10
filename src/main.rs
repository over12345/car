#![no_std]
#![no_main]
#![feature(const_float_bits_conv)]
#![feature(const_fn_floating_point_arithmetic)]

extern crate alloc;
use alloc::string::ToString;
use cortex_m_rt::entry;
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
mod ldc;
use panic_halt as _;
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};
use stm32f1xx_hal::{
    i2c::{BlockingI2c, DutyCycle, Mode},
    prelude::*,
    stm32,
};

use embedded_alloc::Heap;

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().unwrap();
    let mut flash = dp.FLASH.constrain();
    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.freeze(&mut flash.acr);
    let mut afio = dp.AFIO.constrain();
    let mut gpiob = dp.GPIOB.split();

    let i2c1 = BlockingI2c::i2c1(
        dp.I2C1,
        (
            gpiob.pb8.into_alternate_open_drain(&mut gpiob.crh),
            gpiob.pb9.into_alternate_open_drain(&mut gpiob.crh),
        ),
        &mut afio.mapr,
        Mode::Fast {
            frequency: 400_000.Hz(),
            duty_cycle: DutyCycle::Ratio2to1,
        },
        clocks,
        1000,
        10,
        1000,
        1000,
    );
    let i2c2 = BlockingI2c::i2c2(
        dp.I2C2,
        (
            gpiob.pb10.into_alternate_open_drain(&mut gpiob.crh),
            gpiob.pb11.into_alternate_open_drain(&mut gpiob.crh),
        ),
        Mode::Fast {
            frequency: 400_000.Hz(),
            duty_cycle: DutyCycle::Ratio2to1,
        },
        clocks,
        1000,
        10,
        1000,
        1000,
    );

    let interface = I2CDisplayInterface::new(i2c2);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().unwrap();
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    let adr = u8::from_str_radix("2b", 16).unwrap(); // I2C 地址
    let mut ldc = ldc::Ldc::new(i2c1, adr);
    ldc.reset().unwrap();
    let div = ldc::Fsensor::from_inductance_capacitance(19.0, 100.0).to_clock_dividers(None);
    ldc.set_clock_dividers(ldc::Channel::One, div).unwrap();
    ldc.set_conv_settling_time(ldc::Channel::One, 40).unwrap();
    ldc.set_ref_count_conv_interval(ldc::Channel::One, 0x0546)
        .unwrap();
    ldc.set_sensor_drive_current(ldc::Channel::One, 0b01110)
        .unwrap();
    ldc.set_mux_config(
        ldc::MuxConfig::default()
            .with_auto_scan(true)
            .with_deglitch_filter_bandwidth(ldc::Deglitch::ThreePointThreeMHz),
    )
    .unwrap();
    ldc.set_config(ldc::Config::default()).unwrap();
    ldc.set_error_config(
        ldc::ErrorConfig::default().with_amplitude_high_error_to_data_register(true),
    )
    .unwrap();

    // timing ignored because polling with a cp2112 with no delays is slow enough already
    // outputting just newline separated numbers so you can feed it into https://github.com/mogenson/ploot
    loop {
        Text::with_baseline(
            &ldc.read_data_24bit(ldc::Channel::Zero).unwrap().to_string(),
            Point::new(0, 16),
            text_style,
            Baseline::Top,
        )
        .draw(&mut display)
        .unwrap();
        display.flush().unwrap();
    }
}

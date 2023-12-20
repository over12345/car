#![deny(unsafe_code)]
#![no_main]
#![no_std]

//! Testing PWM output for custom pin combinations
mod init;

use panic_halt as _;

use stm32f1xx_hal::{
    pac,
    prelude::*,
    timer::{Tim2NoRemap, Timer},
};

use cortex_m_rt::entry;
use tb6612fng::Motor;
use tb6612fng::{DriveCommand, Tb6612fng};
#[entry]
fn left1_init() -> ! {
    // Get access to the device specific peripherals from the peripheral access crate
    let dp = pac::Peripherals::take().unwrap();
    // Take ownership over the raw flash and rcc devices and convert them into the corresponding
    // HAL structs
    let mut flash = dp.FLASH.constrain();
    let rcc = dp.RCC.constrain();

    // Freeze the configuration of all the clocks in the system and store the frozen frequencies in
    // `clocks`
    let clocks = rcc.cfgr.freeze(&mut flash.acr);
    let mut gpioa = dp.GPIOA.split();
    let pins_pwm = (
        gpioa.pa0.into_alternate_push_pull(&mut gpioa.crl),
        gpioa.pa1.into_alternate_push_pull(&mut gpioa.crl),
    );

    let mut afio = dp.AFIO.constrain();
    let pwm_hz =
        Timer::new(dp.TIM2, &clocks).pwm_hz::<Tim2NoRemap, _, _>(pins_pwm, &mut afio.mapr, 1.kHz());
    let mut pwm_channel = pwm_hz.split();
    pwm_channel.0.set_duty(50);

    let motor_a_in1 = gpioa.pa10.into_push_pull_output(&mut gpioa.crh); //电机A的输入信号1的引脚与pa0相连，用的是tim2的通道1
    let motor_a_in2 = gpioa.pa11.into_push_pull_output(&mut gpioa.crh); //电机A的输入信号2的引脚与pa1相连，用的是tim2的通道2
    let motor_b_in1 = gpioa.pa12.into_push_pull_output(&mut gpioa.crh); //电机B的输入信号1的引脚与pa2相连，用的是tim2的通道3
    let motor_b_in2 = gpioa.pa9.into_push_pull_output(&mut gpioa.crh); //电机B的输入信号2的引脚与pa3相连，用的是tim2的通道4
    let standby = gpioa.pa3.into_push_pull_output(&mut gpioa.crl);
    // let mut motor_a = Motor::new(motor_a_in1, motor_a_in2, pwm_channel.0);
    // let mut motor_b = Motor::new(motor_b_in1, motor_b_in2, pwm_channel.1);
    let mut controller = Tb6612fng::new(
        motor_a_in1,
        motor_a_in2,
        pwm_channel.0,
        motor_b_in1,
        motor_b_in2,
        pwm_channel.1,
        standby,
    );
    // // let mut controller = Tb6612fng::new(motor_a_in1, motor_a_in2, pwm_channel.0, motor_b_in1, motor_b_in2, pwm_channel.1, standby);

    // Tb6612fng::disable_standby(&mut controller);

    //

    //   let controller = Tb6612fng::new(
    //     motor_a_in1,
    //     motor_a_in2,
    //     motor_a_pwm,
    //     motor_b_in1,
    //     motor_b_in2,
    //     motor_b_pwm,
    //     standby,
    // );

    //  a0.set_duty(a1.get_max_duty());
    //  a1.enable();

    // Set up the timer as a PWM output. If selected pins may correspond to different remap options,
    // then you must specify the remap generic parameter. Otherwise, if there is no such ambiguity,
    // the remap generic parameter can be omitted without complains from the compiler.

    // Start using the channels

    loop {
        controller.motor_a.drive_forward(100).unwrap();
    }
}

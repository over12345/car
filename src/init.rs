use fugit::RateExtU32;
//主要需要使用constrain来从外设对象上分离子对象，该功能在xx::xxExt里
use stm32f1xx_hal::{afio::AfioExt, flash::FlashExt, gpio::GpioExt, rcc::RccExt};
use stm32f1xx_hal::{gpio, i2c, pac, timer};
//电机控制
use tb6612fng::Tb6612fng;
//I2C屏幕
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use ssd1306::{mode::BufferedGraphicsMode, prelude::*, I2CDisplayInterface, Ssd1306};

/// 这里将包含所有需要引脚的初始话和定义调度器结构体

pub struct CarPins {
    motor: Tb6612fng<
        gpio::Pin<'A', 10, gpio::Output>,
        gpio::Pin<'A', 11, gpio::Output>,
        timer::PwmChannel<pac::TIM2, 0>, //PA0
        gpio::Pin<'A', 12, gpio::Output>,
        gpio::Pin<'A', 9, gpio::Output>,
        timer::PwmChannel<pac::TIM2, 1>, //PA1
        gpio::Pin<'A', 3, gpio::Output>,
    >,
}

impl CarPins {
    pub fn new() -> Self {
        let dp = pac::Peripherals::take().unwrap();
        let mut flash = dp.FLASH.constrain();
        let rcc = dp.RCC.constrain();
        let clocks = rcc.cfgr.freeze(&mut flash.acr);
        let mut gpioa = dp.GPIOA.split();
        let mut gpiob = dp.GPIOB.split();

        let pins_pwm = (
            gpioa.pa0.into_alternate_push_pull(&mut gpioa.crl),
            gpioa.pa1.into_alternate_push_pull(&mut gpioa.crl),
        );
        let mut afio = dp.AFIO.constrain();
        let (mut pwm_0, mut pwm_1) = timer::Timer::new(dp.TIM2, &clocks)
            .pwm_hz::<timer::Tim2NoRemap, _, _>(pins_pwm, &mut afio.mapr, 100.Hz())
            .split();
        pwm_0.set_duty(50);
        pwm_1.set_duty(50);
        let motor_a_in1 = gpioa.pa10.into_push_pull_output(&mut gpioa.crh); //电机A的输入信号1的引脚与pa0相连，用的是tim2的通道1
        let motor_a_in2 = gpioa.pa11.into_push_pull_output(&mut gpioa.crh); //电机A的输入信号2的引脚与pa1相连，用的是tim2的通道2
        let motor_b_in1 = gpioa.pa12.into_push_pull_output(&mut gpioa.crh); //电机B的输入信号1的引脚与pa2相连，用的是tim2的通道3
        let motor_b_in2 = gpioa.pa9.into_push_pull_output(&mut gpioa.crh); //电机B的输入信号2的引脚与pa3相连，用的是tim2的通道4
        let standby = gpioa.pa3.into_push_pull_output(&mut gpioa.crl);
        let scl = gpiob.pb8.into_alternate_open_drain(&mut gpiob.crh);
        let sda = gpiob.pb9.into_alternate_open_drain(&mut gpiob.crh);
        let i2c = BlockingI2c::i2c1(
            dp.I2C1,
            (scl, sda),
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
        let interface = I2CDisplayInterface::new(i2c);
        let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
        display.init().unwrap();

        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();

        Text::with_baseline("Hello world!", Point::zero(), text_style, Baseline::Top)
            .draw(&mut display)
            .unwrap();

        Text::with_baseline("Hello Rust!", Point::new(0, 16), text_style, Baseline::Top)
            .draw(&mut display)
            .unwrap();

        display.flush().unwrap();
        let mut controller: Tb6612fng<
            gpio::Pin<'A', 10, gpio::Output>,
            gpio::Pin<'A', 11, gpio::Output>,
            _,
            gpio::Pin<'A', 12, gpio::Output>,
            gpio::Pin<'A', 9, gpio::Output>,
            _,
            gpio::Pin<'A', 3, gpio::Output>,
        > = Tb6612fng::new(
            motor_a_in1,
            motor_a_in2,
            pwm_0,
            motor_b_in1,
            motor_b_in2,
            pwm_1,
            standby,
        );
        Self { motor: controller }
    }
}

// fn drive(gpioa: gpio::gpioa::Parts) {
//     let driver = tb6612fng::Motor::new(gpioa.pa10, in2, pwm)
// }

use core::fmt::Write;

use fugit::RateExtU32;
//主要需要使用constrain来从外设对象上分离子对象，该功能在xx::xxExt里
use stm32f1xx_hal::{afio::AfioExt, flash::FlashExt, gpio::GpioExt, rcc::RccExt};
use stm32f1xx_hal::{gpio, i2c, pac, timer};
//电机控制
use tb6612fng::Tb6612fng;
//I2C屏幕，终端模式
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use ssd1306::{mode::BufferedGraphicsMode, prelude::*, I2CDisplayInterface, Ssd1306};

/// 这里将包含所有需要引脚的初始话和定义调度器结构体

pub struct CarPins {
    //马达
    motor: Tb6612fng<
        gpio::Pin<'A', 10, gpio::Output>,
        gpio::Pin<'A', 11, gpio::Output>,
        timer::PwmChannel<pac::TIM2, 0>, //PA0
        gpio::Pin<'A', 12, gpio::Output>,
        gpio::Pin<'A', 9, gpio::Output>,
        timer::PwmChannel<pac::TIM2, 1>, //PA1
        gpio::Pin<'A', 3, gpio::Output>,
    >,
    //i2c屏幕
    display: Ssd1306<
        I2CInterface<
            i2c::BlockingI2c<
                pac::I2C1,
                (
                    gpio::Pin<'B', 8, gpio::Alternate<gpio::OpenDrain>>,
                    gpio::Pin<'B', 9, gpio::Alternate<gpio::OpenDrain>>,
                ),
            >,
        >,
        DisplaySize128x64,
        ssd1306::mode::TerminalMode,
    >,
}

impl CarPins {
    pub fn new() -> Self {
        //初始化外设桥，时钟
        let dp = pac::Peripherals::take().unwrap();
        let mut flash = dp.FLASH.constrain();
        let rcc = dp.RCC.constrain();
        let clocks = rcc.cfgr.freeze(&mut flash.acr);
        let mut gpioa = dp.GPIOA.split();
        let mut gpiob = dp.GPIOB.split();

        //获取所需引脚，为引脚设定功能，并启用时钟。载入对应功能控制对象

        //电机PWM控制
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

        //电机IO控制
        let motor_a_in1 = gpioa.pa10.into_push_pull_output(&mut gpioa.crh);
        let motor_a_in2 = gpioa.pa11.into_push_pull_output(&mut gpioa.crh);
        let motor_b_in1 = gpioa.pa12.into_push_pull_output(&mut gpioa.crh);
        let motor_b_in2 = gpioa.pa9.into_push_pull_output(&mut gpioa.crh);
        let standby = gpioa.pa3.into_push_pull_output(&mut gpioa.crl);

        //电机控制对象
        let mut motor = Tb6612fng::new(
            motor_a_in1,
            motor_a_in2,
            pwm_0,
            motor_b_in1,
            motor_b_in2,
            pwm_1,
            standby,
        );

        //I2C引脚与初始化I2C
        let scl = gpiob.pb8.into_alternate_open_drain(&mut gpiob.crh);
        let sda = gpiob.pb9.into_alternate_open_drain(&mut gpiob.crh);
        let i2c = i2c::BlockingI2c::i2c1(
            dp.I2C1,
            (scl, sda),
            &mut afio.mapr,
            i2c::Mode::Fast {
                frequency: 400_000.Hz(),
                duty_cycle: i2c::DutyCycle::Ratio2to1,
            },
            clocks,
            1000,
            10,
            1000,
            1000,
        );
        let interface = I2CDisplayInterface::new(i2c);
        //屏幕控制对象
        let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_terminal_mode();
        display.init().unwrap();

        Self { motor, display }
    }
    pub fn flashln(&mut self, string: &str) {
        let _ = self.display.clear();
        self.display.write_str(string).unwrap();
    }
}

// fn drive(gpioa: gpio::gpioa::Parts) {
//     let driver = tb6612fng::Motor::new(gpioa.pa10, in2, pwm)
// }

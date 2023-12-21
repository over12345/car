use core::fmt::Write;
use core::str::from_utf8_unchecked;
use embedded_hal::blocking::delay::DelayMs;
use nb::block;
//一些单位的trait
use fugit::RateExtU32;
use stm32f1xx_hal::time::U32Ext;
//主要需要使用constrain来从外设对象上分离子对象，该功能在xx::xxExt里
use stm32f1xx_hal::dma::DmaExt;
use stm32f1xx_hal::prelude::_stm32f4xx_hal_timer_SysCounterExt;
use stm32f1xx_hal::{afio::AfioExt, flash::FlashExt, gpio::GpioExt, rcc::RccExt};
use stm32f1xx_hal::{gpio, i2c, pac, serial, timer};
//电机控制
use tb6612fng::Tb6612fng;
//I2C屏幕，终端模式
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};
//JSON解析
use serde_json_core::de;

/// 这里将包含所有需要引脚的初始话和定义调度器结构体

pub struct CarPins {
    //马达
    _motor: Tb6612fng<
        gpio::Pin<'A', 10, gpio::Output>,
        gpio::Pin<'A', 11, gpio::Output>,
        timer::PwmChannel<pac::TIM2, 0>, //PA0
        gpio::Pin<'A', 12, gpio::Output>,
        gpio::Pin<'A', 9, gpio::Output>,
        timer::PwmChannel<pac::TIM2, 1>, //PA1
        gpio::Pin<'A', 3, gpio::Output>,
    >,
    //i2c屏幕
    pub display: Ssd1306<
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
    // pub openmv:
    //     serial::Serial<pac::USART3, (gpio::Pin<'B', 10, gpio::Alternate>, gpio::Pin<'B', 11>)>,
    pub delay: timer::SysDelay,
    pub led: gpio::Pin<'A', 8, gpio::Output>,
    pub rx: stm32f1xx_hal::dma::RxDma<serial::Rx<pac::USART3>, stm32f1xx_hal::dma::dma1::C3>,
}

struct mes {
    theta: i16,
    rho: i16,
    ain: [bool; 2],
    bin: [bool; 2],
    ch: [i8; 2],
}

impl CarPins {
    pub fn new() -> Self {
        //初始化外设桥，时钟
        let dp = pac::Peripherals::take().unwrap();
        let cp = cortex_m::Peripherals::take().unwrap();
        let mut flash = dp.FLASH.constrain();
        let rcc = dp.RCC.constrain();
        let clocks = rcc.cfgr.freeze(&mut flash.acr);
        let mut gpioa = dp.GPIOA.split();
        let mut gpiob = dp.GPIOB.split();
        let delay: timer::SysDelay = cp.SYST.delay(&clocks);

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
        let mut led: gpio::Pin<'A', 8, gpio::Output> =
            gpioa.pa8.into_push_pull_output(&mut gpioa.crh);

        //电机控制对象
        let motor = Tb6612fng::new(
            motor_a_in1,
            motor_a_in2,
            pwm_0,
            motor_b_in1,
            motor_b_in2,
            pwm_1,
            standby,
        );
        led.set_high();

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

        //串口通信引脚
        let tx = gpiob.pb10.into_alternate_push_pull(&mut gpiob.crh);
        let rx = gpiob.pb11;
        let openmv = serial::Serial::new(
            dp.USART3,
            (tx, rx),
            &mut afio.mapr,
            serial::Config::default().baudrate(19_200_u32.bps()),
            &clocks,
        );
        let channels = dp.DMA1.split();
        let rx = openmv.rx.with_dma(channels.3);

        Self {
            _motor: motor,
            display,
            rx,
            delay,
            led,
        }
    }
    pub fn read(&mut self) {
        self.led.set_low();
        let mut byte = match block!(self.openmv.rx.read()) {
            Ok(a) => a,
            Err(e) => {
                let _ = self.display.clear();
                writeln!(self.display, "g156: {:?}", e).unwrap();
                self.delay.delay_ms(1000 as u16);
                0
            }
        };
        self.delay.delay_ms(100 as u16);
        self.led.set_high();
        while byte != b'{' {
            if self.openmv.is_rx_not_empty() {
                match block!(self.openmv.rx.read()) {
                    Ok(a) => byte = a,
                    Err(e) => {
                        let _ = self.display.clear();
                        writeln!(self.display, "g169: {:?}", e).unwrap();
                        self.delay.delay_ms(1000 as u16);
                    }
                };
            }
        }

        let mut json = [0 as u8; 74];
        for i in 0..74 {
            json[i] = match block!(self.openmv.rx.read()) {
                Ok(a) => a,
                Err(e) => {
                    let _ = self.display.clear();
                    writeln!(self.display, "g:182 {:?}", e).unwrap();
                    self.delay.delay_ms(1000 as u16);
                    0
                }
            };
        }

        let json = unsafe { from_utf8_unchecked(&json) };

        //let json = de::from_str(json).unwrap();
        self.display.write_str(json).unwrap();
    }
}

pub fn painc(
    display: &mut Ssd1306<
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
    string: &str,
) {
    writeln!(display, "g: {}", string).unwrap();
}

// fn drive(gpioa: gpio::gpioa::Parts) {
//     let driver = tb6612fng::Motor::new(gpioa.pa10, in2, pwm)
// }

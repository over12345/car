use core::fmt::Write;
use core::str::from_utf8;
use cortex_m::singleton;
// use nb::block;
//一些单位的trait
use fugit::RateExtU32;
use nb::block;
use stm32f1xx_hal::time::U32Ext;
//主要需要使用constrain来从外设对象上分离子对象，该功能在xx::xxExt里
// use stm32f1xx_hal::dma::DmaExt;
use stm32f1xx_hal::prelude::_stm32f4xx_hal_timer_SysCounterExt;
use stm32f1xx_hal::{afio::AfioExt, flash::FlashExt, gpio::GpioExt, rcc::RccExt};

use stm32f1xx_hal::{gpio, i2c, pac, serial, timer};
//电机控制
use tb6612fng::Tb6612fng;
//I2C屏幕，终端模式
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};
//JSON解析
use lite_json::json_parser::parse_json;
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
    pub delay: timer::SysDelay,
    // pub openmv:
    //     serial::Serial<pac::USART3, (gpio::Pin<'B', 10, gpio::Alternate>, gpio::Pin<'B', 11>)>,
    pub rx: serial::Rx<pac::USART3>, //: stm32f1xx_hal::dma::RxDma<serial::Rx<pac::USART3>, stm32f1xx_hal::dma::dma1::C3>,
    pub tx: serial::Tx<pac::USART3>,
}

struct Mes {
    theta: i16,
    rho: i16,
    ain: [bool; 2],
    bin: [bool; 2],
    ch: [i16; 2],
}

trait DisplaySsd {
    fn ssdwrap(
        self,
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
    ) -> u8;
}

impl<E: core::fmt::Debug> DisplaySsd for core::result::Result<u8, E> {
    fn ssdwrap(
        self,
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
    ) -> u8 {
        match self {
            Ok(a) => a,
            Err(e) => {
                display.clear().unwrap();
                writeln!(display, "{:?}", e).unwrap();
                0
            }
        }
    }
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
        let delay = cp.SYST.delay(&clocks);

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
        let mut led = gpioa.pa8.into_push_pull_output(&mut gpioa.crh);

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
            serial::Config::default().baudrate(9_600_u32.bps()),
            &clocks,
        );
        let tx: serial::Tx<pac::USART3> = openmv.tx;
        let rx: serial::Rx<pac::USART3> = openmv.rx;
        // let rx = openmv.rx.with_dma(channels.3);
        // let channels = dp.DMA1.split();
        // let rx = openmv.rx.with_dma(channels.3);

        Self {
            _motor: motor,
            display,
            delay,
            rx,
            tx,
        }
    }
    pub fn read(&mut self) -> lite_json::JsonValue {
        // let mut json = [0 as u8; 150];
        block!(self.tx.write(b'0')).unwrap();
        writeln!(self.display, "write 0").unwrap();
        // self.delay.delay_ms(10 as u16);
        let buf = singleton!(: [u8; 150] = [0; 150]).unwrap();
        // use stm32f1xx_hal::dma::ReadDma;
        for index in 0..74 {
            buf[index] = self.rx.read().ssdwrap(&mut self.display);
        }
        let json = from_utf8(buf).unwrap();
        write!(self.display, "{}", json).unwrap();
        parse_json(json).unwrap()
    }
}

// fn drive(gpioa: gpio::gpioa::Parts) {
//     let driver = tb6612fng::Motor::new(gpioa.pa10, in2, pwm)
// }

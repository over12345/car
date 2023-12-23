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
use cortex_m::prelude::_embedded_hal_adc_OneShot;
use stm32f1xx_hal::prelude::_stm32f4xx_hal_timer_SysCounterExt;
use stm32f1xx_hal::{afio::AfioExt, flash::FlashExt, gpio::GpioExt, rcc::RccExt};

use stm32f1xx_hal::{adc, gpio, i2c, pac, serial, timer};
//电机控制
use tb6612fng::Tb6612fng;
//I2C屏幕，终端模式
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};
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
    mental: (adc::Adc<pac::ADC1>, gpio::Pin<'B', 0, gpio::Analog>),
    pub delay: timer::SysDelay,
    // pub openmv:
    //     serial::Serial<pac::USART3, (gpio::Pin<'B', 10, gpio::Alternate>, gpio::Pin<'B', 11>)>,
    pub rx: serial::Rx<pac::USART3>, //: stm32f1xx_hal::dma::RxDma<serial::Rx<pac::USART3>, stm32f1xx_hal::dma::dma1::C3>,
    pub tx: serial::Tx<pac::USART3>,
    pub integral:i32,
        
    pub derivative:i32,

    pub last_error:i32,
}

pub struct Mes {
    pwm: [u8; 2],
    direction: [bool; 2],
}

impl Mes {
    fn new(pwm: [u8; 2], direction: [bool; 2]) -> Self {
        Self { pwm, direction }
    }
}

trait DisplaySsd<T> {
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
    ) -> T;
}

impl<E: core::fmt::Debug> DisplaySsd<()> for core::result::Result<(), E> {
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
    ) -> () {
        match self {
            Ok(()) => (),
            Err(e) => {
                display.clear().unwrap();
                writeln!(display, "{:?}", e).unwrap();
                ()
            }
        }
    }
}

impl<E: core::fmt::Debug> DisplaySsd<u8> for core::result::Result<u8, E> {
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

        // 设置 ADC
        let adc1 = adc::Adc::adc1(dp.ADC1, clocks);
        let ch0 = gpiob.pb0.into_analog(&mut gpiob.crl);

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
        motor.enable_standby();

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
        display.clear().unwrap();

        //串口通信引脚
        let tx = gpiob.pb10.into_alternate_push_pull(&mut gpiob.crh);
        let rx = gpiob.pb11;
        let openmv = serial::Serial::new(
            dp.USART3,
            (tx, rx),
            &mut afio.mapr,
            serial::Config::default().baudrate(115200_u32.bps()),
            &clocks,
        );
        let tx: serial::Tx<pac::USART3> = openmv.tx;
        let rx: serial::Rx<pac::USART3> = openmv.rx;
        // let rx = openmv.rx.with_dma(channels.3);
        // let channels = dp.DMA1.split();
        // let rx = openmv.rx.with_dma(channels.3);

        Self {
            motor,
            mental: (adc1, ch0),
            display,
            delay,
            rx,
            tx,
            integral:0,
            derivative:0,
            last_error:0,
        }
    }
    pub fn read(&mut self) -> Mes {
        // let mut json = [0 as u8; 150];
        writeln!(self.display, "write 0").unwrap();
        let mut buf = [0 as u8; 5];
        block!(self.tx.write(b'0')).unwrap();
        for index in 0..5 {
            buf[index] = block!(self.rx.read()).ssdwrap(&mut self.display);
        }
        let offset = 15;
        let error = 5*buf[0] +10*buf[1] + 15*buf[2] + 20*buf[3] + 25*buf[4] -offset;
        let error = error as i32;
        let kp = 5;
        let ki = 1 ;
        let kd = 10;
       
    
        self.integral += error;
        self.derivative = error - self.last_error ;
        self.last_error = error;
        let turn = kp*error + ki*self.integral + kd*self.derivative ;
        let turn = turn as u8;
        // write!(self.display, "{:?}", buf).unwrap();
        Mes {
            
            pwm: [50 + turn, 50 - turn],
            direction: [
                match buf[0] {
                    48 => true,
                    _ => false,
                },
                match buf[1] {
                    48 => true,
                    _ => false,
                },
            ],
        }
    }
    pub fn go_with_openmv(&mut self) {
        let mes = self.read();
        let data: u16 = self.mental.0.read(&mut self.mental.1).unwrap();
        writeln!(self.display, "{}|{}", mes.pwm[0], mes.pwm[1]).unwrap();
        if data >= 2000 {
            self.motor.disable_standby();
        } else {
            self.motor.enable_standby();
        }
        match mes.direction {
            //按理来说，不可能后退，所以，代码就这样了。根据已有的Openmv代码来看，大概率只会命中第一种情况
            [true, true] => {
                self.motor
                    .motor_a
                    .drive_forward(mes.pwm[1])
                    .ssdwrap(&mut self.display);
                self.motor
                    .motor_b
                    .drive_forward(mes.pwm[0])
                    .ssdwrap(&mut self.display);
            }
            [true, false] => {
                self.motor
                    .motor_a
                    .drive_forward(mes.pwm[1])
                    .ssdwrap(&mut self.display);
                self.motor
                    .motor_b
                    .drive_backwards(mes.pwm[0])
                    .ssdwrap(&mut self.display);
            }
            [false, true] => {
                self.motor
                    .motor_a
                    .drive_backwards(mes.pwm[1])
                    .ssdwrap(&mut self.display);
                self.motor
                    .motor_b
                    .drive_forward(mes.pwm[0])
                    .ssdwrap(&mut self.display);
            }
            [false, false] => self.motor.enable_standby(),
        }
    }
}

// fn drive(gpioa: gpio::gpioa::Parts) {
//     let driver = tb6612fng::Motor::new(gpioa.pa10, in2, pwm)
// }

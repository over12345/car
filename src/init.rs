use fugit::RateExtU32;
//主要需要使用constrain来从外设对象上分离子对象，该功能在xx::xxExt里
use stm32f1xx_hal::{afio::AfioExt, flash::FlashExt, gpio::GpioExt, rcc::RccExt};
use stm32f1xx_hal::{gpio, pac, timer};
//电机控制
use tb6612fng::Tb6612fng;

/// 这里将包含所有需要引脚的初始话和定义调度器结构体

struct CarPins {
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
    fn new() -> Self {
        let dp = pac::Peripherals::take().unwrap();
        let mut flash = dp.FLASH.constrain();
        let rcc = dp.RCC.constrain();
        let clocks = rcc.cfgr.freeze(&mut flash.acr);
        let mut gpioa = dp.GPIOA.split();
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

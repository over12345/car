use stm32f1xx_hal::flash::FlashExt;
use stm32f1xx_hal::gpio::GpioExt;
use stm32f1xx_hal::prelude::_stm32_hal_rcc_RccExt;
/// 这里讲包含所有需要引脚的初始话和定义调度器结构体
use stm32f1xx_hal::{gpio, pac};
use tb6612fng::Tb6612fng;

struct CarPins {
    io: (gpio::gpioa::Parts, gpio::gpiob::Parts),
}

impl CarPins {
    fn new() -> Self {
        
    // 获取外设对象
    let dp = pac::Peripherals::take().unwrap();
     // 获取闪存对象，用于堆分配
     let mut flash = dp.FLASH.constrain();
     //获取外设桥上时钟对象
     let rcc = dp.RCC.constrain();
    //启动所用时钟
     let clocks = rcc.cfgr.freeze(&mut flash.acr);
     let mut gpioa = dp.GPIOA.split();
     let pins_pwm = (
        gpioa.pa0.into_alternate_push_pull(&mut gpioa.crl),
        gpioa.pa1.into_alternate_push_pull(&mut gpioa.crl),
     );

    let pwm_hz = Timer::new(dp.TIM2,&clocks).pwm_hz::<Tim2NoRemap, _, _>(pins_pwm, &mut afio.mapr, 1.kHz());
    let mut pwm_channel = pwm_hz.split();
    pwm_channel.0.set_duty(50);
    let motor_a_in1:  = gpioa.pa10.into_push_pull_output(&mut gpioa.crh);         //电机A的输入信号1的引脚与pa0相连，用的是tim2的通道1
    let motor_a_in2 = gpioa.pa11.into_push_pull_output(&mut gpioa.crh);         //电机A的输入信号2的引脚与pa1相连，用的是tim2的通道2
    let motor_b_in1 = gpioa.pa12.into_push_pull_output(&mut gpioa.crh);          //电机B的输入信号1的引脚与pa2相连，用的是tim2的通道3
    let motor_b_in2 = gpioa.pa9.into_push_pull_output(&mut gpioa.crh);          //电机B的输入信号2的引脚与pa3相连，用的是tim2的通道4
    let standby = gpioa.pa3.into_push_pull_output(&mut gpioa.crl);
    // let mut motor_a = Motor::new(motor_a_in1, motor_a_in2, pwm_channel.0);
    // let mut motor_b = Motor::new(motor_b_in1, motor_b_in2, pwm_channel.1);
    let mut controller = Tb6612fng::new(motor_a_in1, motor_a_in2, pwm_channel.0, motor_b_in1,motor_b_in2, pwm_channel.1,standby);
    // // let mut controller = Tb6612fng::new(motor_a_in1, motor_a_in2, pwm_channel.0, motor_b_in1, motor_b_in2, pwm_channel.1, standby);

        Self {
            io: (dp.GPIOA.split(), dp.GPIOB.split()),
        }
    }
}

fn drive(gpioa: gpio::gpioa::Parts) {
    let driver = tb6612fng::Motor::new(gpioa.pa10, in2, pwm)
}

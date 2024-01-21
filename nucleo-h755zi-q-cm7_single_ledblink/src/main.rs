#![deny(warnings)]
#![no_main]
#![no_std]

use cortex_m_rt::entry;
use stm32h7xx_hal::{pac, prelude::*};
use log::info;

//use rtt_target::{rprintln, rtt_init_print, ChannelMode::BlockIfFull};

#[macro_use]
mod utilities;

#[entry]
fn main() -> ! {

    utilities::logger::init();
    //rtt_init_print!(BlockIfFull);
    info!("wake cm7");

    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    // Constrain and Freeze power
    info!("Setup PWR...                  ");
    let pwr = dp.PWR.constrain();
    let pwrcfg = example_power!(pwr).freeze();

    // Constrain and Freeze clock
    info!("Setup RCC...                  ");
    let rcc = dp.RCC.constrain();
    let ccdr = rcc.sys_ck(100.MHz()).freeze(pwrcfg, &dp.SYSCFG);

    let gpiob = dp.GPIOB.split(ccdr.peripheral.GPIOB);

    // Configure PE1 as output.
    let mut led = gpiob.pb0.into_push_pull_output();

    // Get the delay provider.
    let mut delay = cp.SYST.delay(ccdr.clocks);

    info!("start blinking LD1                  ");
    loop {
        led.set_high();
        delay.delay_ms(500_u16);

        led.set_low();
        delay.delay_ms(500_u16);
    }
}

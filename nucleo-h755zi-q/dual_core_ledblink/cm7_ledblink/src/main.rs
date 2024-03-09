#![deny(warnings)]
#![no_main]
#![no_std]

use cortex_m_rt::entry;
use stm32h7xx_hal::{pac, prelude::*};
use log::info;

#[macro_use]
mod utilities;

#[entry]
fn main() -> ! {

    utilities::logger::init();
    //rtt_init_print!(BlockIfFull);
    info!("wake cm7");
    log::debug!("wake cm7 debug");
    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = unsafe { pac::Peripherals::steal() };

    // Constrain and Freeze power
    info!("Setup PWR...                  ");
    let pwr = dp.pwr.constrain();
    let pwrcfg = example_power!(pwr).freeze();

    // Constrain and Freeze clock
    info!("Setup RCC...                  ");
    let rcc = dp.rcc.constrain();
    let ccdr = rcc.sys_ck(100.MHz()).freeze(pwrcfg, &dp.syscfg);

    let gpiob = dp.gpiob.split(ccdr.peripheral.GPIOB);

    // Configure PB0 as output.
    let mut led = gpiob.pb0.into_push_pull_output();

    // Get the delay provider.
    info!("clocks: {:?}", ccdr.clocks);
    let mut delay = cp.SYST.delay(ccdr.clocks);

    info!("start blinking LD1                  ");
    loop {
        led.set_high();
        delay.delay_ms(500_u16);

        led.set_low();
        delay.delay_ms(500_u16);
    }
}

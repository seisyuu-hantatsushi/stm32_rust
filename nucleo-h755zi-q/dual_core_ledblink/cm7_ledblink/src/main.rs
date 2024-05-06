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
    let cp = unsafe { cortex_m::Peripherals::steal() };
    let dp = unsafe { pac::Peripherals::steal() };

    let rcc = dp.RCC.constrain();
    // Wait until CPU2 boot and enters in stop mode
    {
        info!("wait until cm4 enters in dstop mode");
        loop {
            let d2ckrdy = rcc.is_d2_domain_available();
            //info!("d2ckrdy = {d2ckrdy}");
            if !d2ckrdy { break; };
        };
    }

    // Constrain and Freeze power
    info!("Setup PWR...                  ");
    let pwr = dp.PWR.constrain();
    let pwrcfg = example_power!(pwr).freeze();

    // Constrain and Freeze clock
    info!("Setup RCC...                  ");
    let ccdr = rcc.sys_ck(200.MHz()).freeze(pwrcfg, &dp.SYSCFG);

    let hsem = dp.HSEM.hsem_without_reset(ccdr.peripheral.HSEM);
    let mut sem0 = hsem.sema(0);
    sem0.fast_take();
    sem0.release(0);

    info!("wait until cm4 exit from dstop mode");
    loop {
        if ccdr.rcc.is_d2_domain_available() { break; }
    }

    let gpiob = dp.GPIOB.split(ccdr.peripheral.GPIOB);

    // Configure PB0 as output.
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

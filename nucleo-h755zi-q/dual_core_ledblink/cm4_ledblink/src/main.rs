#![deny(warnings)]
#![no_main]
#![no_std]

use cortex_m_rt::entry;
use stm32h7xx_hal::{pac, prelude::*, rcc};
use log::info;

//use rtt_target::{rprintln, rtt_init_print, ChannelMode::BlockIfFull};

#[macro_use]
mod utilities;

#[entry]
fn main() -> ! {

    utilities::logger::init();
    //rtt_init_print!(BlockIfFull);
    info!("wake cm4");

    let _cp = cortex_m::Peripherals::take().unwrap();
    let dp = unsafe { pac::Peripherals::steal() };

    let prec = unsafe { rcc::rec::steal_peripheral_rec() };

    let gpioe = dp.gpioe.split(prec.GPIOE);

    // Configure PE1 as output.
    let mut led = gpioe.pe1.into_push_pull_output();

    // Get the delay provider.
    //let mut delay = cp.SYST.delay(ccdr.clocks);

    info!("start blinking LD2                  ");
    loop {
        led.set_high();
        //delay.delay_ms(500_u16);

        led.set_low();
        //delay.delay_ms(500_u16);
    }
}

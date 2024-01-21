#![deny(unsafe_code)]
#![no_main]
#![no_std]


use cortex_m_rt::entry;
use stm32f4xx_hal::{
    pac,
    prelude::*,
};

// Print panic message to probe console
use panic_probe as _;
use rtt_target::{rprintln, rtt_init_print, ChannelMode::BlockIfFull};


#[allow(clippy::empty_loop)]
#[entry]
fn main() -> ! {
    if let (Some(dp), Some(cp)) = (
        pac::Peripherals::take(),
        cortex_m::peripheral::Peripherals::take(),
    ) {
	rtt_init_print!(BlockIfFull);
	rprintln!("hello from RTT");
        // Set up the LED. On the Nucleo-446RE it's connected to pin PA5.
        let gpiod = dp.GPIOD.split();
        let mut led = gpiod.pd12.into_push_pull_output();

        // Set up the system clock. We want to run at 48MHz for this one.
        let rcc = dp.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(48.MHz()).freeze();

        // Create a delay abstraction based on SysTick
        let mut delay = cp.SYST.delay(&clocks);

        loop {
            // On for 1s, off for 1s.
            led.toggle();
            delay.delay_ms(1000u32);
        }
    }
    loop {}
}

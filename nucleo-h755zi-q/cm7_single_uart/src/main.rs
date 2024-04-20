// Enable UART3 on pins PD8 and PD8.
// UART Echo program.
// able to run Virtual COM Port of STM32H7 Nucleo-144.

//#![deny(warnings)]
#![no_main]
#![no_std]

use core::{
    fmt::Write,
    cell::RefCell,
    sync::atomic::{AtomicU32, Ordering},
};

use cortex_m::peripheral::DWT;
use cortex_m_rt::entry;
use stm32h7xx_hal::{pac, pac::interrupt, prelude::*, time};
use nb::block;

#[macro_use]
mod utilities;

use log::info;


#[entry]
fn main() -> ! {

    utilities::logger::init();

    info!("wake cm7");

    let mut cp = unsafe { cortex_m::Peripherals::steal() };
    let dp = unsafe { pac::Peripherals::steal() };

    // Constrain and Freeze power
    info!("Setup PWR...                  ");
    let pwr = dp.PWR.constrain();
    let pwrcfg = example_power!(pwr).freeze();

    // Constrain and Freeze clock
    info!("Setup RCC...                  ");
    let rcc = dp.RCC.constrain();
    let ccdr = rcc.sys_ck(240.MHz()).freeze(pwrcfg, &dp.SYSCFG);

    let gpiob = dp.GPIOB.split(ccdr.peripheral.GPIOB);
    let gpiod = dp.GPIOD.split(ccdr.peripheral.GPIOD);

    let uart_tx_pd8 = gpiod.pd8.into_alternate();
    let uart_rx_pd9 = gpiod.pd9.into_alternate();

    let serial = match dp.USART3.serial((uart_tx_pd8, uart_rx_pd9), 115200.bps(), ccdr.peripheral.USART3, &ccdr.clocks) {
        Ok(s) => s,
        Err(err) => {
            panic!("failed to setup uart3. {:?}",err);
        }
    };

    let (mut usart_tx, mut usart_rx) = serial.split();


    // Configure PB0 as output.
    let mut led = gpiob.pb0.into_push_pull_output();

    match writeln!(usart_tx,"Hello world!\r"){
        Ok(_) => (),
        Err(err) => {
            panic!("failed to write uart3. {:?}",err);
        }
    }

    info!("start blinking LD1                  ");

    let mut blink:bool = true;
    cp.DWT.enable_cycle_counter();
    let mut prev_timestamp = DWT::cycle_count();
    let sysclk_freq = ccdr.clocks.sysclk().to_kHz();
    loop {
        let current_timestamp = DWT::cycle_count();
        match usart_rx.read() {
            Ok(v) => {
                block!(usart_tx.write(v)).ok();
                if v == '\r' as u8 {
                    block!(usart_tx.write('\n' as u8)).ok();
                }
            },
            Err(_) => ()
        }
        if current_timestamp.wrapping_sub(prev_timestamp) >= 1000*sysclk_freq {
            if blink {
                led.set_high();
            }
            else {
                led.set_low();
            }
            blink = !blink;
            prev_timestamp = current_timestamp;
        }
    }
}

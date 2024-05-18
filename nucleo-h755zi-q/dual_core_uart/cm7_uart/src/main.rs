//#![deny(warnings)]
#![no_main]
#![no_std]

use core::{
    fmt::Write,
    cell::RefCell,
    sync::atomic::{AtomicU32, Ordering},
};

use cortex_m_rt::entry;
use cortex_m::interrupt as cm_interrupt;
use cortex_m::peripheral::NVIC;
use stm32h7xx_hal::{pac, interrupt, timer, prelude::*};
use stm32h7xx_hal::time::MilliSeconds;
use stm32h7xx_hal::block;
use log::{info,debug};
use embedded_lib::console;

#[macro_use]
mod utilities;

static SEC_COUNTER: AtomicU32 = AtomicU32::new(0);
static TIMER: cm_interrupt::Mutex<RefCell<Option<timer::Timer<pac::TIM3>>>> =
    cm_interrupt::Mutex::new(RefCell::new(None));
static LED_BLINK: cm_interrupt::Mutex<RefCell<bool>> =
    cm_interrupt::Mutex::new(RefCell::new(false));

#[link_section = ".axisram"]
static mut CONSOLE_BUFFER: [u8; 1024] = [0u8; 1024];

#[allow(dead_code)]
fn type_of<T>(_: &T) -> &'static str {
    core::any::type_name::<T>()
}

#[entry]
fn main() -> ! {

    utilities::logger::init();
    //rtt_init_print!(BlockIfFull);
    info!("wake cm7");
    let mut cp = unsafe { cortex_m::Peripherals::steal() };
    let dp = unsafe { pac::Peripherals::steal() };

    // Constrain and Freeze clock
    let rcc = dp.RCC.constrain();

    // Wait until CPU2 boot and enters in stop mode
    {
        info!("wait until CPU2 enters in stop mode");
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

    info!("Setup RCC...                  ");
    let ccdr = rcc.sys_ck(200.MHz()).freeze(pwrcfg, &dp.SYSCFG);

    let hsem = dp.HSEM.hsem_without_reset(ccdr.peripheral.HSEM);
    let mut sem0 = hsem.sema(0);

    info!("wake up cm4.");
    sem0.fast_take();
    sem0.release(0);
    loop {
        if ccdr.rcc.is_d2_domain_available() { break; }
    }

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

    let _ = writeln!(usart_tx, "hello, I'm cm7.\r");

    let mut timer = dp.TIM3.timer(1.Hz(), ccdr.peripheral.TIM3, &ccdr.clocks);
    timer.listen(timer::Event::TimeOut); //Enable Interrupt
    timer.start(MilliSeconds::from_ticks(1000).into_rate());

    cm_interrupt::free(|cs| {
        TIMER.borrow(cs).replace(Some(timer));
    });

    unsafe {
        cp.NVIC.set_priority(interrupt::TIM3, 1);
        NVIC::unmask::<stm32h7xx_hal::interrupt>(interrupt::TIM3);
    }

    // Configure PB0 as output.
    let mut led = gpiob.pb0.into_push_pull_output();

    let _ = writeln!(usart_tx,"start blinking LD1                  \r");

    let mut console =
        unsafe { console::Console::new(
            &mut CONSOLE_BUFFER,
            "cm7> ",
            move || {
                match usart_rx.read() {
                    Ok(c) => {
                        Some(c)
                    },
                    Err(_) => None
                }
            },
            move |c| {
                block!(usart_tx.write(c)).ok();
            }) };

    let mut prev_blink = false;
    loop {
        let mut update = false;

        cm_interrupt::free(|cs| {
            let current = *LED_BLINK.borrow(cs).borrow();
            update = prev_blink != current;
            if update {
                if current {
                    led.set_high();
                } else {
                    led.set_low();
                };
            }
            prev_blink = current;
        });
        console.input();
    }
}

#[stm32h7xx_hal::interrupt]
fn TIM3() {
    SEC_COUNTER.fetch_add(1, Ordering::SeqCst);
    cortex_m::interrupt::free(|cs| {
        let mut rc = TIMER.borrow(cs).borrow_mut();
        let timer = rc.as_mut().unwrap();
        timer.clear_irq();
        let mut led_blink = LED_BLINK.borrow(cs).borrow_mut();
        *led_blink = !(*led_blink);
    });
}


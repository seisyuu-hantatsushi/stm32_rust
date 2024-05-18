//#![deny(warnings)]
#![no_main]
#![no_std]

/*
   PD5: USART2 TX
   PD6: USART2 RX
*/

use core::{
    fmt::Write,
    cell::RefCell,
    sync::atomic::{AtomicU32, Ordering},
};

use cortex_m_rt::entry;
use cortex_m::interrupt as cm_interrupt;
use cortex_m::peripheral::NVIC;
use stm32h7xx_hal::{pac, interrupt, rcc, pwr, timer, hsem, exti, block, prelude::* };
use stm32h7xx_hal::time::MilliSeconds;
use log::info;
use embedded_lib::console;

#[link_section = ".sram2"]
static mut CONSOLE_BUFFER: [u8; 1024] = [0u8; 1024];

#[macro_use]
mod utilities;

static SEC_COUNTER: AtomicU32 = AtomicU32::new(0);
static HSEM_CH0: cm_interrupt::Mutex<RefCell<Option<hsem::Sema>>> =
    cm_interrupt::Mutex::new(RefCell::new(None));
static TIMER: cm_interrupt::Mutex<RefCell<Option<timer::Timer<pac::TIM3>>>> =
    cm_interrupt::Mutex::new(RefCell::new(None));
static LED_BLINK: cm_interrupt::Mutex<RefCell<bool>> =
    cm_interrupt::Mutex::new(RefCell::new(false));

#[entry]
fn main() -> ! {
    utilities::logger::init();
    //rtt_init_print!(BlockIfFull)
    info!("wake cm4");

    let mut cp = unsafe { cortex_m::Peripherals::steal() };
    let dp = unsafe { pac::Peripherals::steal() };

    let prec = unsafe { rcc::rec::steal_peripheral_rec() };
    let rcc = dp.RCC.constrain();
    let pwr = dp.PWR.constrain();

    // Activate HSEM notification.

    let hsem = dp.HSEM.hsem(prec.HSEM);

    dp.EXTI.listen(exti::Event::HSEM1);

    let sem0 = hsem.sema(0);
    cm_interrupt::free(|cs| {
        HSEM_CH0.borrow(cs).replace(Some(sem0));
        HSEM_CH0.borrow(cs).borrow_mut().as_mut().unwrap().enable_irq();
    });

    unsafe {
        cp.NVIC.set_priority(interrupt::HSEM1, 3);
        NVIC::unmask::<stm32h7xx_hal::interrupt>(interrupt::HSEM1);
    }

    // Domain D2 enter stop mode.
    // Waiting for CM7 perforing system initialization.
    pwr.clear_pending_event();

    pwr.d2_domain_enters_stopmode(&mut cp,
                                  pwr::ReguratorStateInStopMode::MainReguratorOn,
                                  pwr::StopModeEnterWith::WaitForEvent);

    let clocks = rcc.get_frozen_core_clocks().expect("could not get clocks");

    // GPIOD was reseted by CM7
    let gpiod = dp.GPIOD.split_without_reset(prec.GPIOD);
    let gpioe = dp.GPIOE.split(prec.GPIOE);

    let uart_tx_pd5 = gpiod.pd5.into_alternate();
    let uart_rx_pd6 = gpiod.pd6.into_alternate();

    let serial = match dp.USART2.serial((uart_tx_pd5, uart_rx_pd6), 115200.bps(), prec.USART2, &clocks) {
        Ok(s) => s,
        Err(err) => {
            panic!("failed to setup usart2. {:?}",err);
        }
    };

    let (mut usart_tx, mut usart_rx) = serial.split();
    let _ = writeln!(usart_tx, "hello, I'm cm4.\r");

    let mut timer = dp.TIM3.timer(1.Hz(), prec.TIM3, &clocks);
    timer.listen(timer::Event::TimeOut); //Enable Interrupt
    timer.start(MilliSeconds::from_ticks(1000).into_rate());

    cm_interrupt::free(|cs| {
        TIMER.borrow(cs).replace(Some(timer));
    });

    unsafe {
        cp.NVIC.set_priority(interrupt::TIM3, 1);
        NVIC::unmask::<stm32h7xx_hal::interrupt>(interrupt::TIM3);
    }

    let mut console =
        unsafe { console::Console::new(
            &mut CONSOLE_BUFFER,
            "cm4> ",
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
            })
        };

    // Configure PE1 as output.
    let mut led = gpioe.pe1.into_push_pull_output();

    info!("start blinking LD2                  ");

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
fn HSEM1() {
    info!("HSEM1 fire!");
    cm_interrupt::free(|cs| {
        HSEM_CH0.borrow(cs).borrow_mut().as_mut().unwrap().clear_irq();
    });
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


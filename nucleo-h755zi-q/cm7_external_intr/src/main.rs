//
// Example of External Interrupt.
// Able to run on nucleo H755ZI-Q
// B1 user(Blue button) is connected to PC13 on nucleo-H755ZI-Q.
// toggle LED1 when B1 button is pushed.

//#![deny(warnings)]
#![no_main]
#![no_std]

use core::cell::RefCell;

use cortex_m::interrupt as cm_interrupt;
use cortex_m::peripheral::NVIC;
use cortex_m_rt::entry;
use stm32h7xx_hal::{pac, interrupt, prelude::*, gpio, gpio::ExtiPin};
use stm32h7xx_hal::gpio::gpioc::PC13;

use nb::block;

#[macro_use]
mod utilities;

use log::info;

// Setup the sharing of pins between the main loop and the interrupt
static BLUE_BUTTON_PIN: cm_interrupt::Mutex<RefCell<Option<PC13<gpio::Input>>>> =
    cm_interrupt::Mutex::new(RefCell::new(None));

static LED_TOGGLE: cm_interrupt::Mutex<RefCell<bool>> =
    cm_interrupt::Mutex::new(RefCell::new(false));

/*
For dumping memory mapped register
use core::ptr::read_volatile;
const SYSCFG_BASE : *mut u32 = 0x5800_0400 as *mut u32;
const SYSCFG_EXTICR1 : *mut u32 = (SYSCFG_BASE.wrapping_add(2)) as *mut u32;
const SYSCFG_EXTICR2 : *mut u32 = (SYSCFG_BASE.wrapping_add(3)) as *mut u32;
const SYSCFG_EXTICR3 : *mut u32 = (SYSCFG_BASE.wrapping_add(4)) as *mut u32;
const SYSCFG_EXTICR4 : *mut u32 = (SYSCFG_BASE.wrapping_add(5)) as *mut u32;
const EXTI_BASE : *mut u32 = 0x5800_0000 as *mut u32;
const EXTI_C1PR1: *mut u32 = (EXTI_BASE.wrapping_add(34)) as *mut u32;
const GPIOC_BASE: *mut u32 = 0x5802_0800 as *mut u32;
const GPIOC_IDR:  *mut u32 = (GPIOC_BASE.wrapping_add(4)) as *mut u32;
 */

#[entry]
fn main() -> ! {

    utilities::logger::init();

    info!("wake cm7");

    let mut cp = cortex_m::Peripherals::take().unwrap();
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
    let gpioc = dp.GPIOC.split(ccdr.peripheral.GPIOC);

    let mut syscfg = dp.SYSCFG;
    let mut exti = dp.EXTI;

    let mut blue_button = gpioc.pc13.into_pull_down_input();
    blue_button.make_interrupt_source(&mut syscfg);
    blue_button.trigger_on_edge(&mut exti, gpio::Edge::Rising);
    blue_button.enable_interrupt(&mut exti);

    let exti_no = blue_button.interrupt();

    cm_interrupt::free(|cs| {
        BLUE_BUTTON_PIN.borrow(cs).replace(Some(blue_button));
    });

    unsafe {
        cp.NVIC.set_priority(exti_no, 1);
        NVIC::unmask::<stm32h7xx_hal::interrupt>(exti_no);
    }
    // Configure PB0 as output.
    let mut led = gpiob.pb0.into_push_pull_output();
    loop {
        cortex_m::asm::wfi();
        info!("wake by interrupt");
        cm_interrupt::free(|cs| {
            if *LED_TOGGLE.borrow(cs).borrow() {
                led.set_high();
            } else {
                led.set_low();
            }
        });
    }
}

#[stm32h7xx_hal::interrupt]
fn EXTI15_10() {
    info!("EXTI15_10 fired!");
    cm_interrupt::free(|cs| {
        if let Some(b) = BLUE_BUTTON_PIN.borrow(cs).borrow_mut().as_mut() {
            b.clear_interrupt_pending_bit();
        }
        let mut b = LED_TOGGLE.borrow(cs).borrow_mut();
        *b = !(*b);
    });
}

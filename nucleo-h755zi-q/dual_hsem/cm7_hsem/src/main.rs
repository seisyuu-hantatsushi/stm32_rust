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
use stm32h7xx_hal::gpio::{self, gpioc::PC13, ExtiPin};
use stm32h7xx_hal::time::MilliSeconds;
use stm32h7xx_hal::block;
use log::info;

#[macro_use]
mod utilities;

static SEC_COUNTER: AtomicU32 = AtomicU32::new(0);
static TIMER: cm_interrupt::Mutex<RefCell<Option<timer::Timer<pac::TIM3>>>> =
    cm_interrupt::Mutex::new(RefCell::new(None));
static BLUE_BUTTON_PIN: cm_interrupt::Mutex<RefCell<Option<PC13<gpio::Input>>>>
    = cm_interrupt::Mutex::new(RefCell::new(None));
static LED_BLINK: cm_interrupt::Mutex<RefCell<bool>> =
    cm_interrupt::Mutex::new(RefCell::new(false));
static KICK_HSEM: cm_interrupt::Mutex<RefCell<bool>> =
    cm_interrupt::Mutex::new(RefCell::new(false));

use core::ptr::read_volatile;
const TIM3_BASE   : *mut u32 = 0x40000400 as *mut u32;
const TIM3_PSC    : *mut u16 = 0x40000428 as *mut u16;
const TIM3_ARR    : *mut u32 = 0x4000042C as *mut u32;
const RCC_BASE    : *mut u32 = 0x58024400 as *mut u32;
const HSEM_BASE   : *mut u32 = 0x58026400 as *mut u32;
const HSEM_C1IER  : *mut u32 = 0x58026500 as *mut u32;
const HSEM_C2IER  : *mut u32 = 0x58026510 as *mut u32;
const HSEM_C2ISR  : *mut u32 = 0x58026518 as *mut u32;
const HSEM_C2MISR : *mut u32 = 0x5802651C as *mut u32;

//#[maybe_unused]
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
    //#[cfg(not(all()))]
    {
        info!("wait until CPU2 enters in stop mode");
        loop {
            let d2ckrdy = rcc.is_d2_domain_available();
            //info!("d2ckrdy = {d2ckrdy}");
            if !d2ckrdy { break; };
        };
    }
    info!("D2 Domain Clock {}\r", rcc.is_d2_domain_available());
    // Constrain and Freeze power
    info!("Setup PWR...                  ");
    let pwr = dp.PWR.constrain();
    let pwrcfg = example_power!(pwr).freeze();
    info!("1 D2 Domain Clock {}\r", rcc.is_d2_domain_available());
    info!("Setup RCC...                  ");
    let ccdr = rcc.sys_ck(200.MHz()).freeze(pwrcfg, &dp.SYSCFG);
    info!("2 D2 Domain Clock {}\r", ccdr.rcc.is_d2_domain_available());

    let hsem = dp.HSEM.hsem_without_reset(ccdr.peripheral.HSEM);
    let mut sem0 = hsem.sema(0);
    info!("3 D2 Domain Clock {}\r", ccdr.rcc.is_d2_domain_available());

    //#[cfg(not(all()))]
    {
        sem0.fast_take();
        sem0.release(0);
        // let _ = writeln!(usart_tx, "waiting util CM4 wakes up\r");
        loop {
            if ccdr.rcc.is_d2_domain_available() { break; }
        }
    }

    info!("4 D2 Domain Clock {}\r", ccdr.rcc.is_d2_domain_available());
    let gpiob = dp.GPIOB.split(ccdr.peripheral.GPIOB);
    let gpioc = dp.GPIOC.split(ccdr.peripheral.GPIOC);
    let gpiod = dp.GPIOD.split(ccdr.peripheral.GPIOD);

    info!("5 D2 Domain Clock {}\r", ccdr.rcc.is_d2_domain_available());
    /*
    let uart_tx_pd8 = gpiod.pd8.into_alternate();
    let uart_rx_pd9 = gpiod.pd9.into_alternate();

    let serial = match dp.USART3.serial((uart_tx_pd8, uart_rx_pd9), 115200.bps(), ccdr.peripheral.USART3, &ccdr.clocks) {
        Ok(s) => s,
        Err(err) => {
            panic!("failed to setup uart3. {:?}",err);
        }
    };

    let (mut usart_tx, mut usart_rx) = serial.split();
*/
    //info!("CLK_INT {:?}", ccdr.clocks.timx_ker_ck());
    //let _ = writeln!(usart_tx, "hello, I'm cm7.\r");
    info!("hello, I'm cm7.");
    //let _ = writeln!(usart_tx, "AIRCR {:08X}\r", cp.SCB.aircr().read().bits());
    let mut syscfg = dp.SYSCFG;
    let mut exti = dp.EXTI;

    let mut blue_button = gpioc.pc13.into_pull_down_input();
    blue_button.make_interrupt_source(&mut syscfg);
    blue_button.trigger_on_edge(&mut exti, gpio::Edge::Rising);
    blue_button.enable_interrupt(&mut exti);

    let exti_no = blue_button.interrupt();
/*
    let mut timer = dp.TIM3.timer(1.Hz(), ccdr.peripheral.TIM3, &ccdr.clocks);
    timer.listen(timer::Event::TimeOut); //Enable Interrupt
    timer.start(MilliSeconds::from_ticks(1000).into_rate());
    #[cfg(not(all()))]
    unsafe {
        let _ = writeln!(usart_tx, "TIM3_BASE: 0x{:08X}", read_volatile(TIM3_BASE));
        let _ = writeln!(usart_tx, "TIM3_PSC:  0x{:04X}", read_volatile(TIM3_PSC));
        let _ = writeln!(usart_tx, "TIM3_ARR:  0x{:08x}", read_volatile(TIM3_ARR));
    }

    cm_interrupt::free(|cs| {
        BLUE_BUTTON_PIN.borrow(cs).replace(Some(blue_button));
    });

    {
        cm_interrupt::free(|cs| {
            TIMER.borrow(cs).replace(Some(timer));
        });

        unsafe {
            cp.NVIC.set_priority(exti_no, 2);
            NVIC::unmask::<stm32h7xx_hal::interrupt>(exti_no);
            cp.NVIC.set_priority(interrupt::TIM3, 1);
            NVIC::unmask::<stm32h7xx_hal::interrupt>(interrupt::TIM3);
        }
    }
*/
    // Configure PB0 as output.
    let mut led = gpiob.pb0.into_push_pull_output();

    info!("start loop");
    // Get the delay provider.
    let mut delay = cp.SYST.delay(ccdr.clocks);

    info!("7 D2 Domain Clock {}\r", ccdr.rcc.is_d2_domain_available());

    /*
    {
        sem0.fast_take();
        sem0.release(0);
        info!("wait until cm4 wakes up");
        loop {
            let wake = ccdr.rcc.is_d2_domain_available();
            info!("D2 Domain Clock {}", wake);
            if wake { break; }
            delay.delay_ms(1000_u16);
        }
    }
     */
    cortex_m::asm::sev();
    loop {};
    //let _ = writeln!(usart_tx,"start blinking LD1                  \r");
/*
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
                /*
                let _ = writeln!(usart_tx,"current stat\r");
                unsafe {
    let _ = writeln!(usart_tx,"HSEM_C2IER {:08X}\r", read_volatile(HSEM_C2IER));
                    let _ = writeln!(usart_tx,"HSEM_C2ISR {:08X}\r", read_volatile(HSEM_C2ISR));
                    let _ = writeln!(usart_tx,"HSEM_C2MISR {:08X}\r", read_volatile(HSEM_C2MISR));
                    let _ = writeln!(usart_tx,"D2 Domain Clock {}\r", ccdr.rcc.is_d2_domain_available());
                }
                 */
            }
            prev_blink = current;
            let mut kick_hsem = KICK_HSEM.borrow(cs).borrow_mut();
            if *kick_hsem {
                let second = SEC_COUNTER.load(Ordering::SeqCst);
                //let _ = writeln!(usart_tx,"kick hsem {second} sec\r");
                info!("kick hsem {second} sec\r");
                sem0.fast_take();
                sem0.release(0);
                *kick_hsem = false;
            }
        });
    }
*/
}

#[stm32h7xx_hal::interrupt]
fn EXTI15_10() {
    //info!("EXTI15_10 fired!");
    cm_interrupt::free(|cs| {
        if let Some(b) = BLUE_BUTTON_PIN.borrow(cs).borrow_mut().as_mut() {
            b.clear_interrupt_pending_bit();
        }
        let mut kick_hsem = KICK_HSEM.borrow(cs).borrow_mut();
        *kick_hsem = true;
    });
}

#[stm32h7xx_hal::interrupt]
fn TIM3() {
    //info!("TIM3 fired!");
    SEC_COUNTER.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
    cortex_m::interrupt::free(|cs| {
        let mut rc = TIMER.borrow(cs).borrow_mut();
        let timer = rc.as_mut().unwrap();
        timer.clear_irq();
        let mut led_blink = LED_BLINK.borrow(cs).borrow_mut();
        *led_blink = !(*led_blink);
    });
}


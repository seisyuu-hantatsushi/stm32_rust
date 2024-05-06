//#![deny(warnings)]
#![no_main]
#![no_std]

use core::cell::RefCell;

use cortex_m_rt::entry;
use cortex_m::interrupt as cm_interrupt;
use cortex_m::peripheral::NVIC;
use stm32h7xx_hal::{pac, interrupt, rcc, pwr, hsem, exti, prelude::* };
use log::info;

use core::ptr::read_volatile;
const HSEM_C2IER  : *mut u32 = 0x58026510 as *mut u32;
const HSEM_C2ISR  : *mut u32 = 0x58026518 as *mut u32;
const HSEM_C2MISR : *mut u32 = 0x5802651C as *mut u32;

#[macro_use]
mod utilities;

static HSEM_CH0: cm_interrupt::Mutex<RefCell<Option<hsem::Sema>>> =
    cm_interrupt::Mutex::new(RefCell::new(None));

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

    dp.EXTI.listen(exti::Event::HSEM1);

    let hsem = dp.HSEM.hsem(prec.HSEM);
    info!("HSEM clock enable: {}", rcc.is_hsem_enable());

    // Activate HSEM notification.
    let sem0 = hsem.sema(0);

    cm_interrupt::free(|cs| {
        HSEM_CH0.borrow(cs).replace(Some(sem0));
        HSEM_CH0.borrow(cs).borrow_mut().as_mut().unwrap().enable_irq();
    });

    unsafe {
        info!("HSEM_C2IER {:08X}", read_volatile(HSEM_C2IER));
        cp.NVIC.set_priority(interrupt::HSEM1, 3);
        NVIC::unmask::<stm32h7xx_hal::interrupt>(interrupt::HSEM1);
        cp.NVIC.set_priority(interrupt::CM7_SEV_IT, 2);
        NVIC::unmask::<stm32h7xx_hal::interrupt>(interrupt::CM7_SEV_IT);
    }

    //#[cfg(not(all()))]
    {
        // Domain D2 enter stop mode.
        // Waiting for CM7 perforing system initialization.
        pwr.clear_pending_event();

        pwr.d2_domain_enters_stopmode(&mut cp,
                                      pwr::ReguratorStateInStopMode::MainReguratorOn,
                                      pwr::StopModeEnterWith::WaitForEvent);
    }

    let gpioe = dp.GPIOE.split(prec.GPIOE);

    // Configure PE1 as output.
    let mut led = gpioe.pe1.into_push_pull_output();

    let clocks = rcc.get_frozen_core_clocks().expect("could not get clocks");
    // Get the delay provider.
    let mut delay = cp.SYST.delay(clocks);

    info!("start blinking LD2                  ");
    loop {
        led.set_high();
        delay.delay_ms(500_u16);

        led.set_low();
        delay.delay_ms(500_u16);
        unsafe {
            info!("cm4 HSEM_C2IER {:08X}", read_volatile(HSEM_C2IER));
            info!("cm4 HSEM_C2ISR {:08X}", read_volatile(HSEM_C2ISR));
        }
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
fn CM7_SEV_IT() {
    info!("sev it");
    cortex_m::asm::wfe(); //clear event
}

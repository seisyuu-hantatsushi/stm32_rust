#![deny(warnings)]
#![no_main]
#![no_std]

use core::cell::RefCell;

use cortex_m_rt::entry;
use cortex_m::interrupt as cm_interrupt;
use cortex_m::peripheral::NVIC;
use stm32h7xx_hal::{pac, interrupt, rcc, pwr, hsem, exti, prelude::* };
use log::info;

//use rtt_target::{rprintln, rtt_init_print, ChannelMode::BlockIfFull};

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

    let gpioe = dp.GPIOE.split(prec.GPIOE);

    // Configure PE1 as output.
    let mut led = gpioe.pe1.into_push_pull_output();

    // Get the delay provider.
    let mut delay = cp.SYST.delay(clocks);

    info!("start blinking LD2                  ");
    loop {
        led.set_high();
        delay.delay_ms(500_u16);

        led.set_low();
        delay.delay_ms(500_u16);
    }
}

#[stm32h7xx_hal::interrupt]
fn HSEM1() {
    info!("HSEM1 fire!");
    cm_interrupt::free(|cs| {
        HSEM_CH0.borrow(cs).borrow_mut().as_mut().unwrap().clear_irq();
    });
}

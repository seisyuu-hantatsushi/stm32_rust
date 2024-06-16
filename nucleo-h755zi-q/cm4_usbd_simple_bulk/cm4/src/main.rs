//#![deny(warnings)]
#![no_main]
#![no_std]

/*
   This code is tested with as follows.
   - NUCLEO-H755ZI-Q

*/

/*
   functions are assinged to pin as follows,

   PA8:  USB SOF <- Not used
   PA9:  USB VBUS
   PA10: USB ID
   PA11: USB DM
   PA12: USB DP
   PD10: USB FW PWR EN

   PD5: USART2 TX
   PD6: USART2 RX

*/

use core::{
    mem::MaybeUninit,
    fmt::Write,
    cell::RefCell,
    sync::atomic::{AtomicU32, AtomicBool, Ordering},
};

use cortex_m_rt::entry;
use cortex_m::interrupt as cm_interrupt;
use cortex_m::peripheral::NVIC;
use stm32h7xx_hal::{pac, interrupt, rcc, pwr, timer, hsem, exti, block, prelude::* };
use stm32h7xx_hal::usb_hs::{UsbBus, USB2};
use stm32h7xx_hal::time::MilliSeconds;
use log::{info,debug};
use embedded_lib::console;

use usb_device::prelude::*;

#[link_section = ".sram2"]
static mut CONSOLE_BUFFER: [u8; 1024] = [0u8; 1024];
#[link_section = ".sram2"]
static mut EP_MEMORY: MaybeUninit<[u32; 1024]> = MaybeUninit::uninit();

#[macro_use]
mod utilities;

static SEC_COUNTER: AtomicU32 = AtomicU32::new(0);
static HSEM_CH0: cm_interrupt::Mutex<RefCell<Option<hsem::Sema<0>>>> =
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

    let mut prec = unsafe { rcc::rec::steal_peripheral_rec() };
    let rcc = dp.RCC.constrain();
    let pwr = dp.PWR.constrain();

    // Hsi48 clock is inputed into USB clock.
    prec.kernel_usb_clk_mux(rcc::rec::UsbClkSel::Hsi48);

    // Activate HSEM notification.
    let mut hsem = dp.HSEM.hsem(prec.HSEM);

    dp.EXTI.listen(exti::Event::HSEM1);

    let sem0 = hsem.sema0();
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

    let gpioa = dp.GPIOA.split(prec.GPIOA);
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

    let (pin_dm, pin_dp) = (gpioa.pa11.into_alternate(), gpioa.pa12.into_alternate());

    let usb = USB2::new(
        dp.OTG2_HS_GLOBAL,
        dp.OTG2_HS_DEVICE,
        dp.OTG2_HS_PWRCLK,
        pin_dm,
        pin_dp,
        prec.USB2OTG,
        &clocks
    );

    unsafe {
        let buf: &mut [MaybeUninit<u32>; 1024] =
            &mut *(core::ptr::addr_of_mut!(EP_MEMORY) as *mut _);
        buf.iter_mut().for_each(|v| v.as_mut_ptr().write(0));
    }

    let usb_bus = UsbBus::new(usb, unsafe { EP_MEMORY.assume_init_mut() });

    let mut serial = usbd_serial::SerialPort::new(&usb_bus);
    let mut usb_dev =
        UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
            .strings(&[usb_device::device::StringDescriptors::default()
                .manufacturer("Fake company")
                .product("Serial port")
                .serial_number("TEST PORT 1")])
            .unwrap()
            .device_class(usbd_serial::USB_CLASS_CDC)
            .build();

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
        unsafe {
            console::Console::new(
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
                },
                None::<fn (&str)>)
        };

    // Configure PE1 as output.
    let mut led = gpioe.pe1.into_push_pull_output();

    let _ = writeln!(console,"start blinking LD2                  \r");

    let mut prev_blink = false;
    loop {

        if usb_dev.poll(&mut [&mut serial]) {
            let mut buf = [0u8; 64];
            match serial.read(&mut buf) {
                Ok(count) if count > 0 => {
                    // Echo back in upper case
                    for c in buf[0..count].iter_mut() {
                        if 0x61 <= *c && *c <= 0x7a {
                            *c &= !0x20;
                        }
                    }

                    let mut write_offset = 0;
                    while write_offset < count {
                        match serial.write(&buf[write_offset..count]) {
                            Ok(len) if len > 0 => {
                                write_offset += len;
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

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
    cm_interrupt::free(|cs| {
        let mut binding = HSEM_CH0.borrow(cs).borrow_mut();
        let sem0 = binding.as_mut().unwrap();
        if sem0.status_irq() {
            sem0.clear_irq()
        };
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


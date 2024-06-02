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
    sync::atomic::{AtomicU32, AtomicBool, Ordering},
};

use cortex_m_rt::entry;
use cortex_m::interrupt as cm_interrupt;
use cortex_m::peripheral::NVIC;
use stm32h7xx_hal::{pac, interrupt, rcc, pwr, timer, hsem, exti, block, prelude::* };
use stm32h7xx_hal::time::MilliSeconds;
use log::{info,debug};
use embedded_lib::{console,shared_ringbuffer};

#[link_section = ".sram2"]
static mut CONSOLE_BUFFER: [u8; 1024] = [0u8; 1024];

const CM7_TO_CM4_SHARED_RINGBUFFER: *mut u32 = 0x10040000 as *mut u32; // in D2 Domain, Write-Through
const CM7_TO_CM4_SHARED_RINGBUFFER_SIZE: u32 = 512+1024*8; //
const CM4_TO_CM7_SHARED_RINGBUFFER: *mut u32 = 0x10042400 as *mut u32; // in D2 Domain, Write-Through
const CM4_TO_CM7_SHARED_RINGBUFFER_SIZE: u32 = 512+1024*8; //

#[macro_use]
mod utilities;

static SEC_COUNTER: AtomicU32 = AtomicU32::new(0);
static MESSAGE_NOTIFY: AtomicBool = AtomicBool::new(false);
static HSEM_CH0: cm_interrupt::Mutex<RefCell<Option<hsem::Sema<0>>>> =
    cm_interrupt::Mutex::new(RefCell::new(None));
static HSEM_CH2: cm_interrupt::Mutex<RefCell<Option<hsem::Sema<2>>>> =
    cm_interrupt::Mutex::new(RefCell::new(None));
static TIMER: cm_interrupt::Mutex<RefCell<Option<timer::Timer<pac::TIM3>>>> =
    cm_interrupt::Mutex::new(RefCell::new(None));
static LED_BLINK: cm_interrupt::Mutex<RefCell<bool>> =
    cm_interrupt::Mutex::new(RefCell::new(false));

struct HardwareCriticalSection {
    procid : u8,
    sem: RefCell<hsem::Sema<3>>
}

struct HardwareCriticalSectionLock<'a> {
    procid : u8,
    sem: &'a RefCell<hsem::Sema<3>>
}

impl<'a> HardwareCriticalSectionLock<'a> {
    fn new(procid : u8, sem: &'a RefCell<hsem::Sema<3>>) -> Self {
        HardwareCriticalSectionLock {
            procid,
            sem
        }
    }
}

impl Drop for HardwareCriticalSectionLock<'_> {
    fn drop(&mut self) {
        self.sem.borrow_mut().release(self.procid);
    }
}

impl shared_ringbuffer::CriticalSection for HardwareCriticalSection {
    fn lock(&self) -> Result<HardwareCriticalSectionLock,shared_ringbuffer::SharedRingBufferError> {
        while !self.sem.borrow_mut().take(self.procid) {};
        Ok(HardwareCriticalSectionLock::new( self.procid, &self.sem ))
    }
}

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

    let mut sem1 = hsem.sema1();
    let mut cm7_to_cm4_shared_ringbuffer = unsafe {
        let ptr : *mut u8 = CM7_TO_CM4_SHARED_RINGBUFFER as *mut u8;
        ptr.write_bytes(0, CM7_TO_CM4_SHARED_RINGBUFFER_SIZE as usize);
        shared_ringbuffer::SharedRingBuffer::<1024,8>::assign(CM7_TO_CM4_SHARED_RINGBUFFER,
                                                              CM7_TO_CM4_SHARED_RINGBUFFER_SIZE)
    };

    let hwcs = HardwareCriticalSection {
        procid: 1,
        sem: RefCell::new(hsem.sema3())
    };

    let mut cm4_to_cm7_shared_ringbuffer = unsafe {
        let ptr : *mut u8 = CM4_TO_CM7_SHARED_RINGBUFFER as *mut u8;
        ptr.write_bytes(0, CM4_TO_CM7_SHARED_RINGBUFFER_SIZE as usize);
        shared_ringbuffer::SharedRingBufferWithCS::<1024,8, HardwareCriticalSection>
            ::assign(CM4_TO_CM7_SHARED_RINGBUFFER,
                     CM4_TO_CM7_SHARED_RINGBUFFER_SIZE,
                     hwcs)
    };

    sem1.fast_take();
    sem1.release(0);

    let sem2 = hsem.sema2();
    cm_interrupt::free(|cs| {
        HSEM_CH2.borrow(cs).replace(Some(sem2));
        HSEM_CH2.borrow(cs).borrow_mut().as_mut().unwrap().enable_irq();
    });

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
            },
            Some(move |command:&str| {
                //debug!("send {} <{}>", command.len(), command);
                let _ = cm4_to_cm7_shared_ringbuffer.write(command.as_bytes());
            }))
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
                debug!("update led");
                if current {
                    led.set_high();
                } else {
                    led.set_low();
                };
            }
            prev_blink = current;
        });

        console.input();

        if let Ok(notify) = MESSAGE_NOTIFY.fetch_update(Ordering::SeqCst,
                                                        Ordering::SeqCst,
                                                        |notify| if notify {
                                                            Some(false)
                                                        }
                                                        else {
                                                            None
                                                        }){
            if notify {
                let mut recvbuf = [0u8;1024];
                match cm7_to_cm4_shared_ringbuffer.read(&mut recvbuf) {
                    Ok(readsize) => {
                        let _ = write!(console,">cm7> {}\r\n", core::str::from_utf8(&recvbuf).unwrap());
                    },
                    Err(e) => { debug!("read error: {}", e); }
                };
            }
        }
    }
}

#[stm32h7xx_hal::interrupt]
fn HSEM1() {
    debug!("HSEM1 fired!");
    cm_interrupt::free(|cs| {
        let mut binding = HSEM_CH0.borrow(cs).borrow_mut();
        let sem0 = binding.as_mut().unwrap();
        if sem0.status_irq() {
            sem0.clear_irq()
        };
        let mut binding = HSEM_CH2.borrow(cs).borrow_mut();
        if let Some(sem2) = binding.as_mut(){
            if sem2.status_irq() {
                MESSAGE_NOTIFY.store(true,Ordering::SeqCst);
                sem2.clear_irq()
            };
        };
    });
}

#[stm32h7xx_hal::interrupt]
fn TIM3() {
    SEC_COUNTER.fetch_add(1, Ordering::SeqCst);
    debug!("TIM3 fired!");
    cortex_m::interrupt::free(|cs| {
        let mut rc = TIMER.borrow(cs).borrow_mut();
        let timer = rc.as_mut().unwrap();
        timer.clear_irq();
        let mut led_blink = LED_BLINK.borrow(cs).borrow_mut();
        *led_blink = !(*led_blink);
    });
}


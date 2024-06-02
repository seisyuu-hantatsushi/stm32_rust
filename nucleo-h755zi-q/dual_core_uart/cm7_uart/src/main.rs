//#![deny(warnings)]
#![no_main]
#![no_std]

use core::{
    fmt::Write,
    cell::RefCell,
    sync::atomic::{AtomicU32, AtomicBool, Ordering},
};

use cortex_m_rt::entry;
use cortex_m::interrupt as cm_interrupt;
use cortex_m::peripheral::NVIC;
use stm32h7xx_hal::{pac, interrupt, timer, block, hsem, prelude::*};
use stm32h7xx_hal::time::MilliSeconds;
use log::{info,debug};
use embedded_lib::{console,shared_ringbuffer};

#[macro_use]
mod utilities;

struct HardwareCriticalSection {
    procid : u8,
    sem: RefCell<hsem::SemaOp<3>>
}

struct HardwareCriticalSectionLock<'a> {
    procid : u8,
    sem: &'a RefCell<hsem::SemaOp<3>>
}

impl<'a> HardwareCriticalSectionLock<'a> {
    fn new(procid : u8, sem: &'a RefCell<hsem::SemaOp<3>>) -> Self {
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

static SEC_COUNTER: AtomicU32 = AtomicU32::new(0);
static MESSAGE_NOTIFY: AtomicBool = AtomicBool::new(false);
static TIMER: cm_interrupt::Mutex<RefCell<Option<timer::Timer<pac::TIM2>>>> =
    cm_interrupt::Mutex::new(RefCell::new(None));
static LED_BLINK: cm_interrupt::Mutex<RefCell<bool>> =
    cm_interrupt::Mutex::new(RefCell::new(false));

#[link_section = ".axisram"]
static mut CONSOLE_BUFFER: [u8; 1024] = [0u8; 1024];

const CM7_TO_CM4_SHARED_RINGBUFFER: *mut u32 = 0x10040000 as *mut u32; // in D2 Domain, Write-Through
const CM7_TO_CM4_SHARED_RINGBUFFER_SIZE: u32 = 512+1024*8;
const CM4_TO_CM7_SHARED_RINGBUFFER: *mut u32 = 0x10042400 as *mut u32; // in D2 Domain, Write-Through
const CM4_TO_CM7_SHARED_RINGBUFFER_SIZE: u32 = 512+1024*8; //

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

    let mut hsem = dp.HSEM.hsem_without_reset(ccdr.peripheral.HSEM);
    let mut sem0 = hsem.sema0();
    let mut sem1 = hsem.sema1();
    let mut sem2 = hsem.sema2();
    sem1.enable_irq();

    info!("cm7# wake up cm4.");
    sem0.fast_take();
    sem0.release(0);
    loop {
        if ccdr.rcc.is_d2_domain_available() { break; }
    }

    info!("wait to initialize D2 domain");
    loop {
        if sem1.status_irq() {
            sem1.clear_irq();
            break;
        }
    }

    info!("setup shared ringbuffer");
    let mut cm7_to_cm4_shared_ringbuffer = unsafe {
        shared_ringbuffer::SharedRingBuffer::<1024,8>::assign(CM7_TO_CM4_SHARED_RINGBUFFER,
                                                              CM7_TO_CM4_SHARED_RINGBUFFER_SIZE)
    };

    let (sem3op, mut sem3intr) = hsem.sema3().split();
    let hwcs = HardwareCriticalSection {
        procid: 1,
        sem: RefCell::new(sem3op)
    };

    let mut cm4_to_cm7_shared_ringbuffer = unsafe {
        shared_ringbuffer::SharedRingBufferWithCS::<1024, 8, HardwareCriticalSection>
            ::assign(CM4_TO_CM7_SHARED_RINGBUFFER,
                     CM4_TO_CM7_SHARED_RINGBUFFER_SIZE,
                     hwcs)
    };

    sem3intr.enable_irq();

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

    let mut timer = dp.TIM2.timer(1.Hz(), ccdr.peripheral.TIM2, &ccdr.clocks);
    timer.listen(timer::Event::TimeOut); //Enable Interrupt
    timer.start(MilliSeconds::from_ticks(1000).into_rate());

    cm_interrupt::free(|cs| {
        TIMER.borrow(cs).replace(Some(timer));
    });

    unsafe {
        cp.NVIC.set_priority(interrupt::TIM2, 1);
        NVIC::unmask::<stm32h7xx_hal::interrupt>(interrupt::TIM2);
    }

    // Configure PB0 as output.
    let mut led = gpiob.pb0.into_push_pull_output();

    let _ = writeln!(usart_tx,"start blinking LD1                  \r");

    let mut console =
        unsafe {
            console::Console::new(
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
                },
                Some(move |command:&str| {
                    if sem2.take(1) {
                        debug!("send {} <{}>", command.len(), command);
                        let _ = cm7_to_cm4_shared_ringbuffer.write(command.as_bytes());
                        sem2.release(1);
                    }
                }))
        };

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

            if sem3intr.status_irq() {
                MESSAGE_NOTIFY.store(true, Ordering::SeqCst);
                sem3intr.clear_irq();
            }
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
                /*
                match cm4_to_cm7_shared_ringbuffer.read(&mut recvbuf) {
                    Ok(_readsize) => {
                let _ = write!(console,">cm4> {}\r\n", core::str::from_utf8(&recvbuf).unwrap());
                    },
                    Err(e) => { debug!("read error: {}", e); }
                }
                 */
                let _ = write!(console,">cm4> {}\r\n", "notify");
            }
        }
    }
}

#[stm32h7xx_hal::interrupt]
fn TIM2() {
    SEC_COUNTER.fetch_add(1, Ordering::SeqCst);
    cortex_m::interrupt::free(|cs| {
        let mut rc = TIMER.borrow(cs).borrow_mut();
        let timer = rc.as_mut().unwrap();
        timer.clear_irq();
        let mut led_blink = LED_BLINK.borrow(cs).borrow_mut();
        *led_blink = !(*led_blink);
    });
}

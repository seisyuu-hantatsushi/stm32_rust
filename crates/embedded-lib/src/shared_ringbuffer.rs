
use core::slice;
use log::debug;

#[repr(C)]
#[derive(Debug)]
struct SharedRingBufferLayout<const S:usize,const N:usize> {
    head: u32,
    tail: u32,
    start_of_frame: *mut [u8;S],
}

pub struct SharedRingBuffer<const S:usize,const N:usize, LockFn, UnlockFn>
where
    LockFn: FnMut(), UnlockFn: FnMut()
{
    shared_address  : *mut u32,
    buffer_size : u32,
    lock: Option<LockFn>,
    unlock: Option<UnlockFn>,
    shared_ringbuffer: &'static mut SharedRingBufferLayout<S, N>,
    shared_ringbuffer_holder: &'static mut [[u8;S]],
}

impl<const S:usize,const N:usize, LockFn, UnlockFn> SharedRingBuffer<S,N,LockFn,UnlockFn>
where
    LockFn: FnMut(), UnlockFn: FnMut()
{

    pub unsafe fn assign(shared_address: *mut u32,
                         buffer_size: u32,
                         lock_fns: Option<(LockFn, UnlockFn)>) -> Self {
        let shared_ringbuffer_ptr = shared_address as *mut SharedRingBufferLayout<S, N>;
        let mut shared_ringbuffer: &mut SharedRingBufferLayout<S, N> = unsafe { shared_ringbuffer_ptr.as_mut().unwrap() };
        if (buffer_size as usize) < 512+S*N { panic!("memory size is not enough. memory size must be rather than {}", S*N+512); };
        shared_ringbuffer.start_of_frame = (shared_address as *mut [u8;S]).add(128);
        let shared_ringbuffer_holder = slice::from_raw_parts_mut(shared_ringbuffer.start_of_frame, N);
        debug!("{:?}", shared_ringbuffer);
        let (lock_fn, unlock_fn) = if let Some((lock_fn, unlock_fn)) = lock_fns { (Some(lock_fn),Some(unlock_fn)) } else { (None,None) };

        SharedRingBuffer {
            shared_address,
            buffer_size,
            lock : lock_fn,
            unlock: unlock_fn,
            shared_ringbuffer,
            shared_ringbuffer_holder
        }
    }

    pub fn write(&mut self) {
        debug!("head {}, tail {}",
               self.shared_ringbuffer.head,
               self.shared_ringbuffer.tail);
        self.shared_ringbuffer.tail += 1;
        if self.shared_ringbuffer.tail >= N as u32 { self.shared_ringbuffer.tail = 0 };
    }

    pub fn read(&mut self) -> usize {
        debug!("head {}, tail {}",
               self.shared_ringbuffer.head,
               self.shared_ringbuffer.tail);
        let head = self.shared_ringbuffer.head;
        self.shared_ringbuffer.head += 1;
        if self.shared_ringbuffer.head >= N as u32 { self.shared_ringbuffer.head = 0 };
        head as usize
    }

}

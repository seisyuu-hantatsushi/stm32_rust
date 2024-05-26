
use core::{slice,fmt};
use log::debug;

pub enum SharedRingBufferError {
    NoSpace, NoData
}

impl fmt::Display for SharedRingBufferError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SharedRingBufferError::NoSpace => write!(f, "No Space"),
            SharedRingBufferError::NoData => write!(f, "No Data")
        }
    }
}

#[repr(C)]
#[derive(Debug)]
struct SharedRingBufferLayout<const S:usize,const N:usize> {
    head: usize,
    tail: usize,
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
        shared_ringbuffer.start_of_frame = (shared_address as *mut u8).add(128) as *mut [u8; S];
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

    pub fn write(&mut self, message : &[u8]) -> Result<(), SharedRingBufferError>{
        debug!("head {}, tail {}",
               self.shared_ringbuffer.head,
               self.shared_ringbuffer.tail);
        if let Some(ref mut lock_fn) = self.lock {
            (lock_fn)();
            if let Some(ref mut unlock_fn) = self.unlock {
                (unlock_fn)();
            };
        }
        else {
            let next_tail = if self.shared_ringbuffer.tail + 1 >= N { 0 } else { self.shared_ringbuffer.tail + 1 };
            if next_tail == self.shared_ringbuffer.head {
                return Err(SharedRingBufferError::NoSpace);
            }
            let mut dst_buffer = self.shared_ringbuffer_holder[self.shared_ringbuffer.tail].as_mut_slice();

            dst_buffer[..message.len()].copy_from_slice(message);
            self.shared_ringbuffer.tail = next_tail;
        }
        Ok(())
    }

    pub fn read(&mut self, message : &mut [u8]) -> Result<usize, SharedRingBufferError> {
        debug!("head {}, tail {}",
               self.shared_ringbuffer.head,
               self.shared_ringbuffer.tail);
        let head = self.shared_ringbuffer.head;
        let tail = self.shared_ringbuffer.tail;

        if head == tail {
            return Err(SharedRingBufferError::NoData);
        }

        let src = self.shared_ringbuffer_holder[self.shared_ringbuffer.head].as_slice();
        let dst_size = message.len();
        let src_size = src.len();

        let copy_size = if dst_size < src_size { dst_size } else { src_size };

        message[..copy_size].copy_from_slice(&src[..copy_size]);

        self.shared_ringbuffer.head += 1;
        if self.shared_ringbuffer.head >= N { self.shared_ringbuffer.head = 0 };

        Ok(copy_size)
    }

}

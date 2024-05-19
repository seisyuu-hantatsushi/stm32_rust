
use core::iter::Iterator;
use core::option::Option;
use core::option::Option::*;
use core::ops::FnMut;
use core::str;

use log::debug;

const DEL:u8 = b'\x7f';
const ESC:u8 = b'\x1b';
const CR:u8  = b'\x0d';
const LF:u8  = b'\x0a';

const CURSOL_NEXT:[u8;4] = [ESC, b'[', b'1', b'C'];
const CURSOL_PREV:[u8;4] = [ESC, b'[', b'1', b'D'];
#[cfg(not(all()))]
const DEL_CHARS_LEFT_SIDE_OF_CURSOR:[u8;4] = [ ESC, b'[', b'1', b'K' ];

enum InputMode {
    Normal, Esc, CsiFirst
}

pub struct Console<Getc, Putc, InputStr>
where
    Getc: FnMut() -> Option<u8>,
    Putc: FnMut(u8),
    InputStr: FnMut(&str)
{
    cursor_pos: usize,
    tail_pos: usize,
    input_mode: InputMode,
    csi_pn: usize,
    buffer: &'static mut [u8],
    prompt: &'static str,
    getc: Getc,
    putc: Putc,
    input_str: Option<InputStr>
}

impl<Getc,Putc,InputStr> Console<Getc,Putc,InputStr>
where Getc: FnMut() -> Option<u8>,
      Putc: FnMut(u8),
      InputStr: FnMut(&str)
{
    fn move_cursor_prev(&mut self) {
        for c in CURSOL_PREV {
            ((*self).putc)(c);
        }
    }

    fn move_cursor_next(&mut self){
        for c in CURSOL_NEXT {
            ((*self).putc)(c);
        }
    }

    #[cfg(not(all()))]
    fn del_chars_left_side_of_cursor(&mut self) {
        for c in DEL_CHARS_LEFT_SIDE_OF_CURSOR {
            ((*self).putc)(c);
        }
    }

    fn input_normal(&mut self, c:u8){
        if c.is_ascii_control() {
            match c {
                CR => {
                    //debug!("input CR");
                    if let Some(ref mut input_str) = (*self).input_str {
                        if let Ok(command) = str::from_utf8(&(*self).buffer) {
                            (input_str)(command);
                        }
                    }
                    ((*self).putc)(LF);
                    ((*self).putc)(CR);
                    (*self).cursor_pos = 0;
                    (*self).tail_pos = 0;
                    (*self).buffer.fill(0);
                    for c in (*self).prompt.chars() {
                        ((*self).putc)(c as u8)
                    }
                },
                DEL => {
                    // Back Space
                    if (*self).tail_pos > 0 {
                        if (*self).cursor_pos < (*self).tail_pos {
                            if (*self).cursor_pos > 0 {
                                for p in (*self).cursor_pos..(*self).tail_pos {
                                    (*self).buffer[p-1] = (*self).buffer[p];
                                }
                                (*self).buffer[(*self).tail_pos-1] = 0;
                                (*self).move_cursor_prev();
                                (*self).cursor_pos -= 1;
                                (*self).tail_pos -= 1;
                                for p in (*self).cursor_pos..(*self).tail_pos {
                                    ((*self).putc)((*self).buffer[p]);
                                }
                                ((*self).putc)(b' ');
                                for _ in (*self).cursor_pos..=(*self).tail_pos {
                                    (*self).move_cursor_prev();
                                }
                            }
                        }
                        else {
                            (*self).move_cursor_prev();
                            ((*self).putc)(b' ');
                            (*self).move_cursor_prev();
                            (*self).buffer[(*self).tail_pos] = 0;
                            (*self).cursor_pos -= 1;
                            (*self).tail_pos -= 1;
                        }
                    }
                },
                ESC => {
                    debug!("input ESC");
                    (*self).input_mode = InputMode::Esc
                },
                _ => ()
            }
        }
        else {
            if (*self).cursor_pos < (*self).tail_pos {
                for p in ((*self).cursor_pos..=(*self).tail_pos).rev() {
                    (*self).buffer[p] = (*self).buffer[p-1];
                }
                (*self).buffer[(*self).cursor_pos] = c;

                for p in (*self).cursor_pos..=(*self).tail_pos {
                    ((*self).putc)((*self).buffer[p]);
                }

                for _ in (*self).cursor_pos..(*self).tail_pos {
                    (*self).move_cursor_prev();
                }
                (*self).tail_pos += 1;
                (*self).cursor_pos += 1;
            }
            else {
                (*self).buffer[(*self).cursor_pos] = c;
                (*self).cursor_pos += 1;
                (*self).tail_pos += 1;
                ((*self).putc)(c)
            }
        }
    }

    fn input_esc(&mut self, c:u8){
        match c {
            b'[' => { (*self).input_mode = InputMode::CsiFirst; self.csi_pn = 0; },
            _ => { debug!(" ESC unknown code {:02x}", c); (*self).input_mode = InputMode::Normal; }
        }
    }

    fn input_csi_first(&mut self, c:u8){
        if c.is_ascii_digit() {
            self.csi_pn = (c - b'0') as usize;
        }
        else {
            match c {
                b'D' => {
                    if (*self).cursor_pos > 0 {
                        (*self).move_cursor_prev();
                        (*self).cursor_pos -= 1;
                    };
                    (*self).input_mode = InputMode::Normal;
                },
                b'C' => {
                    if (*self).cursor_pos < (*self).tail_pos {
                        (*self).move_cursor_next();
                        (*self).cursor_pos += 1;
                    }
                    (*self).input_mode = InputMode::Normal;
                },
                b'~' => {
                    // DEL
                    if (*self).cursor_pos < (*self).tail_pos {
                        for p in (*self).cursor_pos..(*self).tail_pos {
                            if p == (*self).buffer.len()-1 {
                                (*self).buffer[p] = 0;
                            }
                            else {
                                (*self).buffer[p] = (*self).buffer[p+1];
                            }
                        }

                        (*self).tail_pos -= 1;
                        for p in (*self).cursor_pos..(*self).tail_pos {
                            ((*self).putc)((*self).buffer[p]);
                        }
                        ((*self).putc)(b' ');
                        for _ in (*self).cursor_pos..=(*self).tail_pos {
                            (*self).move_cursor_prev();
                        }
                    }
                    (*self).input_mode = InputMode::Normal;
                },
                _ => {
                    debug!("unknown csi code {:02x}", c);
                    (*self).input_mode = InputMode::Normal;
                }
            }
        }
    }

    pub unsafe fn new(buffer: &'static mut [u8],
                      prompt: &'static str,
                      getc : Getc,
                      mut putc : Putc,
                      input_str: Option<InputStr>) -> Self {

        for c in prompt.chars() {
            (putc)(c as u8)
        }

        Self {
            cursor_pos : 0,
            tail_pos : 0,
            input_mode: InputMode::Normal,
            csi_pn: 0,
            buffer,
            prompt,
            getc,
            putc,
            input_str,
        }
    }

    pub fn input(&mut self) {
        if let Some(c) = (self.getc)() {
            //debug!("input {:02x}", c);
            if self.tail_pos >= self.buffer.len() { return (); }
            match self.input_mode {
                InputMode::Normal => self.input_normal(c),
                InputMode::Esc => self.input_esc(c),
                InputMode::CsiFirst => self.input_csi_first(c),
            }
        }
    }

    pub fn output(&mut self, message: &str) {
        for c in message.bytes() {
            (self.putc)(c)
        }
    }
}

#![feature(asm)]
#![feature(const_fn)]
#![feature(lang_items)]
#![no_std]

extern crate spin;
extern crate x86;

use spin::Mutex;
use core::fmt;

pub const DEFAULT_COLOR: ColorCode = ColorCode::new(Color::LightGreen, Color::Black);
const CONSOLE_COLS: isize = 80;
const CONSOLE_ROWS: isize = 25;

#[repr(u8)]
pub enum Color {
   Black = 0,
   Blue = 1,
   Green = 2,
   Cyan = 3,
   Red = 4,
   Magenta = 5,
   Brown = 6,
   LightGray = 7,
   DarkGray = 8,
   LightBlue = 9,
   LightGreen = 10,
   LightCyan = 11,
   LightRed = 12,
   LightMagenta = 13,
   Yellow = 14,
   White = 15,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ColorCode(u8);

impl ColorCode {
    const fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Copy,Clone)]
#[repr(C)]
struct VgaCell {
    character: u8,
    color: ColorCode,
}

pub static BUFFER: Mutex<VgaBuffer> = Mutex::new(VgaBuffer {
    buffer: [VgaCell {
        character: ' ' as u8,
        color: DEFAULT_COLOR,
    }; (CONSOLE_ROWS * CONSOLE_COLS) as usize],
    position: 0,
});

pub struct VgaBuffer {
    buffer: [VgaCell; (CONSOLE_ROWS * CONSOLE_COLS) as usize],
    position: usize,
}

impl VgaBuffer {
    fn write_byte(&mut self, byte: u8, color: ColorCode) {
        if byte == ('\n' as u8) {
            // to get the current line, we divide by the length of a line
            let current_line = (self.position as isize) / CONSOLE_COLS;

            if current_line + 1 >= CONSOLE_ROWS {
                self.scroll_up();
            } else {
                self.position = ((current_line + 1) * CONSOLE_COLS) as usize;
            }
        } else {
            if self.position >= self.buffer.len() {
                self.scroll_up();
            }
            let cell = &mut self.buffer[self.position];

            *cell = VgaCell {
                character: byte,
                color: color,
            };

            self.position += 1;
        }
        set_cursor(self.position as u16);
    }

    fn scroll_up(&mut self) {
        let end = CONSOLE_ROWS * CONSOLE_COLS;

        for i in CONSOLE_COLS..(end) {
            let prev = i - CONSOLE_COLS;
            self.buffer[prev as usize] = self.buffer[i as usize];
        }

        // blank out the last row
        for i in (end - CONSOLE_COLS)..(end) {
            let cell = &mut self.buffer[i as usize];
            *cell = VgaCell {
                character: ' ' as u8,
                color: DEFAULT_COLOR,
            };
        }

        self.position = (end - CONSOLE_COLS) as usize;
    }

    fn reset_position(&mut self) {
        self.position = 0;
        set_cursor(0);
    }

    pub fn flush(&self) {
        unsafe {
            let vga = 0xb8000 as *mut u8;
            let length = self.buffer.len() * 2;
            let buffer = self.buffer.as_ptr() as *const u8;
            core::ptr::copy_nonoverlapping(buffer, vga, length);
        }
    }

    fn clear(&mut self) {
        for i in 0..(CONSOLE_ROWS * CONSOLE_COLS) {
            let cell = &mut self.buffer[i as usize];
            *cell = VgaCell {
                character: ' ' as u8,
                color: DEFAULT_COLOR,
            };
        }

        self.reset_position();

        self.flush();
    }
}

impl fmt::Write for VgaBuffer {
    fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
        let color = DEFAULT_COLOR;
        for byte in s.bytes() {
            self.write_byte(byte, color)
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! kprintln {
    ($fmt:expr) => (kprint!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (kprint!(concat!($fmt, "\n"), $($arg)*));
}

#[macro_export]
macro_rules! kprint {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let mut b = $crate::BUFFER.lock();
        b.write_fmt(format_args!($($arg)*)).unwrap();
        b.flush();
    });
}

/// Clears the console
pub fn clear_console() {
    let mut b = BUFFER.lock();
    b.clear();
}

/// Initializes the cursor
pub fn initialize_cursor() {
    unsafe {
        // Setup cursor start register (0x0Ah)
        // Bits 0-4: Scanline start (where the cursor beings on the y axis)
        // Bit    5: Visibility status (0 = visible, 1 = invisible)
        x86::io::outb(0x3D4, 0x0A);
        x86::io::outb(0x3D5, 0x00);

        // Setup cursor end register (0x0Bh)
        // Bits 0-4: Scanline end (where the cursor ends on the y axis)
        x86::io::outb(0x3D4, 0x0B);
        x86::io::outb(0x3D5, 0x0F); // Scanline 0x0-0xF creates 'block' cursor, 0xE-0xF creates underscore
    }
}

fn set_cursor(position: u16) {
    unsafe {
        // Set cursor low
        x86::io::outb(0x3D4, 0x0F);
        x86::io::outb(0x3D5, position as u8);
        // Set cursor high
        x86::io::outb(0x3D4, 0x0E);
        x86::io::outb(0x3D5, (position >> 8) as u8);
    }
}

use core::fmt::{Arguments, Write};
use spinning_top::Spinlock;
use uart_16550::SerialPort;

static PORT: Spinlock<SerialPort> = Spinlock::new(unsafe { SerialPort::new(0x3F8) });

pub fn init() {
    PORT.lock().init();
}

fn write_fmt(args: Arguments) {
    let _ = PORT.lock().write_fmt(args);
}

#[macro_export]
macro_rules! print {
    ($($t:tt)*) => { $crate::serial::print_fmt(format_args!($($t)*)) };
}

#[macro_export]
macro_rules! println {
    () => { $crate::print!("\r\n") };
    ($($t:tt)*) => { $crate::print!("{}\r\n", format_args!($($t)*)) };
}

pub fn print_fmt(args: Arguments) {
    write_fmt(args);
}

pub fn read_byte() -> u8 {
    // bloqueante; lock mantido (single-core; panic em read trava — aceito na Fase 1)
    PORT.lock().receive()
}

pub fn read_line(buf: &mut [u8]) -> usize {
    let mut n = 0;
    loop {
        let b = read_byte();
        if b == b'\r' || b == b'\n' {
            print!("\r\n");
            return n;
        } else if b == 0x7f {
            if n > 0 {
                n -= 1;
                print!("\x08 \x08");
            }
        } else if n < buf.len() {
            buf[n] = b;
            n += 1;
            print!("{}", b as char);
        }
    }
}

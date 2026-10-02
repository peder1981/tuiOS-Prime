//! Driver GPU basico — framebuffer VGA
//! Modo texto VGA (80x25)

use core::ptr;
use spin::Mutex;


/// Enderecos de display VGA
const VGA_MEM: *mut u16 = 0xB8000 as *mut u16;
const VGA_WIDTH: usize = 80;
const VGA_HEIGHT: usize = 25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuMode {
    Text,
    Framebuffer,
}

pub struct Gpu {
    mode: GpuMode,
    cursor_x: usize,
    cursor_y: usize,
}

impl Gpu {
    pub fn new() -> Self {
        Gpu {
            mode: GpuMode::Text,
            cursor_x: 0,
            cursor_y: 0,
        }
    }

    pub fn clear(&mut self) {
        // VGA write disabled in UEFI mode - use serial only
        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    pub fn putc(&mut self, c: char) {
        match c {
            '\n' => {
                self.cursor_x = 0;
                self.cursor_y = self.cursor_y.saturating_add(1);
            }
            '\r' => {
                self.cursor_x = 0;
            }
            '\t' => {
                let next_tab = (self.cursor_x + 8) & !7;
                while self.cursor_x < next_tab && self.cursor_x < VGA_WIDTH {
                    self.putch(' ');
                }
            }
            c => {
                self.putch(c);
            }
        }
    }

    fn putch(&mut self, _c: char) {
        // VGA write disabled in UEFI mode - use serial only
        if self.cursor_x >= VGA_WIDTH {
            self.cursor_x = 0;
            self.cursor_y = self.cursor_y.saturating_add(1);
        }
        self.cursor_x = self.cursor_x.saturating_add(1);
    }

    fn scroll(&mut self) {
        self.cursor_y = VGA_HEIGHT - 1;
    }
}

// GPU global simplificado
use core::cell::RefCell;
use core::ops::{Deref, DerefMut};

pub struct GpuCell {
    inner: RefCell<Gpu>,
}

unsafe impl Sync for GpuCell {}
unsafe impl Send for GpuCell {}

impl Deref for GpuCell {
    type Target = RefCell<Gpu>;
    fn deref(&self) -> &Self::Target { &self.inner }
}
impl DerefMut for GpuCell {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
}

#[no_mangle]
static mut GPU: Option<GpuCell> = None;

pub fn init() {
    unsafe {
        GPU = Some(GpuCell { inner: RefCell::new(Gpu::new()) });
        GPU.as_mut().unwrap().borrow_mut().clear();
    }
}

pub fn putc(c: char) {
    unsafe {
        if let Some(ref gpu) = GPU {
            gpu.borrow_mut().putc(c);
        }
    }
}

pub fn clear() {
    unsafe {
        if let Some(ref gpu) = GPU {
            gpu.borrow_mut().clear();
        }
    }
}

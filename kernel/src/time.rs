use core::sync::atomic::Ordering;

pub fn millis() -> u64 {
    crate::arch::TICKS.load(Ordering::Relaxed)
}

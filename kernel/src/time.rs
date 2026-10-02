use core::sync::atomic::Ordering;

pub fn millis() -> u64 {
    crate::arch::TICKS.load(Ordering::Relaxed)
}

pub mod smol_instant {
    use smoltcp::time::Instant;
    pub fn now() -> Instant {
        Instant::from_millis(super::millis() as i64)
    }
}

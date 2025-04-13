use std::time;
#[cfg(target_os = "linux")]
fn high_precision_time() -> std::time::Duration {
    use std::mem::MaybeUninit;
    let mut ts = MaybeUninit::<nix::libc::timespec>::uninit();
    unsafe {
        nix::libc::clock_gettime(nix::libc::CLOCK_MONOTONIC_RAW, ts.as_mut_ptr());
        let ts = ts.assume_init();
        std::time::Duration::new(ts.tv_sec as u64, ts.tv_nsec as u32)
    }
}

pub struct Perf {
    #[cfg(windows)]
    start: time::Instant,
    #[cfg(unix)]
    start: time::Duration,
    duration: time::Duration,
}

impl std::fmt::Display for Perf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.duration)
    }
}

impl Perf {
    pub fn new() -> Self {
        Self {
            #[cfg(windows)]
            start: time::Instant::now(),
            #[cfg(unix)]
            start: high_precision_time(),

            duration: time::Duration::ZERO,
        }
    }

    pub fn record(&mut self) {
        #[cfg(unix)]
        {
            self.duration = high_precision_time() - self.start;
        }

        #[cfg(windows)]
        {
            self.duration =  self.start.elapsed();
        }

    }
}

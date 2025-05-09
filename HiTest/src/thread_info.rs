use std::sync::{Mutex, Once};
use std::thread;

pub struct ThreadInfo {
    main_thread_id: Option<thread::ThreadId>,
}

static mut INSTANCE: Option<Mutex<ThreadInfo>> = None;
static INIT: Once = Once::new();

impl ThreadInfo {
    // this function should be called in main thread at first
    pub fn get_instance() -> &'static Mutex<ThreadInfo> {
        INIT.call_once(|| unsafe {
            INSTANCE = Some(Mutex::new(ThreadInfo {
                main_thread_id: Some(thread::current().id()),
            }));
        });
        #[cfg_attr(unix, allow(static_mut_refs))]
        unsafe { INSTANCE.as_ref().unwrap() }
    }

    pub fn is_main_thread(&self) -> bool {
        self.main_thread_id
            .map_or(false, |id| id == thread::current().id())
    }
}

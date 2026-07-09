use std::alloc::{GlobalAlloc, Layout, System};
use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct CountingAllocator;

#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

static CURRENT_BYTES: AtomicUsize = AtomicUsize::new(0);
static PEAK_BYTES: AtomicUsize = AtomicUsize::new(0);
static CURRENT_USABLE_BYTES: AtomicUsize = AtomicUsize::new(0);
static PEAK_USABLE_BYTES: AtomicUsize = AtomicUsize::new(0);
static TOTAL_ALLOCATED_BYTES: AtomicUsize = AtomicUsize::new(0);
static TOTAL_DEALLOCATED_BYTES: AtomicUsize = AtomicUsize::new(0);
static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static DEALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static REALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy)]
pub struct AllocStats {
    pub current_bytes: usize,
    pub peak_bytes: usize,
    pub current_usable_bytes: usize,
    pub peak_usable_bytes: usize,
    pub total_allocated_bytes: usize,
    pub total_deallocated_bytes: usize,
    pub alloc_count: usize,
    pub dealloc_count: usize,
    pub realloc_count: usize,
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            record_alloc(layout.size());
            record_usable_alloc(usable_size(ptr, layout.size()));
            ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let usable = usable_size(ptr, layout.size());
        unsafe { System.dealloc(ptr, layout) };
        record_dealloc(layout.size());
        record_usable_dealloc(usable);
        DEALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let old_usable = usable_size(ptr, layout.size());
        let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() {
            let old_size = layout.size();
            let new_usable = usable_size(new_ptr, new_size);
            if new_size >= old_size {
                record_alloc(new_size - old_size);
            } else {
                record_dealloc(old_size - new_size);
            }
            if new_usable >= old_usable {
                record_usable_alloc(new_usable - old_usable);
            } else {
                record_usable_dealloc(old_usable - new_usable);
            }
            REALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        new_ptr
    }
}

pub fn snapshot() -> AllocStats {
    AllocStats {
        current_bytes: CURRENT_BYTES.load(Ordering::Relaxed),
        peak_bytes: PEAK_BYTES.load(Ordering::Relaxed),
        current_usable_bytes: CURRENT_USABLE_BYTES.load(Ordering::Relaxed),
        peak_usable_bytes: PEAK_USABLE_BYTES.load(Ordering::Relaxed),
        total_allocated_bytes: TOTAL_ALLOCATED_BYTES.load(Ordering::Relaxed),
        total_deallocated_bytes: TOTAL_DEALLOCATED_BYTES.load(Ordering::Relaxed),
        alloc_count: ALLOC_COUNT.load(Ordering::Relaxed),
        dealloc_count: DEALLOC_COUNT.load(Ordering::Relaxed),
        realloc_count: REALLOC_COUNT.load(Ordering::Relaxed),
    }
}

pub fn mib(bytes: usize) -> f64 {
    bytes as f64 / 1024.0 / 1024.0
}

fn record_alloc(size: usize) {
    TOTAL_ALLOCATED_BYTES.fetch_add(size, Ordering::Relaxed);
    let current = CURRENT_BYTES.fetch_add(size, Ordering::Relaxed) + size;
    update_peak(current);
}

fn record_dealloc(size: usize) {
    TOTAL_DEALLOCATED_BYTES.fetch_add(size, Ordering::Relaxed);
    CURRENT_BYTES.fetch_sub(size, Ordering::Relaxed);
}

fn update_peak(current: usize) {
    let mut peak = PEAK_BYTES.load(Ordering::Relaxed);
    while current > peak {
        match PEAK_BYTES.compare_exchange_weak(peak, current, Ordering::Relaxed, Ordering::Relaxed)
        {
            Ok(_) => break,
            Err(actual) => peak = actual,
        }
    }
}

fn record_usable_alloc(size: usize) {
    let current = CURRENT_USABLE_BYTES.fetch_add(size, Ordering::Relaxed) + size;
    update_usable_peak(current);
}

fn record_usable_dealloc(size: usize) {
    CURRENT_USABLE_BYTES.fetch_sub(size, Ordering::Relaxed);
}

fn update_usable_peak(current: usize) {
    let mut peak = PEAK_USABLE_BYTES.load(Ordering::Relaxed);
    while current > peak {
        match PEAK_USABLE_BYTES.compare_exchange_weak(
            peak,
            current,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,
            Err(actual) => peak = actual,
        }
    }
}

fn usable_size(ptr: *mut u8, requested_size: usize) -> usize {
    #[cfg(target_os = "macos")]
    {
        unsafe extern "C" {
            fn malloc_size(ptr: *const c_void) -> usize;
        }

        let usable = unsafe { malloc_size(ptr.cast::<c_void>()) };
        if usable == 0 { requested_size } else { usable }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = ptr;
        requested_size
    }
}

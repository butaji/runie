use runie_core::SharedSnapshot;
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};

struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        System.dealloc(pointer, layout);
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn shared_snapshot_clones_avoid_deep_projection_allocations() {
    let shared = SharedSnapshot::new(vec![0_u8; 4096]);
    let before_shared = ALLOCATIONS.load(Ordering::Relaxed);
    for _ in 0..256 {
        black_box(shared.clone());
    }
    let shared_allocations = ALLOCATIONS.load(Ordering::Relaxed) - before_shared;

    let before_deep = ALLOCATIONS.load(Ordering::Relaxed);
    for _ in 0..256 {
        black_box((*shared).clone());
    }
    let deep_allocations = ALLOCATIONS.load(Ordering::Relaxed) - before_deep;

    assert!(
        shared_allocations <= 1,
        "shared clones allocated repeatedly"
    );
    assert!(
        deep_allocations >= 256,
        "deep clones did not allocate as expected"
    );
    assert!(shared_allocations < deep_allocations);
}

use super::{align_up, Locked};
use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr;


/// # Bump Allocator
/// 
/// Define a BumpAllocator struct, which is the simplest type of allocator
/// 
/// This allocator works by adding items to the heap linearly, not accounting for any new free space. When it reaches the end of the
/// heap, it just gives an OOM error. It works by keeping a counter of allocations, and tracking them. The "next" counter just points 
/// to the end of the allocated heap. When the allocations counter reaches 0, all allocations have been deallocated
pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
    allocations: usize,
}

impl BumpAllocator {
    /// Creates a new empty bump allocator.
    pub const fn new() -> Self {
        BumpAllocator {
            heap_start: 0, // lower bound of heap mem
            heap_end: 0, // upper bound
            next: 0, // start addr of next alloc
            allocations: 0, // num of allocs
        }
    }

    /// Initializes the bump allocator with the given heap bounds.
    ///
    /// This method is unsafe because the caller must ensure that the given
    /// memory range is unused. Also, this method must be called only once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.next = heap_start;
    }
}



/// Set up GlobalAlloc for our BumpAllocator, so we can set it as our global allocator
/// 
/// 
/// Also defines the alloc and dealloc methods
unsafe impl GlobalAlloc for Locked<BumpAllocator> {
    /// Allocate on the heap. We get a reference to a BumpAllocator, and then do checks that we're not out of memory
    /// 
    /// We then add new allocations to bump.allocation, and change the pointer location to the next free bit of memory
    /// 
    /// We return a pointer to the start position of the allocation
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut bump = self.lock(); // get a mutable reference

        let alloc_start = align_up(bump.next, layout.align());
        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end,
            None => return ptr::null_mut(),
        };

        if alloc_end > bump.heap_end {
            ptr::null_mut() // out of memory
        } else {
            bump.next = alloc_end;
            bump.allocations += 1;
            alloc_start as *mut u8
        }
    }

    /// Deallocate on the heap. We subtract one deallocation, and if we're at 0 allocations,
    /// we reset the pointer to the start memory address of the heap.
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        let mut bump = self.lock(); // get a mutable reference

        bump.allocations -= 1;
        if bump.allocations == 0 {
            bump.next = bump.heap_start;
        }
    }
}
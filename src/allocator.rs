use alloc::alloc::{GlobalAlloc, Layout}; // We need these to create our global allocator, as we aren't using std_lib
use core::ptr::null_mut; // Null pointer
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
}; // Used for memory allocation
use linked_list_allocator::LockedHeap; // Allocator. Called lockedheap as it is behind a spinlock (like mutex). Shouldn't allocate in interrupts
                                       // as it will cause a deadlock.


/// Define the memory location where the heap starts
pub const HEAP_START: usize = 0x_4444_4444_0000;
/// Define the heap size (100 KiB). We can increase this as we'd like
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB


/// We define our allocator here, which needs to inherit GlobalAlloc type.
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// We create a zero-sized type as we don't need any fields.
pub struct Dummy;

unsafe impl GlobalAlloc for Dummy {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        panic!("dealloc should be never called")
    }
}

/// This function takes a frame allocator and mapper, then maps the heap into pages
pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    // Create a page range, from the heap start memroy address
    let page_range = {
        // Create a virtual address from our HEAP_START addr
        let heap_start = VirtAddr::new(HEAP_START as u64);
        // Find the heap end by adding the size of the heap (-1 so we get an inclusive bound)
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        // Find the page on the page table that contains the start address
        let heap_start_page = Page::containing_address(heap_start);
        // Same, but end address
        let heap_end_page = Page::containing_address(heap_end);
        // Create an inclusive range, of every page between the two pages
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    // iterate through each page
    for page in page_range {
        // Using the frame allocator (Which we define in memory.rs), allocate a new frame
        // (REMINDER: A frame is just a slice of physical memory, that can have any page value)
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        // Define what flags we want our page table to have for our page and frame. In this case,
        // we want PRESENT and WRITABLE. PRESENT means there is a page PRESENT, and it is WRITABLE
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        // We then map the page to the frame, with the frame allocator, according to the flags.
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)?.flush() // We flush the results, which updates the map
        };
    }

    // Initalize our allocator
    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    Ok(())
}
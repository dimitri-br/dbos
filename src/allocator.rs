pub mod bump; // Bump allocator - the most simple.  Has a counter that only goes up or down. When it is at 0, there are no allocations
pub mod linked_list; // Linked list allocator, which keeps track of free spaces
pub mod fixed_size_block; // Instead of the dynamic sizing of linked list, you have set sizes (Hence fixed_size_block)

use bump::BumpAllocator; // Fast, simple, but not the best as you can't really reuse allocations.
use linked_list::LinkedListAllocator; // Slower, but better as you can assign free memory regions and are not limited by segmentation
use fixed_size_block::FixedSizeBlockAllocator; // Faster than linked lists, but wastes memory.  Better for kernels, as faster performance


use alloc::alloc::{GlobalAlloc, Layout}; // We need these to create our global allocator, as we aren't using std_lib
use core::ptr::null_mut; // Null pointer
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
}; // Used for memory allocation


/// Define the memory location where the heap starts
pub const HEAP_START: usize = 0x_4444_4444_0000;
/// Define the heap size (100 KiB). We can increase this as we'd like
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB


/// We define our allocator here, which needs to inherit GlobalAlloc type.
#[global_allocator] // Select an allocator from the list below (See import notes for specific use cases)
//static ALLOCATOR: Locked<BumpAllocator> = Locked::new(BumpAllocator::new());
//static ALLOCATOR: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new());
static ALLOCATOR: Locked<FixedSizeBlockAllocator> = Locked::new(FixedSizeBlockAllocator::new());



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

/// Align the given address `addr` upwards to alignment `align`.
///
/// Requires that `align` is a power of two.
/// 
/// [See more here](https://os.phil-opp.com/allocator-designs/#introduction)
fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}


/// A wrapper around spin::Mutex to permit trait implementations.
pub struct Locked<A> {
    inner: spin::Mutex<A>,
}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> spin::MutexGuard<A> {
        self.inner.lock()
    }
}
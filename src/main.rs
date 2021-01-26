#![no_std]   // Disable the standard library (as it is OS-dependent)
#![no_main] // Tell the compiler we don't want to use the standard, default
            // Entry point, but instead manually define it ourselves.
#![feature(custom_test_frameworks)] // Setup our custom test framework, as the built-in one relies on the std lib
#![test_runner(dbos::test_runner)] // The test runner is in our lib
#![reexport_test_harness_main = "test_main"]


/// Use our library to get the various macros we want
use dbos::{println, clear_screen};

/// Core libary panic handling. This struct contains panic info, like where the
/// program panicked and what the error was.
use core::panic::PanicInfo;

use bootloader::{BootInfo, entry_point}; // Boot info from our bootloader, for things like paging and memory mapping.

// Define the entry point
entry_point!(kernel_main);

/// # Panic
/// 
/// This function is called on panic. The function should never return, as annotated by the
/// [!] return value (Divergent return).
#[cfg(not(test))] // This panic handler runs when not in test mode
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    dbos::hlt_loop();
}

/// Same as panic handler (not test), except it prints to serial rather than VGA.
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    dbos::test_panic_handler(info);
}


/// # Main
/// 
/// Just our regular old main function. Should be called from [_start](fn._start.html)
fn main() {

}


// Main function
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    clear_screen!(); // Clear the display

    println!("Hello World{}", "!");
    dbos::init();

    use dbos::memory;
    use x86_64::{structures::paging::Page, VirtAddr};
    
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe {
        memory::BootInfoFrameAllocator::init(&boot_info.memory_map)
    };

    // map an unused page
    let page = Page::containing_address(VirtAddr::new(0xdeadbeaf000));
    memory::create_example_mapping(page, &mut mapper, &mut frame_allocator);

    // write the string `New!` to the screen through the new mapping
    let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
    unsafe { page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e)};

    // as before
    #[cfg(test)]
    test_main();

    main();

    dbos::hlt_loop();
}
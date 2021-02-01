#![no_std]   // Disable the standard library (as it is OS-dependent)
#![no_main] // Tell the compiler we don't want to use the standard, default
            // Entry point, but instead manually define it ourselves.
#![feature(custom_test_frameworks)] // Setup our custom test framework, as the built-in one relies on the std lib
#![feature(vec_into_raw_parts)]
#![test_runner(dbos::test_runner)] // The test runner is in our lib
#![reexport_test_harness_main = "test_main"]

/// Use the alloc standard crate
extern crate alloc;

/// Boxes!
use alloc::boxed::Box;
// String support
use alloc::string::{String, ToString};

// Allows string adding and stuff
use core::ops::Add;


/// Use our library to get the various macros we want
use dbos::{println, clear_screen};
use dbos::{memory, allocator, cpu_specs}; // Modules that control memory, the allocator and output CPU info
use dbos::task::{Task, simple_executor::Executor}; // Use our better Executor to run our async tasks
use dbos::driver::keyboard; // Get access to our keyboard module so we can add the print_keypresses async function to our task queue

use x86_64::{structures::paging::Page, VirtAddr}; // We use this to get & create pages, and assign virt addr

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
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    dbos::hlt_loop();
}

/// # Main
/// 
/// Just our regular old main function. Should be called from [kernel_main](fn.kernel_main.html)
fn main(boot_info: &'static BootInfo) {
    //cpu_specs::print_cpu_info();
    //let memory_capacity = memory::get_physical_memory_capacity(&boot_info.memory_map);
    //println!("Free physical memory: {:?}/{:?} MiB\n", memory_capacity.0, memory_capacity.1);

    let mut executor = Executor::new(); // Create a new Executor
    executor.spawn(Task::new(example_task())); // Add a new task to the simple executor
    executor.spawn(Task::new(keyboard::print_keypresses())); // Add our "print_keypresses" task to our executor
    executor.run(); // Run all tasks

    let x = Box::new(41);
    println!("Boxed value: {:?} at (ptr: {:p} -> memory usage: {} bytes)", x, x, core::mem::size_of_val(&x));

    let test_string = String::from("Test String");
    println!("String value before: {} (ptr: {:?} -> memory usage: {} bytes)", test_string.clone(), test_string.clone().into_raw_parts().0,test_string.clone().into_raw_parts().2);

    let test_string = test_string.add(" Sucks!");
    println!("String value after: {} (ptr: {:?} -> memory usage: {} bytes)", test_string.clone(), test_string.clone().into_raw_parts().0,test_string.clone().into_raw_parts().2);
    
    let test_string = test_string.add(" Join me in an adventure when we find out who the murderer is.");
    println!("String value after: {} (ptr: {:?} -> memory usage: {} bytes)", test_string.clone(), test_string.clone().into_raw_parts().0,test_string.clone().into_raw_parts().2);
}


// Main function
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    clear_screen!(); // Clear the display
    dbos::init(); // Initialize the kernel through setting up page tables and stack tables

    // Setup our allocation info. We initialize our memory pages and respective tables.

    // Get the offset from bootinfo
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    // Create the mapper using the offset
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    // Create a frame allocator using our memory map from bootinfo
    let mut frame_allocator = unsafe {
        memory::BootInfoFrameAllocator::init(&boot_info.memory_map)
    };
    // Initialize our allocator heap using the mapper and allocator
    allocator::init_heap(&mut mapper, &mut frame_allocator)
    .expect("heap initialization failed");



    // We can now commence the main program

   


    // We've finished initializing

    /*// map an unused page
    let page = Page::containing_address(VirtAddr::new(0xdeadbeaf000));
    memory::create_example_mapping(page, &mut mapper, &mut frame_allocator);
    // write the string `New!` to the screen through the new mapping
    let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
    unsafe { page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e)};*/

    // as before
    #[cfg(test)]
    test_main();

    main(boot_info);

    dbos::hlt_loop();
}

/* Test async */

async fn async_number() -> u32 {
    42
}

async fn example_task() {
    let number = async_number().await;
    println!("async number: {}", number);
}
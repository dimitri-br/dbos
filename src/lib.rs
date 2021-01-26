#![no_std]
#![cfg_attr(test, no_main)]
#![feature(abi_x86_interrupt)] // Enable interrupts and exception callbacks
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

pub mod vga_buffer; // This module handles writing text to the VGA buffer
pub mod serial; // This module handles writing to the serial port
pub mod interrupts; // This module handles our interrupts and exceptions
pub mod gdt; // Controls kernel/user mode and the various stacks
pub mod memory; // Memory allocation and paging

use core::panic::PanicInfo;

#[cfg(test)]
use bootloader::{entry_point, BootInfo};

/// # init
/// 
/// Initalize our kernel. This will store interrupt initalizing, memory and paging stuff
/// and much, much more.
pub fn init() {
    interrupts::init_idt(); // Load the IDT to the CPU.
    gdt::init(); // init the GDT (Load the TSS and setup the GDT)
    unsafe { interrupts::PICS.lock().initialize() }; // Enable interrupts from the PIC
    x86_64::instructions::interrupts::enable(); // Runs the STI command which enables CPU interrupts (set interrupts)
}

/// # QemuExitCode
/// 
/// Defines the exit codes to write to the IO port, to safely quit a QEMU instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}


/// Function to exit qemu. Use in test conditions.
pub fn exit_qemu(exit_code: QemuExitCode) {
    // x86_64 port struct
    use x86_64::instructions::port::Port;

    // Create a new port at 0xF4 (Which we've told qemu to look out for)
    // then write our exit code (Which can be success or fail, see the enum above)
    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

/// Defines a testable function
/// 
/// Helps to automatically print the test name and success.
pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

/// Runs every test by taking an array of functions, which have `#[test_case]` attribute. This functions is only generated when we run
/// 
/// `cargo test`
/// 
/// This function only takes testable functions. For more refined test control, look at disabling test harnesses.
pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

/// Panic handler for our test framework
/// 
/// It will automatically write out the input to the serial port, which will be picked up by QEMU.serial
/// 
/// We then safely exit.
pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    loop {}
}

/// Halt loop to save CPU cycles and resources
pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[cfg(test)]
entry_point!(test_kernel_main);

/// Entry point for `cargo test`
#[cfg(test)]
fn test_kernel_main(_boot_info: &'static BootInfo) -> ! {
    // like before
    init();
    test_main();
    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}
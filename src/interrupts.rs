// Import our IDT, which defines how exceptions and interrupts are handled
// The x86_64 crate controls how the CPU stores the stack and CPU states when 
// an exception or interrupt is triggered. We also import the stack frame struct
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use x86_64::structures::idt::PageFaultErrorCode;
use crate::hlt_loop;
use crate::{println, print};
use crate::gdt; // Get the double_fault stack index
use lazy_static::lazy_static;
use pic8259_simple::ChainedPics;
use spin;



pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

/// # PICS
/// 
/// A PIC is a Programmable Interrupt Controller. It acts as a buffer between the CPU and interrupts, and runs
/// asynchrynously to the CPU. It can take input from various sources, like mouse, keyboard, Real time clock, 
/// ACPI, a total of 15 interrupts. Interrupts are better than polling as it allows the CPU to react much quicker.
/// 
/// Here, we lock it in a mutex as its mutable state cannot change, especially since it runs async (And especially
/// if we add multiprocessing support).
pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

/// # InterruptIndex
/// 
/// In this enum, we store the offsetted index of each interrupt the 
/// PIC supports. This is due to the fact that 0-32 are already used by the CPU for exceptions.
/// 
/// So, in order to get around this, we offset it by 32. This InterruptIndex struct will 
/// store our interrupt values, to save us time remembering it all.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

lazy_static! {
    /// # IDT
    /// 
    /// Static mutable IDT. See [init_idt](fn.init_idt.html) for more information how this works.
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        
        // Changing stacks is unsafe, as the compiler cannot guarantee the stack exists
        // Stacks cannot be used for multiple exceptions
        unsafe{
            idt.double_fault.set_handler_fn(double_fault_handler).set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX); // We change stacks to preserve memory integrity
        }

        idt.page_fault.set_handler_fn(page_fault_handler); // Set the handler

        idt[InterruptIndex::Timer.as_usize()]
            .set_handler_fn(timer_interrupt_handler); // Add our timer interrupt

        idt[InterruptIndex::Keyboard.as_usize()]
            .set_handler_fn(keyboard_interrupt_handler); // Add our keyboard interrupt


        idt
    };
}

/// # Initialize the IDT
/// 
/// # Important information about this module
/// 
/// `extern "x86-interrupt"` is the calling convention (like the `"C"` calling convention).
/// Calling conventions tell the compiler the various details about a function call, like
/// how parameters are placed in memory, registers and how it returns. This is neccessary
/// for interrupts and exceptions as we need to store the state of the stack (Such as stack pointer position,
/// CPU registers and flags, what stack to switch, err code, instruction pointer, etc). We then need to restore
/// the CPU and stack state if the exception is recoverable. These functions take the stack frame as an input
/// variable, which is kind of like a copy of the above. We can use it to display debug info, and to recover from
/// the exceptions, or as data for use in an interrupt.
/// 
/// Breakpoints are typically recoverable, compared to a triple faults, which are not. 
/// 
/// An example is this function:
/// 
/// ```
/// extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut InterruptStackFrame)
/// {
///    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
/// }
/// 
/// ```
/// 
/// These functions are used as callbacks in the IDT (which the CPU uses as a lookup table when it hits an exception)
pub fn init_idt() {
    // Load our IDT to memory
    IDT.load();
}

/* Exceptions */

// Breakpoint exception handler - it is a "fault", so it can be recovered from
extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut InterruptStackFrame)
{
    // Debug - Output the frame to the VGA buffer
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

// Page fault exception handler (Usually happens when you write to memory you don't own). This function
// never returns, as it is a "trap". Traps cannot be recovered. We include it to prevent a triple fault
extern "x86-interrupt" fn double_fault_handler(stack_frame: &mut InterruptStackFrame, _error_code: u64) -> !
{
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

// Page fault handler
// Much more specific than a generic double fault
// This happens when you try and do something with a page that is not allowed
// It is a non-recoverable fault
extern "x86-interrupt" fn page_fault_handler(stack_frame: &mut InterruptStackFrame, error_code: PageFaultErrorCode) {
    use x86_64::registers::control::Cr2; // CR2 is written to automatically upon a page fault, and contains the
                                         // accessed location that caused it

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    hlt_loop();
}


/* Interrupts */

// Timer interrupt handler. Runs every tick or so.
// Probably got a lot of uses
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: &mut InterruptStackFrame)
{
    //print!(".");
    // Take our mutex, lock it
    // Then tell the PIC that the interrupt has been handled
    // So it can continue serving interrupts
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

// Keyboard interrupt handler
// This gets called on key press and key release
extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: &mut InterruptStackFrame)
{
    use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1}; // Keyboard structs
    use spin::Mutex; // Protect it with a mutex
    use x86_64::instructions::port::Port;

    lazy_static! {
        static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
            Mutex::new(Keyboard::new(layouts::Us104Key, ScancodeSet1,
                HandleControl::Ignore)
            );
    }
    let mut keyboard = KEYBOARD.lock(); // Lock a mutable keyboard ref
    let mut port = Port::new(0x60); // Read IO port 0x60, which is the PS/2 controller port
    let scancode: u8 = unsafe { port.read() }; // The byte we read from the port is the scancode
    // Decode our scancode and output the key
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => print!("{}", character),
                DecodedKey::RawKey(key) => print!("{:?}", key),
            }
        }
    }


    // Let the PIC know that we've finished with the interrupt
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}




/* Testing */

// Test our exception handling works
#[test_case]
fn test_breakpoint_exception() {
    // invoke a breakpoint exception
    x86_64::instructions::interrupts::int3();
}
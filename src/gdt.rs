use x86_64::VirtAddr;
use x86_64::structures::tss::TaskStateSegment;
use x86_64::structures::gdt::SegmentSelector;
use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor};
use lazy_static::lazy_static;
use crate::{serial_println};


/// Define a stack to use as the double fault stack (any stack works)
/// 
/// This stack will be used on a double fault
pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;


/// Create a TSS, which stores our stack tables.
/// This can be used for privilage tables, like for user-only apps. 
/// We define the 0th IST (interrupt stack table) as our double_fault stack. We
/// can use stack switching to ensure that we have a non-corrupt stack when we
/// run into a double fault caused by, for example a stack overflow. NOTE: as we 
/// lack a guard page, we should not do any stack intensive tasks on the double fault stack in case we corrupt the memory below the stack.
/// 
/// TODO: memory allocation
lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
            let stack_end = stack_start + STACK_SIZE;
            stack_end
        };
        tss
    };
}




/// GDT (global descriptor table) is a very old relic of the computing past that was used
/// for memory segmentation before paging was a thing. It is still used today to load TSS and
/// for kernel/user mode configuration. It is used for switching between kernel/user space, and
/// TSS loading. We're currently using it for TSS loading.
/// 
/// We use lazy static as we generate TSS at runtime.
lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        (gdt, Selectors { code_selector, tss_selector })
    };
}

// Helpful struct to load our TSS and cs register
struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

/// # init
/// 
/// Initialize our GDT and TSS. See the `gdt.rs` file for more detailed comments on what they do.
pub fn init() {
    use x86_64::instructions::segmentation::set_cs;
    use x86_64::instructions::tables::load_tss;

    // Load our GDT
    GDT.0.load();
    serial_println!("[LOG] GDT loaded successfully");
    unsafe {
        // Set our code selector (kernel/user mode)
        set_cs(GDT.1.code_selector);
        serial_println!("[LOG] Set GDT code selector");
        // Set our tss selector
        load_tss(GDT.1.tss_selector);
        serial_println!("[LOG] Set GDT tss selector");
    }
}

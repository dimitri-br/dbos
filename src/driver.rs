pub mod keyboard;
pub mod pci;

use keyboard::KeyboardDriver;
use pci::PciScanner;
use lazy_static::lazy_static;
use spin::Mutex;
use crate::serial_println;

lazy_static! {
    /// Static global reference to our DriverHandler
    pub static ref DRIVER_HANDLER: Mutex<DriverHandler> = {
        let mut driver_handler = DriverHandler::new();
        let mut driver_handler = Mutex::new(driver_handler);
        driver_handler
    };
}

/// # DriverHandler
/// 
/// Driver handler controls all kernel level drivers, such as keyboard, usb,
/// networking and more.
/// 
/// It is a global handler that is wrapped within a mutex, so call `lock` to get 
/// access to the `DRIVER_HANDLER` static.
/// 
pub struct DriverHandler{
    pub keyboard_driver: KeyboardDriver,
    pub pci_scanner: PciScanner,
}

impl DriverHandler{
    /// Initialize all our drivers
    pub fn new() -> Self{
        serial_println!("Initialized driver handler");
        Self{
            keyboard_driver: KeyboardDriver::new(),
            pci_scanner: PciScanner::new(),
        }
    }
}
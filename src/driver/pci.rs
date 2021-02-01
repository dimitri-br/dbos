use alloc::vec::Vec;
use tinypci::{brute_force_scan, PciFullClass, PciDeviceInfo};
use crate::{print, println, del_col, serial_println};


/// Scans for PCI devices, and stores the vector of PCI devices
pub struct PciScanner{
    pub devices: Vec::<PciDeviceInfo>,
}

impl PciScanner{
    /// Create a new PciScanner struct. Will scan upon creation
    pub fn new() -> Self{
        serial_println!("Scanned PCI devices!");
        Self{
            devices: brute_force_scan()
        }
    }

    pub fn scan_for_type(&self, pci_type: PciFullClass) -> Vec::<&PciDeviceInfo>{
        let mut scanned_devices = Vec::<&PciDeviceInfo>::new();

        for device in self.devices.iter(){
            if device.full_class == pci_type{
                scanned_devices.push(device);
            }
        }

        scanned_devices
    }
}
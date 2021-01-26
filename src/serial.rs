use uart_16550::SerialPort;
use spin::Mutex;
use lazy_static::lazy_static;



lazy_static! {
    /// Similar to what we did with the vga buffer, except we define a serial port through the uart 16550 crate.
    /// UART - the chips implementing the serial interface. 16550 is compatible with *most* systems
    /// We wrap it in a lazy_static and mutex so we can access the SERIAL1 variable wherever, safely. 
    /// 
    /// Write to the serial port using the macros - [serial_print](../macro.serial_print.html) & [serial_println](../macro.serial_println.html)
    pub static ref SERIAL1: Mutex<SerialPort> = {
        let mut serial_port = unsafe { SerialPort::new(0x3F8) }; // Standard first port for serial
        serial_port.init();
        Mutex::new(serial_port)
    };
}


// Similar to the print macros in vga_buffer, except this works through serial.
#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;
    SERIAL1.lock().write_fmt(args).expect("Printing to serial failed");
}

// function to read from serial port
#[doc(hidden)]
pub fn _read() -> u8 {
    SERIAL1.lock().receive()
}

/// Prints to the host through the serial interface.
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*));
    };
}

/// Prints to the host through the serial interface, appending a newline.
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt, "\n"), $($arg)*));
}

/// Reads from the serial port
#[macro_export]
macro_rules! serial_read {
    () => ($crate::serial::_read());
}
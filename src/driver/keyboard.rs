/// Keyboard driver (Only supports PS/2 right now, USB support to be added)
/// 
/// Controls the various inputs/outputs from the keyboard - and converts scancodes into letters to render. 

use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1}; // Keyboard structs
use spin::Mutex; // Protect it with a mutex
use x86_64::instructions::port::Port;
use crate::{print, del_col};
use lazy_static::lazy_static;

/// KeyboardDriver for PS/2
/// 
/// This struct only contains the port. It converts scancodes into text, and outputs it
/// to the screen. The relevant PS/2 port is `0x60`
/// 
/// 
/// This struct should only be called from the DriverHandler


// Create a static KEYBOARD we can use to convert scancodes into ascii keys
lazy_static! {
    static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
        Mutex::new(Keyboard::new(layouts::Us104Key, ScancodeSet1,
            HandleControl::Ignore)
        );
}


pub struct KeyboardDriver{
    port: Port<u8>,
}

impl KeyboardDriver{
    pub fn new() -> Self{
        Self{
            port: Port::new(0x60) // Read IO port 0x60, which is the PS/2 controller port
        }
    }
    /// print key to the VGA buffer
    pub fn print_key(&mut self){
        let (scancode, key) = self.get_key(); // Get the scancode from the port
        
        let mut keyboard = KEYBOARD.lock(); // Lock a mutable keyboard ref


        // Custom handling
        let should_render_text = match scancode{
            0xE => {del_col!(); false}
            _ => {true}
        };

        // Only render character to screen if we're not using a function key, such as esc, del, backspace etc
        if should_render_text{
            // Decode our scancode and output the key
            if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
                if let Some(key) = keyboard.process_keyevent(key_event) {
                    //key.
                    match key {
                        DecodedKey::Unicode(character) => print!("{}", character),
                        DecodedKey::RawKey(key) => print!("{:?}", key),
                    }
                }
            }
        }
    }

    /// Returns the key as text and scancode
    pub fn get_key(&mut self) -> (u8, char){
        let mut keyboard = KEYBOARD.lock(); // Lock a mutable keyboard ref
        let mut port = &mut self.port; 
        let scancode: u8 = unsafe { port.read() }; // The byte we read from the port is the scancode
        

        // Decode our scancode and output the key
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
                //key.
                match key {
                    DecodedKey::Unicode(character) => return (scancode, character),
                    DecodedKey::RawKey(key) => return (scancode, ' '),
                }
            }
        }
        // If we don't return from the matched key, return an empty character
        return (scancode, ' ');
    }
}
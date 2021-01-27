use volatile::Volatile; // Helps prevent the optimizer optimizing our buffer
use lazy_static::lazy_static; // Allows us to create static structs
use spin::Mutex; // no_std mutex
use core::fmt; // Lets us format strings easily



lazy_static! {
    /// # WRITER_GLOBAL 
    /// Static, mutable reference to a writer. This creates a global writer struct.
    /// 
    /// The lazy_static macro allows us to define static variables that require a runtime variable to work, in this case,
    /// 
    /// a raw pointer to the VGA buffer.
    /// 
    /// Can be modified when you take a [Mutex](../../spin/struct.Mutex.html) and lock it.
    /// 
    /// Not doing this results in unsafe code, as you potentially cause a [race condition](https://doc.rust-lang.org/nomicon/races.html)
    pub static ref WRITER_GLOBAL: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::White, Color::Blue),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}


/// Enum to specify different colors that VGA buffer supports
/// 
/// Just select the color when creating a [Writer](./struct.Writer.html), to define background and text (foreground) color
/// 
/// Can be used in [ColorCode](./struct.ColorCode.html), which is used to define a color set for a [Writer](./struct.Writer.html)
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

/// This struct (Which is just a single u8 bit with the color (Background 0-3, foreground 4-8))
/// Helps with easily creating text background and foregrounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    /// # New
    /// 
    /// Create a new ColorCode, using a foreground and background color
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

/// Screen Char contains the info for a single character. Must be stored in a C like array.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

/// The VGA buffer height (Always 25)
const BUFFER_HEIGHT: usize = 25;
/// The VGA buffer width (always 80)
const BUFFER_WIDTH: usize = 80;

// The buffer stores the text to render
#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

/// # Writer
/// 
/// This struct handles writing text to the screen.
/// 
/// We use `[0xB8000]` as that is the location on the VGA buffer that contains our text.
/// 
/// This example uses external crates to allow us to create a global writer, so we can call it
/// 
/// at any point in any script.
/// 
/// # Example Usage
/// 
/// ```
/// use lazy_static::lazy_static; // Allows us to create static structs
/// use spin::Mutex; // no_std mutex
/// 
/// // Create a global static writer (In a mutex so that we don't run into race conditions)
/// lazy_static! {
///    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
///     column_position: 0,
///     color_code: ColorCode::new(Color::White, Color::Blue),
///     buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
///    });
/// } 
/// ```
/// 
/// It is not reccommended to use this struct directly - instead use the macros avaliable. More info on the macros below.
pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl Writer {
    /// # Use [write_string](struct.Writer.html#method.write_string), as it is probably what you're looking for!
    /// 
    /// This function writes a single byte to the buffer
    /// 
    /// see [ScreenChar](struct.ScreenChar.html) for more information about the way a character is stored.
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                let color_code = self.color_code;
                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code,
                });
                self.column_position += 1;
            }
        }
    }

    /// Create a new line. It works by iterating through every single row and column, moving
    /// 
    /// them up one row. This moves everything up by one, before resetting the column position. 
    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.chars[row][col].read();
                self.buffer.chars[row - 1][col].write(character);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    /// delete a line. It works by iterating through every single row and column, moving
    /// 
    /// them up down row. This moves everything down by one, before resetting the column position. 
    fn del_line(&mut self) {
        let character = ScreenChar{ ascii_character: b' ', color_code: self.color_code };
        let mut copy_buffer = [[character; BUFFER_WIDTH]; BUFFER_HEIGHT];
        for row in 0..BUFFER_HEIGHT-1 {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.chars[row][col].read();
                copy_buffer[row+1][col] = character;
            }
        }

        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = copy_buffer[row][col];
                self.buffer.chars[row][col].write(character);
            }
        }

        self.clear_row(1);
        self.column_position = self.get_char_pos(BUFFER_HEIGHT - 1) + 2;
    }


    /// Get the last character position on a row to wrap the current
    /// column position to the last character, esp when going back a line
    fn get_char_pos(&self, row: usize) -> usize{
        let mut last_char_pos = 0;
        let mut column = 0;
        for col in self.buffer.chars[row].iter(){
            if col.read().ascii_character != b' '{
                last_char_pos = column;
            }
            column += 1;
        }

        last_char_pos
    }

    /// Clear the new row with space (`' '`) characters
    /// 
    /// As it looks cleaner. Also used by [clear_screen](../macro.clear_screen.html)
    /// 
    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }

    /// This function writes a string to the VGA buffer
    /// 
    /// It uses [write_byte](struct.Writer.html#method.write_byte) to write each character to our buffer
    /// 
    /// Recommended to use [println](../macro.println.html) or [print](../macro.print.html) macros, instead of calling directly
    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // not part of printable ASCII range
                _ => self.write_byte(0xfe),
            }

        }
    }

    /// This function reads a row, so you can read the contents of a row (text)
    /// 
    /// Call with the [readln](../macro.readln.html) macro.
    pub fn clear_screen(&mut self){
        for row in 0..BUFFER_HEIGHT{
            self.clear_row(row);
        }
    }

    /// This function deletes the last column, to emulate a backspace
    /// 
    /// Call with the [del_col](../macro.del_col.html) macro.
    pub fn backspace(&mut self){
        if self.column_position <= 0 {
            self.del_line();
        }

        let row = BUFFER_HEIGHT - 1;
        let mut col = self.column_position - 1;
        if col > BUFFER_WIDTH -1{
            col = BUFFER_WIDTH - 1;
        }
        let color_code = self.color_code;
        self.buffer.chars[row][col].write(ScreenChar {
            ascii_character: b' ',
            color_code,
        });
        self.column_position = col;
    }

    /// This function clears each row individually, so you can wipe the contents of the screen
    /// 
    /// Call with the [clear_screen](../macro.clear_screen.html) macro.
    pub fn read(&self) -> Result<[u8; BUFFER_WIDTH], &str>{
        use crate::serial_println;
        let mut row_data: [u8; BUFFER_WIDTH] = [0; BUFFER_WIDTH];

        for i in 0..BUFFER_WIDTH {
            row_data[i] = self.buffer.chars[BUFFER_HEIGHT - 2][i].read().ascii_character;
        }
        
        Ok(row_data)
    }
}

/// Format implementation for the [Writer](./vga_buffer/struct.Writer.html) struct
impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

/// Reimplementation from the std library, as we are unable to use the std library. Prints text directly to the
/// 
/// VGA buffer, in a [Writer](./vga_buffer/Writer.html) struct.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

/// Reimplementation from the std library, as we are unable to use the std library. Prints text (with a newline) directly to the
/// 
/// VGA buffer, in a [Writer](./vga_buffer/struct.Writer.html) struct.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

/// Helpful macro to wipe the contents of the buffer with `' '` characters.
#[macro_export]
macro_rules! clear_screen {
    () => ($crate::vga_buffer::_clear_screen());
}

/// Helpful macro to simulate a backspace
#[macro_export]
macro_rules! del_col {
    () => ($crate::vga_buffer::_backspace());
}

/// Create a _print function that takes the input from the macros, and then takes a lock of
/// our writer, then outputting the contents onto the screen.
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {     // Make sure that no interrupt can run while the writer is locked
        WRITER_GLOBAL.lock().write_fmt(args).unwrap();
    });
}

/// Create a _read function that reads the last row, and returns a string value
pub fn readln() -> [u8; BUFFER_WIDTH] {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;
    let mut row_bytes: [u8; BUFFER_WIDTH] = [0; BUFFER_WIDTH];
    interrupts::without_interrupts(|| {     // Make sure that no interrupt can run while the writer is locked
        row_bytes = WRITER_GLOBAL.lock().read().unwrap();
    });

    row_bytes
}

/// Create a _clear_screen function that clears the screen contents.
/// Useful for wiping screen contents (IE, if you want to reset or something)
#[doc(hidden)]
pub fn _clear_screen() {
    use x86_64::instructions::interrupts;

        interrupts::without_interrupts(|| {     // Make sure that no interrupt can run while the writer is locked
        WRITER_GLOBAL.lock().clear_screen();
    });
}

/// Deletes the last character in the column - useful for backspace
#[doc(hidden)]
pub fn _backspace() {
    use x86_64::instructions::interrupts;

        interrupts::without_interrupts(|| {     // Make sure that no interrupt can run while the writer is locked
        WRITER_GLOBAL.lock().backspace();
    });
}

/* Tests */

// Test our VGA implementations
#[test_case]
fn test_println_simple() {
    println!("test_println_simple output");
}

#[test_case]
fn test_println_many() {
    for _ in 0..200 {
        println!("test_println_many output");
    }
}

#[test_case]
fn test_println_output() {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;


    let s = "Some test string that fits on a single line";
    // We lock the writer for the duration of the test, as otherwise
    // a timer interrupt will run, and ruin the test.
    interrupts::without_interrupts(|| {
        let mut writer = WRITER_GLOBAL.lock();
        writeln!(writer, "\n{}", s).expect("writeln failed");
        for (i, c) in s.chars().enumerate() {
            let screen_char = writer.buffer.chars[BUFFER_HEIGHT - 2][i].read();
            assert_eq!(char::from(screen_char.ascii_character), c);
        }
    });
}
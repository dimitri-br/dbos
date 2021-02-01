/// Keyboard driver (Only supports PS/2 right now, USB support to be added)
/// 
/// Controls the various inputs/outputs from the keyboard - and converts scancodes into letters to render. 

use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1}; // Keyboard structs
use futures_util::stream::StreamExt; // needed for the next() function when reading scancodes from the queue
use spin::Mutex; // Protect it with a mutex
use x86_64::instructions::port::Port;
use crate::{print, println, del_col};
use lazy_static::lazy_static;

/// safe one time initialization of static structs
use conquer_once::spin::OnceCell;
/// Atomic queue that only needs an &self ref to pop and push
use crossbeam_queue::ArrayQueue;
use futures_util::task::AtomicWaker;

/// Queue of scancodes to reduce interrupt time, as we don't want to run CPU intensive tasks during interrupt time
/// 
/// We will do this through async
static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();

/// Atomic waker
/// 
/// Like any waker, except made for atomic things, such as our SCANCODE_QUEUE's ArrayQueue
static WAKER: AtomicWaker = AtomicWaker::new();


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

/// Called by the keyboard interrupt handler
///
/// Must not block or allocate.
pub(crate) fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if let Err(_) = queue.push(scancode) {
            println!("WARNING: scancode queue full; dropping keyboard input");
        } else {
            WAKER.wake(); // wake up our waker, we have new input! we MUST do this after we add the new input to make sure we don't get a race condition
        }
    } else {
        println!("WARNING: scancode queue uninitialized");
    }
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


    pub fn read_scancode(&mut self){
        let mut port = &mut self.port; 
        let scancode: u8 = unsafe { port.read() }; // The byte we read from the port is the scancode

        add_scancode(scancode); // Add a scancode to our scancode queue
    }
}

/// This struct should only be called from this module.
/// 
/// This struct constructs our SCANCODE_QUEUE which we use for asynchrynous scancode polling from the interrupt handler
pub struct ScancodeStream {
    _private: (),
}

impl ScancodeStream {
    /// Create a new scancode stream. This function also initializes the SCANCODE_QUEUE
    pub fn new() -> Self {
        SCANCODE_QUEUE.try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeStream::new should only be called once");
        ScancodeStream { _private: () }
    }
}

use core::{pin::Pin, task::{Poll, Context}};
use futures_util::stream::Stream;


impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
        // Get a reference to our queue
        let queue = SCANCODE_QUEUE
            .try_get()
            .expect("scancode queue not initialized");

        // fast path - we optimistically check there is already a scancode in our queue
        if let Ok(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        // if not :( , we register a new waker - from the context, which will continue this function when the waker awakes.
        WAKER.register(&cx.waker());
        match queue.pop() {
            Ok(scancode) => {
                WAKER.take(); // Remove the waker from the context, we no longer need it
                Poll::Ready(Some(scancode))
            }
            Err(crossbeam_queue::PopError) => Poll::Pending,
        }
    }
}


/// using async, print key to the VGA buffer
pub async fn print_keypresses(){
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(layouts::Us104Key, ScancodeSet1,
        HandleControl::Ignore);

    // Keep looping through scancodes that haven't been handles yet
    while let Some(scancode) = scancodes.next().await {

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
}
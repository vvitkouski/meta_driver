// Remove standard library which cannot be used in the kernel 
#![no_std]

// Required to define panic handler
use core::panic::PanicInfo;

// Define our own panic handler (defeule RUST panic handler was in standard lib)
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}


// Entry point
#[no_mangle]
pub extern "system" fn driver_entry() -> u32 {
    0 /* STATUS_SUCCESS */
}
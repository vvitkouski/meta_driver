// Remove standard library which cannot be used in the kernel
#![no_std]

// Required to define panic handler
use core::panic::PanicInfo;

// Define our own panic handler (defeule RUST panic handler was in standard lib)
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

// Immports
use winapi::km::wdm::{DbgPrint, DbgPrintEx, DRIVER_OBJECT};
use winapi::shared::ntdef::{NTSTATUS, NT_SUCCESS, UNICODE_STRING};
use winapi::shared::ntstatus::STATUS_SUCCESS;

// Entry point
#[no_mangle]
pub extern "system" fn driver_entry(
    driver: &mut DRIVER_OBJECT,
    _: *const UNICODE_STRING,
) -> NTSTATUS {
    unsafe {
        DbgPrintEx(0, 0, "Hello, world!\0".as_ptr());
    }

    driver.DriverUnload = Some(driver_exit);

    STATUS_SUCCESS
}

// Unload
pub extern "system" fn driver_exit(driver: &mut DRIVER_OBJECT) {
    unsafe {
        DbgPrintEx(0, 0, "Bye-bye!\0".as_ptr());
    }
}

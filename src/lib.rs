// Remove standard library which cannot be used in the kernel
#![no_std]

// Immports
use core::panic::PanicInfo;
use winapi::shared::ntdef::{NTSTATUS, UNICODE_STRING, FALSE};
use winapi::shared::ntstatus::{
    STATUS_SUCCESS,
    STATUS_INVALID_PARAMETER
};
use wchar::{wchz, wchar_t};
use winapi::km::wdm::{
    DbgPrintEx,
    IoCreateDevice,
    IoDeleteDevice,
    IoCreateSymbolicLink,
    IoDeleteSymbolicLink,
    IoCompleteRequest,
    IoGetCurrentIrpStackLocation,
    DRIVER_OBJECT,
    PDRIVER_OBJECT,
    DEVICE_TYPE,
    DEVICE_OBJECT,
    PDEVICE_OBJECT,
    IRP,
    PIRP,
    IO_PRIORITY,
    PIO_STACK_LOCATION
};
pub use winapi::shared::minwindef::{
    USHORT,
    ULONG,
    DWORD
};
pub use winapi::um::winioctl::{
    METHOD_BUFFERED,
    FILE_SPECIAL_ACCESS
};
use winapi_local::km::wdm::{
    zeroed_unicode_string,
    RtlInitUnicodeString,
    IRP_MJ_MAXIMUM_FUNCTION,
    IRP_MJ_CREATE,
    IRP_MJ_CLOSE,
    IRP_MJ_DEVICE_CONTROL,
    FILE_DEVICE_SECURE_OPEN,
    DO_DIRECT_IO,
    DO_DEVICE_INITIALIZING
};
use mouse::{
    zeroed_mouse_object,
    mouse_init,
    mouse_event,
    MOUSE_OBJECT,
    PMOUSE_OBJECT,
    MOUSE_LEFT_BUTTON_DOWN,
    MOUSE_LEFT_BUTTON_UP,
    MOUSE_MOVE_ABSOLUTE
};
use keyboard::{
    zeroed_kbd_object,
    kbd_init,
    KBD_OBJECT,
    PKBD_OBJECT
};

// Constants
const IO_DEVICE_NAME: &[wchar_t] = wchz!("\\Device\\MetaDriver");
const IO_SYMLINK_NAME: &[wchar_t] = wchz!("\\??\\MetaDriver");

// IRP CODES & OBJECTS
pub const fn CTL_CODE(
    DeviceType: DWORD,
    Function: DWORD,
    Method: DWORD,
    Access: DWORD,
) -> DWORD {
    (DeviceType << 16) | (Access << 14) | (Function << 2) | Method
}
const META_IRP_MOUSE_EVENT: DWORD = CTL_CODE(DEVICE_TYPE::FILE_DEVICE_UNKNOWN as DWORD, 0xf9004, METHOD_BUFFERED, FILE_SPECIAL_ACCESS);

pub struct MouseRequest {
    x: u32,
    y: u32,
}
pub type PMouseRequest = *mut MouseRequest;

// Define modules
pub mod winapi_local;
pub mod mouse;
pub mod keyboard;

// Static mouse object
static mut GLOBAL_MOUSE_OBJ: MOUSE_OBJECT = zeroed_mouse_object();

// Temporary _fltused fix
#[no_mangle]
pub static _fltused: i32 = 0;

// Define our own panic handler (defeule RUST panic handler was in standard lib)
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

// Entry point
#[no_mangle]
pub extern "system" fn driver_entry(
    driver: &mut DRIVER_OBJECT,
    _: *const UNICODE_STRING,
) -> NTSTATUS {
    // Unsafe code
    unsafe {

        // Debug output
        DbgPrintEx(0, 0, "meta_driver loaded. Start initialization.\0".as_ptr());

        // Create and normalize device name
        let mut io_device_name_unicode: UNICODE_STRING = zeroed_unicode_string();
        RtlInitUnicodeString(&mut io_device_name_unicode as *mut UNICODE_STRING,
            IO_DEVICE_NAME.as_ptr());
        // Debug check
        DbgPrintEx(0, 0, "Unicode device name struct initialized. Buffer size is %u.\0".as_ptr(), io_device_name_unicode.Length as u32);

        // Create and normalize symlink name
        let mut io_symlink_name_unicode: UNICODE_STRING = zeroed_unicode_string();
        RtlInitUnicodeString(&mut io_symlink_name_unicode as *mut UNICODE_STRING,
            IO_SYMLINK_NAME.as_ptr());
        // Debug check
        DbgPrintEx(0, 0, "Unicode device symlink struct initialized. Buffer size is %u.\0".as_ptr(), io_symlink_name_unicode.Length as u32);

        // Create IO device
        let mut device_obj_ptr: PDEVICE_OBJECT = core::ptr::null_mut();
        let status: NTSTATUS = IoCreateDevice(
            driver as PDRIVER_OBJECT,
            0,
            &mut io_device_name_unicode as *mut UNICODE_STRING,
            DEVICE_TYPE::FILE_DEVICE_UNKNOWN,
            FILE_DEVICE_SECURE_OPEN,
            FALSE,
            &mut device_obj_ptr
        );
        DbgPrintEx(0, 0, "IoCreateDevice status: %u\0".as_ptr(), status);

        // Create IO symlink
        let status: NTSTATUS = IoCreateSymbolicLink(&io_symlink_name_unicode, &io_device_name_unicode);
        DbgPrintEx(0, 0, "IoCreateSymbolicLink status: %u\0".as_ptr(), status);

        // Assign driver major functions
        for func_idx in 0..IRP_MJ_MAXIMUM_FUNCTION {
            driver.MajorFunction[func_idx as usize] = Some(irp_mj_unsupported);
        }
        
        driver.MajorFunction[IRP_MJ_CREATE] = Some(irp_mj_create);
        driver.MajorFunction[IRP_MJ_CLOSE] = Some(irp_mj_close);
        driver.MajorFunction[IRP_MJ_DEVICE_CONTROL] = Some(irp_mj_device_control);
        

        // Set device flags
        (*device_obj_ptr).Flags |= DO_DIRECT_IO;
        (*device_obj_ptr).Flags &= !DO_DEVICE_INITIALIZING;

        // Init mouse
        let mouse_init_status = mouse_init(&mut GLOBAL_MOUSE_OBJ as PMOUSE_OBJECT);
        DbgPrintEx(0, 0, "mouse::mouse_init status: %u\0".as_ptr(), mouse_init_status);

        // Init keyboard
        let mut kbd_object = zeroed_kbd_object();
        let kdb_init_status = kbd_init(&mut kbd_object as PKBD_OBJECT);
        
        // Test
        // let mut flags: USHORT = 0;
        // flags |= MOUSE_MOVE_ABSOLUTE;
        // mouse_event(&mut GLOBAL_MOUSE_OBJ as PMOUSE_OBJECT, 10, 10, 0x0000, flags);
        //DbgPrintEx(0, 0, "MOUSE POINTER: %p\0".as_ptr(), GLOBAL_MOUSE_OBJ_PTR);
    }

    // Assign unload function
    driver.DriverUnload = Some(driver_exit);


    // Return success status
    STATUS_SUCCESS
}

// I/O Request Package Major function - create - Unsupported
pub unsafe extern "system" fn irp_mj_unsupported(device: &mut DEVICE_OBJECT, irp: &mut IRP) -> NTSTATUS {
    DbgPrintEx(0, 0, "Unsupported IRP called.\0".as_ptr());
    STATUS_SUCCESS
}

// I/O Request Package Major function - device_control
pub unsafe extern "system" fn irp_mj_device_control(device: &mut DEVICE_OBJECT, irp: &mut IRP) -> NTSTATUS {
    let mut status: NTSTATUS = STATUS_SUCCESS;
    DbgPrintEx(0, 0, "Device control IRP called.\0".as_ptr());

    let io_stack_location_ptr: PIO_STACK_LOCATION = IoGetCurrentIrpStackLocation(irp as PIRP);
    let io_control_code: ULONG = (*io_stack_location_ptr).Parameters.DeviceIoControl().IoControlCode;
    DbgPrintEx(0, 0, "IRP_MJ_DEVICE_CONTROL>>IO control code: %u.\0".as_ptr(), io_control_code);
    DbgPrintEx(0, 0, "IRP_MJ_DEVICE_CONTROL>>MouseEvent control code: %u.\0".as_ptr(), META_IRP_MOUSE_EVENT);

    let mut bytes_io = 0;

    if io_control_code == META_IRP_MOUSE_EVENT {
        bytes_io = core::mem::size_of::<MouseRequest>();
        let mouse_request: &MouseRequest = &(*(*irp.AssociatedIrp.SystemBuffer() as PMouseRequest));
        /*let mut flags: USHORT = 0;
        flags |= MOUSE_MOVE_ABSOLUTE;
        mouse_event(GLOBAL_MOUSE_OBJ_PTR, mouse_request.x as _, mouse_request.y as _, 0x0000, flags);
        DbgPrintEx(0, 0, "IRP_MJ_DEVICE_CONTROL>>x: %u.\0".as_ptr(), mouse_request.x);*/
        let mut flags: USHORT = 0;
        flags |= MOUSE_MOVE_ABSOLUTE;
        mouse_event(&mut GLOBAL_MOUSE_OBJ as PMOUSE_OBJECT, mouse_request.x as _, mouse_request.y as _, 0x0000, flags);
        //DbgPrintEx(0, 0, "!MOUSE POINTER: %p\0".as_ptr(), GLOBAL_MOUSE_OBJ_PTR);

    }
    
    irp.IoStatus.Information = bytes_io;
    let io_status = irp.IoStatus.__bindgen_anon_1.Status_mut();
    *io_status = status;

    IoCompleteRequest(irp as PIRP, IO_PRIORITY::IO_NO_INCREMENT);

    status
}

// I/O Request Package Major function - create
pub unsafe extern "system" fn irp_mj_create(device: &mut DEVICE_OBJECT, irp: &mut IRP) -> NTSTATUS {
    DbgPrintEx(0, 0, "IRP_MJ_CREATE called %u.\0".as_ptr(), irp.Type as i32);
    IoCompleteRequest(irp as PIRP, IO_PRIORITY::IO_NO_INCREMENT);
    // Return NSTATUS
    let status = *irp.IoStatus.__bindgen_anon_1.Status();
    DbgPrintEx(0, 0, "IRP_MJ_CREATE>>IoCompleteRequest status: %u.\0".as_ptr(), status);
    status
}

// I/O Request Package Major function - close
pub unsafe extern "system" fn irp_mj_close(device: &mut DEVICE_OBJECT, irp: &mut IRP) -> NTSTATUS {
    DbgPrintEx(0, 0, "IRP_MJ_CLOSE called.\0".as_ptr());
    IoCompleteRequest(irp as PIRP, IO_PRIORITY::IO_NO_INCREMENT);
    // Return NSTATUS
    let status = *irp.IoStatus.__bindgen_anon_1.Status();
    DbgPrintEx(0, 0, "IRP_MJ_CLOSE>>IoCompleteRequest status: %u.\0".as_ptr(), status);
    status
}

// Unload
pub extern "system" fn driver_exit(driver: &mut DRIVER_OBJECT) {
    unsafe {

        // Delete device symlink
        let mut io_symlink_name_unicode: UNICODE_STRING = zeroed_unicode_string();
        RtlInitUnicodeString(&mut io_symlink_name_unicode as *mut UNICODE_STRING,
            IO_SYMLINK_NAME.as_ptr());
        IoDeleteSymbolicLink(&io_symlink_name_unicode);
        DbgPrintEx(0, 0, "Device symlink removed.\0".as_ptr());

        // Close IO device
        IoDeleteDevice(driver.DeviceObject);
        DbgPrintEx(0, 0, "Device removed.\0".as_ptr());

        // Bye-bye
        DbgPrintEx(0, 0, "meta_driver unloaded. Bye-Bye!.\0".as_ptr());
    }
}

// Temporary __CxxFrameHandler3 issue fix
#[no_mangle]
pub extern "system" fn __CxxFrameHandler3() -> i32 {
    0
}
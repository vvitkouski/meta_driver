// Imports
use wchar::{wchz, wchar_t};
use winapi::shared::ntdef::{
	NTSTATUS,
	UNICODE_STRING,
	LONG,
	OBJ_CASE_INSENSITIVE,
	PVOID,
	KIRQL,
	PKIRQL
};
pub use winapi::shared::minwindef::{
	ULONG,
	PULONG,
	USHORT,
	PUCHAR
};
use winapi::km::wdm::{
	DbgPrintEx,
	DEVICE_OBJECT,
	PDEVICE_OBJECT,
	PDRIVER_OBJECT,
	KPROCESSOR_MODE
};
use winapi::shared::ntstatus::{
	STATUS_SUCCESS,
	STATUS_FAIL_CHECK
};
use crate::winapi_local::km::wdm::{
    zeroed_unicode_string,
    RtlInitUnicodeString,
    ObReferenceObjectByName,
    ObDereferenceObject,
    IoDriverObjectType,
    KeRaiseIrql,
    KeLowerIrql,
    ULONG_PTR,
    PULONG_PTR,
    DISPATCH_LEVEL,
    PASSIVE_LEVEL
};

// DATA TYPES, CONSTANTS, STRUCTS etc... ================================================

// Keyboard input data bind
#[allow(non_snake_case)]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct KEYBOARD_INPUT_DATA {
	pub UnitId: USHORT,
	pub MakeCode: USHORT,
	pub Flags: USHORT,
	pub Reserved: USHORT,
	pub ExtraInformation: ULONG
}
#[allow(non_camel_case_types)]
pub type PKEYBOARD_INPUT_DATA = *mut KEYBOARD_INPUT_DATA;

#[allow(non_camel_case_types)]
pub type KEYBOARD_EXTERN_CALLBACK = unsafe extern "C" fn(
	DeviceObject: PDEVICE_OBJECT,
	InputDataStart: PKEYBOARD_INPUT_DATA,
	InputDataEnd: PKEYBOARD_INPUT_DATA,
	InputDataConsumed: PULONG
) -> NTSTATUS;
pub type KeyboardClassServiceCallback = Option<KEYBOARD_EXTERN_CALLBACK>;
#[allow(non_camel_case_types)]
pub struct KBD_OBJECT {
	pub kbd_device: PDEVICE_OBJECT,
	pub service_callback: KeyboardClassServiceCallback
}
#[allow(non_camel_case_types)]
pub type PKBD_OBJECT = *mut KBD_OBJECT;

// Constants
pub const KBD_DRIVER_CLASS_NAME: &[wchar_t] = wchz!("\\Driver\\KbdClass");
// pub const KBD_DRIVER_HID_NAME: &[wchar_t] = wchz!("\\Driver\\KbdHID");
pub const KBD_DRIVER_HID_NAME: &[wchar_t] = wchz!("\\Driver\\ASWkbd");


// PUBLIC FUNCTIONS ==========================================

pub const fn zeroed_kbd_object() -> KBD_OBJECT {
	KBD_OBJECT {
		kbd_device : core::ptr::null_mut(),
		service_callback: None
	}
}

pub fn kbd_init(kbd_object: PKBD_OBJECT) -> NTSTATUS {
	unsafe {
		DbgPrintEx(0, 0, "kbd_init called.\0".as_ptr());

		// FIND KBD CLASS DRIVER
		let mut kbd_driver_class_unicode: UNICODE_STRING = zeroed_unicode_string();
		let mut class_driver_obj: PDRIVER_OBJECT = core::ptr::null_mut();
		RtlInitUnicodeString(&mut kbd_driver_class_unicode as *mut UNICODE_STRING,
            KBD_DRIVER_CLASS_NAME.as_ptr());
		let kbd_driver_find_status = ObReferenceObjectByName(
			&mut kbd_driver_class_unicode as *mut UNICODE_STRING,
			OBJ_CASE_INSENSITIVE,
			core::ptr::null_mut(),
			0,
			*IoDriverObjectType,
			KPROCESSOR_MODE::KernelMode,
			core::ptr::null_mut(),
			&mut class_driver_obj as *mut _ as *mut PVOID
		);
		DbgPrintEx(0, 0, "ObReferenceObjectByName>>KbdClass status: %u\0".as_ptr(), kbd_driver_find_status);
		DbgPrintEx(0, 0, "Kbd class object pointer: %p\0".as_ptr(), class_driver_obj);
		if kbd_driver_find_status != STATUS_SUCCESS {
			return kbd_driver_find_status;
		}
		let class_driver_obj_uptr: ULONG_PTR = class_driver_obj as ULONG_PTR;

		// FIND KBD HID DRIVER
		let mut kdb_driver_hid_unicode: UNICODE_STRING = zeroed_unicode_string();
		let mut hid_driver_obj: PDRIVER_OBJECT = core::ptr::null_mut();
		RtlInitUnicodeString(&mut kdb_driver_hid_unicode as *mut UNICODE_STRING,
            KBD_DRIVER_HID_NAME.as_ptr());
		let hid_driver_find_status = ObReferenceObjectByName(
			&mut kdb_driver_hid_unicode as *mut UNICODE_STRING,
			OBJ_CASE_INSENSITIVE,
			core::ptr::null_mut(),
			0,
			*IoDriverObjectType,
			KPROCESSOR_MODE::KernelMode,
			core::ptr::null_mut(),
			&mut hid_driver_obj as *mut _ as *mut PVOID
		);
		DbgPrintEx(0, 0, "ObReferenceObjectByName>>KbdHID status: %u\0".as_ptr(), hid_driver_find_status);
		DbgPrintEx(0, 0, "Kbd Hid object pointer: %p\0".as_ptr(), hid_driver_obj);
		if hid_driver_find_status != STATUS_SUCCESS {
			ObDereferenceObject(class_driver_obj as PVOID);
			return hid_driver_find_status;
		}

		// LOOKUP
		let mut class_driver_base: PVOID  = core::ptr::null_mut();
		let mut hid_device_obj: PDEVICE_OBJECT = (*hid_driver_obj).DeviceObject;

		while !hid_device_obj.is_null() && (*kbd_object).service_callback.is_none() {
			let mut class_device_obj: PDEVICE_OBJECT = (*class_driver_obj).DeviceObject;
			
			while !class_device_obj.is_null() && (*kbd_object).service_callback.is_none() {
				if (*class_device_obj).NextDevice.is_null() && (*kbd_object).kbd_device.is_null() {
					(*kbd_object).kbd_device = class_device_obj;
				}
				if (*hid_device_obj).DeviceObjectExtension.is_null() {
					DbgPrintEx(0, 0, "Kbd NULL DEVICE EXTENSION\0".as_ptr());
					continue;
				}

				let device_extension: ULONG_PTR = (*hid_device_obj).DeviceExtension as ULONG_PTR;
				let device_extension_ptr: PULONG_PTR = (*hid_device_obj).DeviceExtension as PULONG_PTR;
				let mut device_extension_size: ULONG_PTR = (*hid_device_obj).DeviceObjectExtension as ULONG_PTR;
				device_extension_size = (device_extension_size - device_extension) / 4;
				DbgPrintEx(0, 0, "Kbd Device extension size: %u\0".as_ptr(), device_extension_size);
				
				class_driver_base = (*class_driver_obj).DriverStart as PVOID;
				let class_device_obj_uptr: ULONG_PTR = class_device_obj as ULONG_PTR;
				
				for ext_idx in 0..(device_extension_size - 1) {
					let ext_idx_i = ext_idx as isize;
					if *device_extension_ptr.offset(ext_idx_i) == class_device_obj_uptr
						&& *device_extension_ptr.offset(ext_idx_i + 1) > class_driver_obj_uptr {
							(*kbd_object).service_callback = Some(core::mem::transmute(*device_extension_ptr.offset(ext_idx_i + 1)));
							DbgPrintEx(0, 0, "KBD Service callback address: %p\0".as_ptr(), (*kbd_object).service_callback.unwrap());
							break;
					}
				}
				class_device_obj = (*class_device_obj).NextDevice;
			}
			
			hid_device_obj = (*hid_device_obj).AttachedDevice;
			break;
		}

		// Find mouse device if not already
		if (*kbd_object).kbd_device.is_null() {
			let mut target_device_obj: PDEVICE_OBJECT = (*class_driver_obj).DeviceObject;
			while !target_device_obj.is_null() {
				if (*target_device_obj).NextDevice.is_null() {
					(*kbd_object).kbd_device = target_device_obj;
					break;
				}
				target_device_obj = (*target_device_obj).NextDevice;
			}
		}


		// Dereference objects
		ObDereferenceObject(class_driver_obj as PVOID);
		ObDereferenceObject(hid_driver_obj as PVOID);

	}
	STATUS_SUCCESS
}
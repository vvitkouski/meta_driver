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
	USHORT
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

// Constants
pub const MOUSE_DRIVER_CLASS_NAME: &[wchar_t] = wchz!("\\Driver\\MouClass");
pub const MOUSE_DRIVER_HID_NAME: &[wchar_t] = wchz!("\\Driver\\MouHID");

pub const MOUSE_LEFT_BUTTON_DOWN: USHORT = 0x0001;  // Left Button changed to down.
pub const MOUSE_LEFT_BUTTON_UP: USHORT = 0x0002;  // Left Button changed to up.
pub const MOUSE_RIGHT_BUTTON_DOWN: USHORT = 0x0004;  // Right Button changed to down.
pub const MOUSE_RIGHT_BUTTON_UP: USHORT = 0x0008;  // Right Button changed to up.
pub const MOUSE_MIDDLE_BUTTON_DOWN: USHORT = 0x0010;  // Middle Button changed to down.
pub const MOUSE_MIDDLE_BUTTON_UP: USHORT = 0x0020;  // Middle Button changed to up.

pub const MOUSE_BUTTON_1_DOWN: USHORT = MOUSE_LEFT_BUTTON_DOWN;
pub const MOUSE_BUTTON_1_UP: USHORT = MOUSE_LEFT_BUTTON_UP;
pub const MOUSE_BUTTON_2_DOWN: USHORT = MOUSE_RIGHT_BUTTON_DOWN;
pub const MOUSE_BUTTON_2_UP: USHORT = MOUSE_RIGHT_BUTTON_UP;
pub const MOUSE_BUTTON_3_DOWN: USHORT = MOUSE_MIDDLE_BUTTON_DOWN;
pub const MOUSE_BUTTON_3_UP: USHORT = MOUSE_MIDDLE_BUTTON_UP;

pub const MOUSE_BUTTON_4_DOWN: USHORT = 0x0040;
pub const MOUSE_BUTTON_4_UP: USHORT = 0x0080;
pub const MOUSE_BUTTON_5_DOWN: USHORT = 0x0100;
pub const MOUSE_BUTTON_5_UP: USHORT = 0x0200;

pub const MOUSE_WHEEL: USHORT = 0x0400;
pub const MOUSE_HWHEEL: USHORT = 0x0800;

pub const MOUSE_MOVE_RELATIVE: USHORT = 0;
pub const MOUSE_MOVE_ABSOLUTE: USHORT = 1;
pub const MOUSE_VIRTUAL_DESKTOP: USHORT = 0x02;  // the coordinates are mapped to the virtual desktop
pub const MOUSE_ATTRIBUTES_CHANGED: USHORT = 0x04;  // requery for mouse attributes
pub const MOUSE_MOVE_NOCOALESCE: USHORT = 0x08;  // do not coalesce WM_MOUSEMOVEs

pub const MOUSE_TERMSRV_SRC_SHADOW: USHORT = 0x100;

// Mouse input data bind
#[allow(non_snake_case)]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct MOUSE_INPUT_DATA {
	pub UnitId: USHORT,
	pub Flags: USHORT,
	pub ButtonFlags: USHORT,
	pub ButtonData: USHORT,
	pub RawButtons: ULONG,
	pub LastX: LONG,
	pub LastY: LONG,
	pub ExtraInformation: ULONG
}
#[allow(non_camel_case_types)]
pub type PMOUSE_INPUT_DATA = *mut MOUSE_INPUT_DATA;

// Mouse object structure
#[allow(non_camel_case_types)]
pub type MOUSE_EXTERN_CALLBACK = unsafe extern "C" fn(
	DeviceObject: PDEVICE_OBJECT,
	InputDataStart: PMOUSE_INPUT_DATA,
	InputDataEnd: PMOUSE_INPUT_DATA,
	InputDataConsumed: PULONG
) -> NTSTATUS;
pub type MouseClassServiceCallback = Option<MOUSE_EXTERN_CALLBACK>;
#[allow(non_camel_case_types)]
pub struct MOUSE_OBJECT {
	pub mouse_device: PDEVICE_OBJECT,
	pub service_callback: MouseClassServiceCallback
}
#[allow(non_camel_case_types)]
pub type PMOUSE_OBJECT = *mut MOUSE_OBJECT;

// PUBLIC FUNCTIONS ==========================================

// Prepare empty mouse object
pub const fn zeroed_mouse_object() -> MOUSE_OBJECT {
	MOUSE_OBJECT {
		mouse_device : core::ptr::null_mut(),
		service_callback: None
	}
}

// Mouse init
pub fn mouse_init(mouse_object: PMOUSE_OBJECT) -> NTSTATUS {
	unsafe {
		DbgPrintEx(0, 0, "mouse_init called.\0".as_ptr());

		// FIND MOU CLASS DRIVER
		let mut mouse_driver_class_unicode: UNICODE_STRING = zeroed_unicode_string();
		let mut class_driver_obj: PDRIVER_OBJECT = core::ptr::null_mut();
		RtlInitUnicodeString(&mut mouse_driver_class_unicode as *mut UNICODE_STRING,
            MOUSE_DRIVER_CLASS_NAME.as_ptr());
		let mou_driver_find_status = ObReferenceObjectByName(
			&mut mouse_driver_class_unicode as *mut UNICODE_STRING,
			OBJ_CASE_INSENSITIVE,
			core::ptr::null_mut(),
			0,
			*IoDriverObjectType,
			KPROCESSOR_MODE::KernelMode,
			core::ptr::null_mut(),
			&mut class_driver_obj as *mut _ as *mut PVOID
		);
		DbgPrintEx(0, 0, "ObReferenceObjectByName>>MouClass status: %u\0".as_ptr(), mou_driver_find_status);
		DbgPrintEx(0, 0, "Mouse class object pointer: %p\0".as_ptr(), class_driver_obj);
		if mou_driver_find_status != STATUS_SUCCESS {
			return mou_driver_find_status;
		}
		let class_driver_obj_uptr: ULONG_PTR = class_driver_obj as ULONG_PTR;

		// FIND MOU HID DRIVER
		let mut mouse_driver_hid_unicode: UNICODE_STRING = zeroed_unicode_string();
		let mut hid_driver_obj: PDRIVER_OBJECT = core::ptr::null_mut();
		RtlInitUnicodeString(&mut mouse_driver_hid_unicode as *mut UNICODE_STRING,
            MOUSE_DRIVER_HID_NAME.as_ptr());
		let hid_driver_find_status = ObReferenceObjectByName(
			&mut mouse_driver_hid_unicode as *mut UNICODE_STRING,
			OBJ_CASE_INSENSITIVE,
			core::ptr::null_mut(),
			0,
			*IoDriverObjectType,
			KPROCESSOR_MODE::KernelMode,
			core::ptr::null_mut(),
			&mut hid_driver_obj as *mut _ as *mut PVOID
		);
		DbgPrintEx(0, 0, "ObReferenceObjectByName>>MouHID status: %u\0".as_ptr(), hid_driver_find_status);
		DbgPrintEx(0, 0, "Hid object pointer: %p\0".as_ptr(), hid_driver_obj);
		if hid_driver_find_status != STATUS_SUCCESS {
			ObDereferenceObject(class_driver_obj as PVOID);
			return hid_driver_find_status;
		}

		// Lookup
		let mut class_driver_base: PVOID  = core::ptr::null_mut();
		let mut hid_device_obj: PDEVICE_OBJECT = (*hid_driver_obj).DeviceObject;
		
		while !hid_device_obj.is_null() && (*mouse_object).service_callback.is_none() {
			let mut class_device_obj: PDEVICE_OBJECT = (*class_driver_obj).DeviceObject;
			
			while !class_device_obj.is_null() && (*mouse_object).service_callback.is_none() {
				if (*class_device_obj).NextDevice.is_null() && (*mouse_object).mouse_device.is_null() {
					(*mouse_object).mouse_device = class_device_obj;
				}
				if (*hid_device_obj).DeviceObjectExtension.is_null() {
					DbgPrintEx(0, 0, "NULL DEVICE EXTENSION\0".as_ptr());
					continue;
				}

				let device_extension: ULONG_PTR = (*hid_device_obj).DeviceExtension as ULONG_PTR;
				let device_extension_ptr: PULONG_PTR = (*hid_device_obj).DeviceExtension as PULONG_PTR;
				let mut device_extension_size: ULONG_PTR = (*hid_device_obj).DeviceObjectExtension as ULONG_PTR;
				device_extension_size = (device_extension_size - device_extension) / 4;
				DbgPrintEx(0, 0, "Device extension size: %u\0".as_ptr(), device_extension_size);
				
				class_driver_base = (*class_driver_obj).DriverStart as PVOID;
				let class_device_obj_uptr: ULONG_PTR = class_device_obj as ULONG_PTR;
				
				for ext_idx in 0..(device_extension_size - 1) {
					let ext_idx_i = ext_idx as isize;
					if *device_extension_ptr.offset(ext_idx_i) == class_device_obj_uptr
						&& *device_extension_ptr.offset(ext_idx_i + 1) > class_driver_obj_uptr {
							(*mouse_object).service_callback = Some(core::mem::transmute(*device_extension_ptr.offset(ext_idx_i + 1)));
							DbgPrintEx(0, 0, "Service callback address: %p\0".as_ptr(), (*mouse_object).service_callback.unwrap());
							break;
					}
				}
				class_device_obj = (*class_device_obj).NextDevice;
			}
			
			hid_device_obj = (*hid_device_obj).AttachedDevice;
			break;
		}

		// Find mouse device if not already
		if (*mouse_object).mouse_device.is_null() {
			let mut target_device_obj: PDEVICE_OBJECT = (*class_driver_obj).DeviceObject;
			while !target_device_obj.is_null() {
				if (*target_device_obj).NextDevice.is_null() {
					(*mouse_object).mouse_device = target_device_obj;
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


// Mouse event
pub unsafe fn mouse_event(mouse_object: PMOUSE_OBJECT, x: LONG, y: LONG, button_flags: USHORT, flags: USHORT) -> NTSTATUS {
	if (*mouse_object).service_callback.is_some() {
		let mut input_data: ULONG = 0u32;
		let mut origin_irql: KIRQL = PASSIVE_LEVEL;
		let mut origin_irql_ptr = &mut origin_irql as PKIRQL; 
		let new_irql: KIRQL = DISPATCH_LEVEL;
		let mut mouse_input_data = MOUSE_INPUT_DATA {
			UnitId: 0,
			Flags: flags,
			ButtonFlags: button_flags,
			ButtonData: 0,
			RawButtons: 0,
			LastX: x,
			LastY: y,
			ExtraInformation: 0
		};
		let mouse_input_data_ptr = &mut mouse_input_data as PMOUSE_INPUT_DATA;

		DbgPrintEx(0, 0, "mouse_event: NEW LEVEL %u\0".as_ptr(), new_irql as ULONG);
		DbgPrintEx(0, 0, "mouse_event: CALLBACK ADDR %p\0".as_ptr(), (*mouse_object).service_callback.unwrap());
		KeRaiseIrql(new_irql, origin_irql_ptr);
		((*mouse_object).service_callback.unwrap())(
			(*mouse_object).mouse_device,
			mouse_input_data_ptr,
			mouse_input_data_ptr.offset(1),
			&mut input_data
		);
		KeLowerIrql(PASSIVE_LEVEL);
		DbgPrintEx(0, 0, "mouse_event: service callback called with x: %i y: %i button_flags: %i\0".as_ptr(), x, y, button_flags as ULONG);
		DbgPrintEx(0, 0, "mouse_event: OLD LEVEL %u\0".as_ptr(), *origin_irql_ptr as ULONG);
		STATUS_SUCCESS
	} else {
		DbgPrintEx(0, 0, "mouse_event: service callback not defined\0".as_ptr());
		STATUS_FAIL_CHECK
	}
}
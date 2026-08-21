use std::ffi::{c_char, c_void};

#[repr(C)]
pub struct xcw_native_owned_bytes {
    pub data: *mut u8,
    pub length: usize,
}

unsafe extern "C" {

    pub fn xcw_native_initialize_app();

    pub fn xcw_native_list_simulators(error_message: *mut *mut c_char) -> *mut c_char;
    pub fn xcw_native_screenshot_png(
        udid: *const c_char,
        error_message: *mut *mut c_char,
    ) -> xcw_native_owned_bytes;
    pub fn xcw_native_accessibility_snapshot(
        udid: *const c_char,
        has_point: bool,
        x: f64,
        y: f64,
        max_depth: usize,
        interactive_only: bool,
        error_message: *mut *mut c_char,
    ) -> *mut c_char;
    pub fn xcw_native_display_size(
        udid: *const c_char,
        width_points: *mut f64,
        height_points: *mut f64,
        scale: *mut f64,
        error_message: *mut *mut c_char,
    ) -> bool;
    pub fn xcw_native_send_key(
        udid: *const c_char,
        key_code: u16,
        modifiers: u32,
        error_message: *mut *mut c_char,
    ) -> bool;
    pub fn xcw_native_press_button(
        udid: *const c_char,
        button_name: *const c_char,
        duration_ms: u32,
        error_message: *mut *mut c_char,
    ) -> bool;

    pub fn xcw_native_input_create(
        udid: *const c_char,
        error_message: *mut *mut c_char,
    ) -> *mut c_void;
    pub fn xcw_native_input_destroy(handle: *mut c_void);
    pub fn xcw_native_input_send_touch(
        handle: *mut c_void,
        x: f64,
        y: f64,
        phase: *const c_char,
        error_message: *mut *mut c_char,
    ) -> bool;

    pub fn xcw_native_free_string(value: *mut c_char);
    pub fn xcw_native_free_bytes(bytes: xcw_native_owned_bytes);
}

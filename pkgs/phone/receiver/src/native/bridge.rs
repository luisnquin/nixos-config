use crate::error::AppError;
use crate::native::ffi;
use serde::de::Error as DeError;
use serde::{Deserialize, Serialize};
use std::ffi::{c_void, CStr, CString};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

const RECOVERABLE_RESTART_EXIT_CODE: i32 = 75;
const RESTART_ON_CORE_SIMULATOR_MISMATCH_ENV: &str = "SICKDECK_RESTART_ON_CORE_SIMULATOR_MISMATCH";
const ACCESSIBILITY_POINT_SNAPSHOT_MAX_ATTEMPTS: usize = 1;
const ACCESSIBILITY_SNAPSHOT_MAX_ATTEMPTS: usize = 4;
const ACCESSIBILITY_SNAPSHOT_RETRY_DELAY_MS: u64 = 100;

static RECOVERABLE_RESTART_SCHEDULED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DisplaySize {
    pub width: f64,
    pub height: f64,
    pub scale: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Simulator {
    pub udid: String,
    pub name: String,
    pub state: String,
    #[serde(rename = "isBooted")]
    #[serde(deserialize_with = "deserialize_boolish")]
    pub is_booted: bool,
    #[serde(rename = "isAvailable")]
    #[serde(deserialize_with = "deserialize_boolish")]
    pub is_available: bool,
    #[serde(rename = "lastBootedAt")]
    pub last_booted_at: serde_json::Value,
    #[serde(rename = "dataPath")]
    pub data_path: serde_json::Value,
    #[serde(rename = "logPath")]
    pub log_path: serde_json::Value,
    #[serde(rename = "deviceTypeIdentifier")]
    pub device_type_identifier: serde_json::Value,
    #[serde(rename = "deviceTypeName")]
    pub device_type_name: String,
    #[serde(rename = "runtimeIdentifier")]
    pub runtime_identifier: serde_json::Value,
    #[serde(rename = "runtimeName")]
    pub runtime_name: String,
    #[serde(
        rename = "pairedWatchUDID",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub paired_watch_udid: Option<String>,
    #[serde(
        rename = "pairedWatchName",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub paired_watch_name: Option<String>,
    #[serde(
        rename = "pairedPhoneUDID",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub paired_phone_udid: Option<String>,
    #[serde(
        rename = "pairedPhoneName",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub paired_phone_name: Option<String>,
    #[serde(
        rename = "devicePairIdentifier",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub device_pair_identifier: Option<String>,
    #[serde(
        rename = "devicePairState",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub device_pair_state: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SimulatorsEnvelope {
    simulators: Vec<Simulator>,
}

fn deserialize_boolish<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Bool(value) => Ok(value),
        serde_json::Value::Number(value) => match value.as_i64() {
            Some(0) => Ok(false),
            Some(1) => Ok(true),
            _ => Err(D::Error::custom("expected 0 or 1 for boolean field")),
        },
        serde_json::Value::String(value) => match value.as_str() {
            "0" | "false" | "False" | "FALSE" => Ok(false),
            "1" | "true" | "True" | "TRUE" => Ok(true),
            _ => Err(D::Error::custom("expected boolean-like string")),
        },
        _ => Err(D::Error::custom("expected boolean-compatible value")),
    }
}

#[derive(Default, Clone)]
pub struct NativeBridge;

impl NativeBridge {
    pub fn list_simulators(&self) -> Result<Vec<Simulator>, AppError> {
        let json = unsafe {
            let mut error = ptr::null_mut();
            let raw = ffi::xcw_native_list_simulators(&mut error);
            string_from_raw(raw, error)?
        };
        let payload: SimulatorsEnvelope =
            serde_json::from_str(&json).map_err(|e| AppError::internal(e.to_string()))?;
        Ok(payload.simulators)
    }

    pub fn screenshot_png(&self, udid: &str) -> Result<Vec<u8>, AppError> {
        let udid = CString::new(udid).map_err(|e| AppError::bad_request(e.to_string()))?;
        unsafe {
            let mut error = ptr::null_mut();
            let bytes = ffi::xcw_native_screenshot_png(udid.as_ptr(), &mut error);
            if bytes.data.is_null() {
                return Err(
                    take_error(error).unwrap_or_else(|| AppError::native("Unknown native error."))
                );
            }
            let data = std::slice::from_raw_parts(bytes.data, bytes.length).to_vec();
            ffi::xcw_native_free_bytes(bytes);
            Ok(data)
        }
    }

    pub fn accessibility_snapshot(
        &self,
        udid: &str,
        point: Option<(f64, f64)>,
    ) -> Result<serde_json::Value, AppError> {
        self.accessibility_snapshot_with_max_depth(udid, point, None)
    }

    pub fn accessibility_snapshot_with_max_depth(
        &self,
        udid: &str,
        point: Option<(f64, f64)>,
        max_depth: Option<usize>,
    ) -> Result<serde_json::Value, AppError> {
        self.accessibility_snapshot_with_options(udid, point, max_depth, false)
    }

    pub fn accessibility_snapshot_with_options(
        &self,
        udid: &str,
        point: Option<(f64, f64)>,
        max_depth: Option<usize>,
        interactive_only: bool,
    ) -> Result<serde_json::Value, AppError> {
        let udid = CString::new(udid).map_err(|e| AppError::bad_request(e.to_string()))?;
        let max_depth = max_depth.unwrap_or(80).min(80);
        let max_attempts = if point.is_some() || max_depth == 0 {
            ACCESSIBILITY_POINT_SNAPSHOT_MAX_ATTEMPTS
        } else {
            ACCESSIBILITY_SNAPSHOT_MAX_ATTEMPTS
        };
        for attempt in 1..=max_attempts {
            let json =
                match native_accessibility_snapshot_json(&udid, point, max_depth, interactive_only)
                {
                    Ok(json) => json,
                    Err(error) if is_core_simulator_service_mismatch(&error.to_string()) => {
                        std::thread::sleep(Duration::from_millis(250));
                        native_accessibility_snapshot_json(
                            &udid,
                            point,
                            max_depth,
                            interactive_only,
                        )?
                    }
                    Err(error) => return Err(error),
                };
            let snapshot: serde_json::Value =
                serde_json::from_str(&json).map_err(|e| AppError::internal(e.to_string()))?;
            if !accessibility_snapshot_is_transient_empty(&snapshot) || attempt == max_attempts {
                return Ok(snapshot);
            }
            std::thread::sleep(Duration::from_millis(ACCESSIBILITY_SNAPSHOT_RETRY_DELAY_MS));
        }
        unreachable!("accessibility snapshot retry loop always returns")
    }

    /// The panel in points, and the scale that maps them to pixels. Touches are
    /// normalized rather than absolute, so this is what turns a frame read off
    /// the accessibility tree into somewhere to press.
    pub fn display_size(&self, udid: &str) -> Result<DisplaySize, AppError> {
        let udid = CString::new(udid).map_err(|e| AppError::bad_request(e.to_string()))?;
        unsafe {
            let mut error = ptr::null_mut();
            let mut width = 0.0;
            let mut height = 0.0;
            let mut scale = 0.0;
            bool_result(
                ffi::xcw_native_display_size(
                    udid.as_ptr(),
                    &mut width,
                    &mut height,
                    &mut scale,
                    &mut error,
                ),
                error,
            )?;
            Ok(DisplaySize {
                width,
                height,
                scale,
            })
        }
    }

    pub fn send_key(&self, udid: &str, key_code: u16, modifiers: u32) -> Result<(), AppError> {
        let udid = CString::new(udid).map_err(|e| AppError::bad_request(e.to_string()))?;
        unsafe {
            let mut error = ptr::null_mut();
            bool_result(
                ffi::xcw_native_send_key(udid.as_ptr(), key_code, modifiers, &mut error),
                error,
            )
        }
    }

    pub fn press_button(&self, udid: &str, button: &str, duration_ms: u32) -> Result<(), AppError> {
        let udid = CString::new(udid).map_err(|e| AppError::bad_request(e.to_string()))?;
        let button = CString::new(button).map_err(|e| AppError::bad_request(e.to_string()))?;
        unsafe {
            let mut error = ptr::null_mut();
            bool_result(
                ffi::xcw_native_press_button(
                    udid.as_ptr(),
                    button.as_ptr(),
                    duration_ms,
                    &mut error,
                ),
                error,
            )
        }
    }

    pub fn create_input_session(&self, udid: &str) -> Result<NativeInputSession, AppError> {
        let udid = CString::new(udid).map_err(|e| AppError::bad_request(e.to_string()))?;
        unsafe {
            let mut error = ptr::null_mut();
            let handle = ffi::xcw_native_input_create(udid.as_ptr(), &mut error);
            if handle.is_null() {
                return Err(take_error(error).unwrap_or_else(|| {
                    AppError::native("Unable to create native input session.")
                }));
            }
            Ok(NativeInputSession { handle })
        }
    }
}

pub struct NativeInputSession {
    handle: *mut c_void,
}

unsafe impl Send for NativeInputSession {}
unsafe impl Sync for NativeInputSession {}

impl NativeInputSession {
    pub fn send_touch(&self, x: f64, y: f64, phase: &str) -> Result<(), AppError> {
        let phase = CString::new(phase).map_err(|e| AppError::bad_request(e.to_string()))?;
        unsafe {
            let mut error = ptr::null_mut();
            bool_result(
                ffi::xcw_native_input_send_touch(self.handle, x, y, phase.as_ptr(), &mut error),
                error,
            )
        }
    }
}

impl Drop for NativeInputSession {
    fn drop(&mut self) {
        unsafe {
            ffi::xcw_native_input_destroy(self.handle);
        }
    }
}

fn native_accessibility_snapshot_json(
    udid: &CString,
    point: Option<(f64, f64)>,
    max_depth: usize,
    interactive_only: bool,
) -> Result<String, AppError> {
    unsafe {
        let mut error = ptr::null_mut();
        let (has_point, x, y) = point
            .map(|(x, y)| (true, x, y))
            .unwrap_or((false, 0.0, 0.0));
        let raw = ffi::xcw_native_accessibility_snapshot(
            udid.as_ptr(),
            has_point,
            x,
            y,
            max_depth,
            interactive_only,
            &mut error,
        );
        string_from_raw(raw, error)
    }
}

fn is_core_simulator_service_mismatch(message: &str) -> bool {
    message.contains("CoreSimulator.framework was changed while the process was running")
        || message.contains("Service version")
            && message.contains("does not match expected service version")
}

fn accessibility_snapshot_is_transient_empty(snapshot: &serde_json::Value) -> bool {
    let Some(roots) = snapshot.get("roots").and_then(serde_json::Value::as_array) else {
        return true;
    };
    roots.is_empty() || roots.iter().all(node_is_zero_sized_leaf)
}

fn node_is_zero_sized_leaf(node: &serde_json::Value) -> bool {
    let has_children = node
        .get("children")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|children| !children.is_empty());
    !has_children && node_frame_is_empty(node)
}

fn node_frame_is_empty(node: &serde_json::Value) -> bool {
    let Some(frame) = node
        .get("frame")
        .or_else(|| node.get("frameInScreen"))
        .or_else(|| node.get("bounds"))
    else {
        return true;
    };
    let width = frame
        .get("width")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let height = frame
        .get("height")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    width <= 0.0 || height <= 0.0
}

unsafe fn string_from_raw(raw: *mut i8, error: *mut i8) -> Result<String, AppError> {
    if raw.is_null() {
        return Err(take_error(error).unwrap_or_else(|| AppError::native("Unknown native error.")));
    }
    let value = CStr::from_ptr(raw).to_string_lossy().into_owned();
    ffi::xcw_native_free_string(raw);
    Ok(value)
}

unsafe fn bool_result(result: bool, error: *mut i8) -> Result<(), AppError> {
    if result {
        Ok(())
    } else {
        Err(take_error(error).unwrap_or_else(|| AppError::native("Unknown native error.")))
    }
}

unsafe fn take_error(raw: *mut i8) -> Option<AppError> {
    if raw.is_null() {
        return None;
    }
    let message = CStr::from_ptr(raw).to_string_lossy().into_owned();
    ffi::xcw_native_free_string(raw);
    schedule_recoverable_restart_if_needed(&message);
    Some(AppError::native(message))
}

fn schedule_recoverable_restart_if_needed(message: &str) {
    if std::env::var_os(RESTART_ON_CORE_SIMULATOR_MISMATCH_ENV).is_none()
        || !is_core_simulator_service_mismatch(message)
        || RECOVERABLE_RESTART_SCHEDULED.swap(true, Ordering::SeqCst)
    {
        return;
    }

    eprintln!("CoreSimulator service mismatch detected; restarting sickdeck server process.");
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(100));
        std::process::exit(RECOVERABLE_RESTART_EXIT_CODE);
    });
}

#[cfg(test)]
mod tests {
    use super::{
        accessibility_snapshot_is_transient_empty, is_core_simulator_service_mismatch, Simulator,
    };
    use serde_json::json;

    fn simulator_json(is_booted: serde_json::Value, is_available: serde_json::Value) -> String {
        json!({
            "udid": "SIM-1",
            "name": "iPhone Test",
            "state": "Booted",
            "isBooted": is_booted,
            "isAvailable": is_available,
            "lastBootedAt": null,
            "dataPath": null,
            "logPath": null,
            "deviceTypeIdentifier": null,
            "deviceTypeName": "iPhone",
            "runtimeIdentifier": null,
            "runtimeName": "iOS"
        })
        .to_string()
    }

    #[test]
    fn simulator_boolish_fields_accept_native_json_variants() {
        let true_bool: Simulator =
            serde_json::from_str(&simulator_json(json!(true), json!(false))).unwrap();
        let numeric: Simulator = serde_json::from_str(&simulator_json(json!(1), json!(0))).unwrap();
        let string: Simulator =
            serde_json::from_str(&simulator_json(json!("TRUE"), json!("false"))).unwrap();

        assert!(true_bool.is_booted);
        assert!(!true_bool.is_available);
        assert!(numeric.is_booted);
        assert!(!numeric.is_available);
        assert!(string.is_booted);
        assert!(!string.is_available);
    }

    #[test]
    fn simulator_boolish_fields_reject_ambiguous_values() {
        let result = serde_json::from_str::<Simulator>(&simulator_json(json!(2), json!(true)));

        assert!(result.is_err());
    }

    #[test]
    fn core_simulator_mismatch_detection_covers_known_failure_strings() {
        assert!(is_core_simulator_service_mismatch(
            "CoreSimulator.framework was changed while the process was running"
        ));
        assert!(is_core_simulator_service_mismatch(
            "Service version 987 does not match expected service version 654"
        ));
        assert!(!is_core_simulator_service_mismatch(
            "Unable to initialize the private simulator display bridge."
        ));
    }

    #[test]
    fn accessibility_snapshot_retry_detects_empty_native_ax_tree() {
        assert!(accessibility_snapshot_is_transient_empty(&json!({
            "source": "native-ax",
            "roots": []
        })));
        assert!(accessibility_snapshot_is_transient_empty(&json!({
            "source": "native-ax",
            "roots": [{
                "role": "Application",
                "frame": { "x": 0, "y": 0, "width": 0, "height": 0 },
                "children": []
            }]
        })));
    }

    #[test]
    fn accessibility_snapshot_retry_keeps_usable_native_ax_tree() {
        assert!(!accessibility_snapshot_is_transient_empty(&json!({
            "source": "native-ax",
            "roots": [{
                "role": "Application",
                "frame": { "x": 0, "y": 0, "width": 393, "height": 852 },
                "children": []
            }]
        })));
        assert!(!accessibility_snapshot_is_transient_empty(&json!({
            "source": "native-ax",
            "roots": [{
                "role": "Application",
                "frame": { "x": 0, "y": 0, "width": 0, "height": 0 },
                "children": [{
                    "role": "Button",
                    "label": "Continue",
                    "frame": { "x": 10, "y": 20, "width": 100, "height": 44 }
                }]
            }]
        })));
    }
}

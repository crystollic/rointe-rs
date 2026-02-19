use serde::{Deserialize, Serialize};

use super::enums::{DeviceMode, DeviceStatus};

/// Full device object as returned by `/devices/{id}.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RointeDevice {
    /// All mutable device state (temperatures, mode, schedule, etc.).
    pub data: DeviceData,
    /// Device serial number.
    pub serialnumber: Option<String>,
    /// Firmware information reported by the device.
    pub firmware: Option<FirmwareInfo>,
}

/// The `/data` sub-object containing all mutable device state.
///
/// This is both the read model (returned by `GET /devices/{id}.json`) and
/// the basis for write operations (`PATCH /devices/{id}/data.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceData {
    /// Human-readable device name as configured in the Rointe app.
    pub name: String,

    /// Physical device type (e.g. `"radiator"`, `"towel"`).
    ///
    /// Serialised as the JSON field `"type"`.
    #[serde(rename = "type")]
    pub device_type: String,

    /// Hardware generation: `"v1"` or `"v2"`. Absent on some older devices.
    pub product_version: Option<String>,

    /// Rated power output in watts.
    pub nominal_power: Option<u32>,

    /// Whether the device is powered on.
    pub power: bool,

    /// Current operating mode (`manual` or `auto`).
    pub mode: DeviceMode,

    /// Current active preset or status.
    pub status: DeviceStatus,

    /// Current target temperature in °C.
    pub temp: f64,

    /// Internally calculated temperature in °C (device-side computation).
    pub temp_calc: Option<f64>,

    /// Measured probe temperature in °C (actual room/surface temperature).
    pub temp_probe: Option<f64>,

    /// Comfort preset temperature in °C.
    pub comfort: f64,

    /// Eco (energy-saving) preset temperature in °C.
    pub eco: f64,

    /// Frost-protection temperature in °C.
    pub ice: f64,

    /// Whether frost-protection (ice) mode is currently active.
    pub ice_mode: bool,

    /// Weekly schedule: 7 strings (Monday–Sunday), each 24 characters.
    ///
    /// Each character represents one hour of the day:
    /// - `'C'` — comfort temperature
    /// - `'E'` — eco temperature
    /// - `'O'` or other — off
    pub schedule: Option<Vec<String>>,

    /// Current day index within the schedule (0 = Monday).
    pub schedule_day: Option<u8>,

    /// Current hour index within the schedule.
    pub schedule_hour: Option<u8>,

    /// v2 only: upper bound for user-adjustable temperature in °C.
    pub um_max_temp: Option<f64>,

    /// v2 only: lower bound for user-adjustable temperature in °C.
    pub um_min_temp: Option<f64>,

    /// v2 only: whether user-mode (custom temp bounds) is active.
    pub user_mode: Option<bool>,

    /// Epoch milliseconds of the last app-side update.
    ///
    /// **Must be included in every PATCH request** with the current
    /// timestamp (`chrono::Utc::now().timestamp_millis()`).
    pub last_sync_datetime_app: i64,

    /// Epoch milliseconds of the last device-side sync.
    pub last_sync_datetime_device: Option<i64>,
}

/// Firmware version information as reported by the device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareInfo {
    /// Firmware version string installed on the device (e.g. `"3.2.1"`).
    pub firmware_version_device: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_DEVICE_JSON: &str = r#"{
        "data": {
            "name": "Kitchen Radiator",
            "type": "radiator",
            "product_version": "v2",
            "nominal_power": 1500,
            "power": true,
            "mode": "manual",
            "status": "comfort",
            "temp": 21.5,
            "temp_calc": 21.3,
            "temp_probe": 21.2,
            "comfort": 21.0,
            "eco": 18.0,
            "ice": 8.0,
            "ice_mode": false,
            "schedule": [
                "CCCCCCCCEEEEEEEEEEEEEECC",
                "CCCCCCCCEEEEEEEEEEEEEECC",
                "CCCCCCCCEEEEEEEEEEEEEECC",
                "CCCCCCCCEEEEEEEEEEEEEECC",
                "CCCCCCCCEEEEEEEEEEEEEECC",
                "CCCCCCCCCCCCCCCCCCCCCCCC",
                "CCCCCCCCCCCCCCCCCCCCCCCC"
            ],
            "schedule_day": 0,
            "schedule_hour": 0,
            "um_max_temp": 30.0,
            "um_min_temp": 7.0,
            "user_mode": false,
            "last_sync_datetime_app": 1708360000000,
            "last_sync_datetime_device": 1708359000000
        },
        "serialnumber": "ROINTE12345",
        "firmware": {
            "firmware_version_device": "3.2.1"
        }
    }"#;

    #[test]
    fn test_deserialize_full_device() {
        let device: RointeDevice = serde_json::from_str(SAMPLE_DEVICE_JSON).unwrap();

        assert_eq!(device.data.name, "Kitchen Radiator");
        assert_eq!(device.data.device_type, "radiator");
        assert_eq!(device.data.temp, 21.5);
        assert_eq!(device.data.comfort, 21.0);
        assert_eq!(device.data.eco, 18.0);
        assert_eq!(device.data.ice, 8.0);
        assert!(device.data.power);
        assert!(!device.data.ice_mode);
        assert_eq!(device.data.mode, DeviceMode::Manual);
        assert_eq!(device.data.status, DeviceStatus::Comfort);

        let schedule = device.data.schedule.unwrap();
        assert_eq!(schedule.len(), 7);
        assert_eq!(schedule[0], "CCCCCCCCEEEEEEEEEEEEEECC");

        assert_eq!(device.serialnumber.as_deref(), Some("ROINTE12345"));
        let fw = device.firmware.unwrap();
        assert_eq!(fw.firmware_version_device.as_deref(), Some("3.2.1"));
    }

    #[test]
    fn test_deserialize_device_status_none() {
        let json = r#"{
            "data": {
                "name": "Bedroom",
                "type": "radiator",
                "power": true,
                "mode": "manual",
                "status": "none",
                "temp": 20.0,
                "comfort": 21.0,
                "eco": 18.0,
                "ice": 8.0,
                "ice_mode": false,
                "last_sync_datetime_app": 1708360000000
            }
        }"#;

        let device: RointeDevice = serde_json::from_str(json).unwrap();
        assert_eq!(device.data.status, DeviceStatus::NoStatus);
    }

    #[test]
    fn test_deserialize_auto_mode() {
        let json = r#"{
            "data": {
                "name": "Hall",
                "type": "radiator",
                "power": true,
                "mode": "auto",
                "status": "eco",
                "temp": 18.0,
                "comfort": 21.0,
                "eco": 18.0,
                "ice": 8.0,
                "ice_mode": false,
                "last_sync_datetime_app": 1708360000000
            }
        }"#;

        let device: RointeDevice = serde_json::from_str(json).unwrap();
        assert_eq!(device.data.mode, DeviceMode::Auto);
    }
}

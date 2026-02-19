use std::fmt;

use serde::{Deserialize, Serialize};

/// Physical device type as reported by the API.
///
/// This enum is `#[non_exhaustive]` — Rointe may introduce new hardware
/// categories in future firmware updates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RointeProduct {
    /// Standard panel radiator.
    Radiator,
    /// Panel radiator (variant B).
    Radiatorb,
    /// Towel-rail radiator.
    Towel,
    /// ACS (hot-water storage / immersion) heater.
    Acs,
    /// Thermostat controller.
    Therm,
    /// Oval towel-rail radiator.
    OvalTowel,
}

/// Operating mode of the device.
///
/// - `Manual` — target temperature is set explicitly by the user.
/// - `Auto` — the device follows its programmed weekly schedule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceMode {
    /// Temperature is controlled manually.
    Manual,
    /// Temperature follows the weekly schedule.
    Auto,
}

impl fmt::Display for DeviceMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceMode::Manual => write!(f, "manual"),
            DeviceMode::Auto => write!(f, "auto"),
        }
    }
}

/// Current status / active preset of the device.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceStatus {
    /// Heating to the comfort preset temperature.
    Comfort,
    /// Heating to the eco (energy-saving) preset temperature.
    Eco,
    /// Frost-protection (ice) mode active.
    Ice,
    /// Device is off.
    Off,
    /// No preset active; device is in manual temperature control.
    #[serde(rename = "none")]
    NoStatus,
}

impl fmt::Display for DeviceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceStatus::Comfort => write!(f, "comfort"),
            DeviceStatus::Eco => write!(f, "eco"),
            DeviceStatus::Ice => write!(f, "ice"),
            DeviceStatus::Off => write!(f, "off"),
            DeviceStatus::NoStatus => write!(f, "none"),
        }
    }
}

/// High-level HVAC command used with [`crate::RointeClient::set_mode`].
#[derive(Debug, Clone, PartialEq)]
pub enum HvacMode {
    /// Turn the device off (two-step sequence).
    Off,
    /// Turn on and heat to the comfort temperature (two-step sequence).
    Heat,
    /// Follow the programmed weekly schedule (two-step sequence).
    Auto,
}

impl fmt::Display for HvacMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HvacMode::Off => write!(f, "off"),
            HvacMode::Heat => write!(f, "heat"),
            HvacMode::Auto => write!(f, "auto"),
        }
    }
}

/// Comfort preset used with [`crate::RointeClient::set_preset`].
#[derive(Debug, Clone, PartialEq)]
pub enum Preset {
    /// Heat to the device's configured comfort temperature.
    Comfort,
    /// Heat to the device's configured eco (energy-saving) temperature.
    Eco,
    /// Activate frost-protection (ice) mode.
    Ice,
}

impl fmt::Display for Preset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Preset::Comfort => write!(f, "comfort"),
            Preset::Eco => write!(f, "eco"),
            Preset::Ice => write!(f, "ice"),
        }
    }
}

/// Interpretation of a single character slot in a device's weekly schedule string.
///
/// Each day's schedule is a 24-character string; each character represents one
/// hour and maps to one of these modes.
#[derive(Debug, Clone, PartialEq)]
pub enum ScheduleMode {
    /// `'C'` — heat to the comfort preset temperature.
    Comfort,
    /// `'E'` — heat to the eco preset temperature.
    Eco,
    /// Any other character — device is off during this hour.
    Off,
}

impl ScheduleMode {
    /// Parse a schedule character into the corresponding [`ScheduleMode`].
    ///
    /// - `'C'` → [`ScheduleMode::Comfort`]
    /// - `'E'` → [`ScheduleMode::Eco`]
    /// - anything else → [`ScheduleMode::Off`]
    pub fn from_char(c: char) -> Self {
        match c {
            'C' => ScheduleMode::Comfort,
            'E' => ScheduleMode::Eco,
            _ => ScheduleMode::Off,
        }
    }
}

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A Rointe installation (home, building, or other location) as returned by
/// the `installations2` Firebase endpoint.
///
/// The `id` field is populated by the client from the Firebase map key — it
/// is not present in the JSON value itself and is skipped during deserialization.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Installation {
    /// Firebase map key for this installation; populated by the client after parsing.
    #[serde(skip)]
    pub id: String,

    /// Human-readable installation name as set in the Rointe app.
    pub name: Option<String>,

    /// Firebase user ID of the installation owner.
    pub userid: Option<String>,

    /// Top-level zones within this installation, keyed by zone ID.
    ///
    /// Zones can be nested — use [`crate::RointeClient::discover_devices`]
    /// to recursively collect all device IDs.
    pub zones: Option<HashMap<String, Zone>>,
}

/// A zone within an installation.
///
/// Zones can contain devices and/or nested sub-zones, allowing arbitrary
/// grouping hierarchies (e.g. floors, rooms).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    /// Human-readable zone name.
    pub name: Option<String>,

    /// Map of device ID → device metadata for devices in this zone.
    ///
    /// Only the map keys (device IDs) are used for discovery; the metadata
    /// values are not parsed.
    pub devices: Option<HashMap<String, serde_json::Value>>,

    /// Nested sub-zones within this zone, keyed by zone ID.
    pub zones: Option<HashMap<String, Zone>>,
}

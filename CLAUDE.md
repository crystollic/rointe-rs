# CLAUDE.md — Project Plan for `rointe-rs`

## Project Overview

**Name:** `rointe-rs`
**Location:** `~/Coding/rointe-rs`
**Language:** Rust (2021 edition)
**License:** MIT
**Purpose:** A Rust SDK and daemon for controlling Rointe WiFi-enabled radiators, replacing the Python `rointe-sdk` with better connection management and real-time streaming support.

## Background

The existing Python `rointe-sdk` (by Tiago Matias, MIT licensed) is a thin synchronous HTTP wrapper around Firebase Realtime Database. It suffers from SSL connection timeout issues when used in Home Assistant due to lack of connection pooling, retry logic, and keepalive management. This project is a clean-room Rust implementation using the same Firebase REST protocol, with the goal of producing a reliable daemon that bridges Rointe radiators to Home Assistant via MQTT.

### Attribution

This project is derived from protocol knowledge obtained from [rointe-sdk](https://github.com/tggm/rointe-sdk) by Tiago Matias, Copyright (c) 2022, MIT License. The Firebase REST API endpoints, JSON schemas, and control sequences documented below were reverse-engineered from that SDK. Include the original MIT license notice in `LICENSE-THIRD-PARTY`.

---

## Architecture

### Workspace Structure

```
~/Coding/rointe-rs/
├── CLAUDE.md                  # This file — project plan and protocol reference
├── Cargo.toml                 # Workspace manifest
├── LICENSE                    # MIT
├── LICENSE-THIRD-PARTY        # Attribution for rointe-sdk
├── README.md
├── crates/
│   ├── rointe-core/           # Core SDK library crate
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs         # Public API re-exports
│   │       ├── auth.rs        # Firebase Identity Toolkit auth
│   │       ├── client.rs      # RointeClient — high-level API
│   │       ├── error.rs       # Error types (thiserror)
│   │       ├── models/
│   │       │   ├── mod.rs
│   │       │   ├── device.rs      # RointeDevice, DeviceData
│   │       │   ├── installation.rs # Installation, Zone
│   │       │   ├── energy.rs      # EnergyConsumptionData
│   │       │   └── enums.rs       # Product, Mode, Preset, ScheduleMode
│   │       └── firebase/
│   │           ├── mod.rs
│   │           ├── rtdb.rs        # Firebase RTDB REST client
│   │           └── stream.rs      # SSE/EventSource streaming
│   ├── rointe-mqtt/           # MQTT bridge daemon
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── config.rs      # TOML config file support
│   │       ├── bridge.rs      # Rointe ↔ MQTT bridge logic
│   │       └── ha_discovery.rs # Home Assistant MQTT Discovery
│   └── rointe-cli/            # CLI tool for testing/debugging
│       ├── Cargo.toml
│       └── src/
│           └── main.rs
├── config/
│   └── rointe-mqtt.example.toml  # Example config
└── tests/
    └── integration/
```

### Crate Dependencies

**rointe-core:**
```toml
[dependencies]
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2"
tracing = "0.1"
eventsource-client = "0.13"       # SSE streaming for Firebase RTDB
futures = "0.3"
```

**rointe-mqtt:**
```toml
[dependencies]
rointe-core = { path = "../rointe-core" }
rumqttc = "0.24"                   # MQTT client
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
tracing = "0.1"
tracing-subscriber = "0.3"
clap = { version = "4", features = ["derive"] }
```

**rointe-cli:**
```toml
[dependencies]
rointe-core = { path = "../rointe-core" }
tokio = { version = "1", features = ["full"] }
clap = { version = "4", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

---

## Protocol Reference

### Firebase Configuration

```
API_KEY:      AIzaSyBi1DFJlBr9Cezf2BwfaT-PRPYmi3X3pdA
DATABASE_URL: https://elife-prod.firebaseio.com
```

### Authentication Flow

**Step 1: Login (email + password → tokens)**
```
POST https://www.googleapis.com/identitytoolkit/v3/relyingparty/verifyPassword?key={API_KEY}
Content-Type: application/x-www-form-urlencoded

email={email}&password={password}&returnSecureToken=true

Response JSON:
{
  "idToken": "...",        // Firebase auth token (use as ?auth= param)
  "refreshToken": "...",   // Long-lived refresh token
  "expiresIn": "3600",     // Token lifetime in seconds
  "localId": "...",        // User ID (needed for installation queries)
}
```

**Step 2: Refresh token (when idToken expires)**
```
POST https://securetoken.googleapis.com/v1/token?key={API_KEY}
Content-Type: application/x-www-form-urlencoded

grant_type=refresh_token&refresh_token={refresh_token}

Response JSON:
{
  "id_token": "...",       // Note: different key name vs login!
  "refresh_token": "...",
  "expires_in": "3600",
}
```

**Step 3: Get account info (optional, local_id already returned at login)**
```
POST https://www.googleapis.com/identitytoolkit/v3/relyingparty/getAccountInfo?key={API_KEY}

idToken={id_token}

Response: { "users": [{ "localId": "..." }] }
```

### Firebase RTDB Endpoints

All requests include `?auth={idToken}` as a query parameter.

| Operation | Method | URL Path | Notes |
|-----------|--------|----------|-------|
| List installations | GET | `/installations2.json?orderBy="userid"&equalTo="{local_id}"` | Returns map of installation objects |
| Get installation | GET | `/installations2.json?orderBy="userid"&equalTo="{local_id}"` | Filter by installation_id in response |
| Get device | GET | `/devices/{device_id}.json` | Full device object |
| Update device | PATCH | `/devices/{device_id}/data.json` | Control commands |
| Get energy stats | GET | `/history_statistics/{device_id}/daily/{YYYY}/{MM}/{DD}/energy/{HH}0000.json` | Hourly energy data |
| Get firmware info | GET | `/global_settings.json` | Latest firmware per device type |

### Installation → Device Discovery

1. `GET /installations2.json?orderBy="userid"&equalTo="{local_id}"` → returns map of installations
2. Each installation has `zones` → each zone has `devices` (map of device_id → metadata)
3. Zones can be nested (sub-zones within zones) — must recurse
4. Collect all device IDs, then `GET /devices/{id}.json` for each

### Device JSON Schema

```json
{
  "data": {
    "name": "Kitchen Radiator",
    "type": "radiator",           // radiator, radiatorb, towel, acs, therm, oval_towel
    "product_version": "v2",      // v1 or v2
    "nominal_power": 1500,
    "power": true,                // on/off
    "mode": "manual",             // "manual" or "auto"
    "status": "comfort",          // "comfort", "eco", "ice", "off", "none"
    "temp": 21.5,                 // target temperature
    "temp_calc": 21.3,            // calculated temperature
    "temp_probe": 21.2,           // probe/measured temperature
    "comfort": 21.0,              // comfort preset temp
    "eco": 18.0,                  // eco preset temp
    "ice": 8.0,                   // frost protection temp
    "ice_mode": true,             // frost protection enabled
    "schedule": [                 // 7 strings (Mon-Sun), each 24 chars
      "CCCCCCCCEEEEEEEEEEEEEECC", // C=Comfort, E=Eco, O=Off
      "CCCCCCCCEEEEEEEEEEEEEECC",
      "CCCCCCCCEEEEEEEEEEEEEECC",
      "CCCCCCCCEEEEEEEEEEEEEECC",
      "CCCCCCCCEEEEEEEEEEEEEECC",
      "CCCCCCCCCCCCCCCCCCCCCCCC",
      "CCCCCCCCCCCCCCCCCCCCCCCC"
    ],
    "schedule_day": 0,
    "schedule_hour": 0,
    "um_max_temp": 30.0,          // v2 only: user mode max temp
    "um_min_temp": 7.0,           // v2 only: user mode min temp
    "user_mode": false,           // v2 only
    "last_sync_datetime_app": 1708360000000,    // epoch milliseconds
    "last_sync_datetime_device": 1708359000000
  },
  "serialnumber": "ROINTE...",
  "firmware": {
    "firmware_version_device": "3.2.1"
  }
}
```

### Control Commands (PATCH to `/devices/{id}/data.json`)

**IMPORTANT:** Every PATCH body must include `"last_sync_datetime_app": <epoch_ms_now>`.

**Set temperature:**
```json
{ "temp": 21.5, "mode": "manual", "power": true, "last_sync_datetime_app": 1708360000000 }
```

**Set preset — comfort:**
```json
{ "power": true, "mode": "manual", "temp": <device.comfort_temp>, "status": "comfort", "last_sync_datetime_app": ... }
```

**Set preset — eco:**
```json
{ "power": true, "mode": "manual", "temp": <device.eco_temp>, "status": "eco", "last_sync_datetime_app": ... }
```

**Set preset — ice (frost protection):**
```json
{ "power": true, "mode": "manual", "temp": <device.ice_temp>, "status": "ice", "last_sync_datetime_app": ... }
```

**Turn off (auto mode):**
```json
{ "power": false, "mode": "auto", "status": "off", "last_sync_datetime_app": ... }
```

**Turn off (manual mode) — REQUIRES TWO REQUESTS:**
```
Request 1: PATCH { "temp": 20, "last_sync_datetime_app": ... }
Request 2: PATCH { "power": false, "mode": "manual", "status": "off", "last_sync_datetime_app": ... }
```

**Turn on (heat):**
```
Request 1: PATCH { "temp": <device.comfort_temp>, "last_sync_datetime_app": ... }
Request 2: PATCH { "mode": "manual", "power": true, "status": "none", "last_sync_datetime_app": ... }
```

**Set auto mode:**
```
Request 1: PATCH { "temp": <schedule_appropriate_temp>, "last_sync_datetime_app": ... }
Request 2: PATCH { "mode": "auto", "power": true, "last_sync_datetime_app": ... }
```

The schedule-appropriate temp is determined by the current day/hour in the device's schedule array:
- "C" → comfort_temp
- "E" → eco_temp
- If ice_mode → ice_temp
- Fallback → 20.0

### SSE Streaming (Enhancement over Python SDK)

Firebase RTDB supports Server-Sent Events for real-time updates:
```
GET https://elife-prod.firebaseio.com/devices/{device_id}/data.json
Accept: text/event-stream
Authorization via ?auth= query param

Events:
  event: put
  data: {"path":"/","data":{...full data object...}}

  event: patch
  data: {"path":"/temp","data":22.0}

  event: keep-alive
  data: null
```

This allows the daemon to receive instant push notifications when device state changes, eliminating the need for polling.

### Energy Statistics

```
GET /history_statistics/{device_id}/daily/{YYYY}/{MM}/{DD}/energy/{HH}0000.json

Response:
{
  "kw_h": 0.45,
  "effective_power": 750
}
```

The SDK tries the current hour first, then walks backwards up to 5 hours to find data.

---

## Implementation Phases

### Phase 1: Core Library (`rointe-core`)

**Goal:** Library crate that can authenticate, discover devices, read state, and send control commands.

1. **Error types** (`error.rs`)
   - `RointeError` enum with variants: Auth, Network, Firebase, DeviceNotFound, InvalidMode, Timeout
   - Use `thiserror` for derive

2. **Firebase auth** (`auth.rs`)
   - `FirebaseAuth` struct holding tokens and expiry
   - `login(email, password) -> Result<FirebaseAuth>`
   - `refresh() -> Result<()>` — auto-refresh when token nears expiry
   - `ensure_valid_token() -> Result<String>` — returns current valid token
   - Store refresh_token for persistence across restarts

3. **Firebase RTDB client** (`firebase/rtdb.rs`)
   - Thin wrapper around `reqwest::Client` with connection pooling
   - `get<T: DeserializeOwned>(path) -> Result<T>`
   - `patch<T: Serialize>(path, body) -> Result<()>`
   - Auto-injects `?auth=` parameter
   - Configurable timeouts and retry with exponential backoff

4. **Models** (`models/`)
   - `RointeDevice` — full device representation
   - `DeviceData` — the `/data` sub-object (serde rename attributes for JSON mapping)
   - `DeviceUpdate` — partial update struct for PATCH commands
   - `Installation`, `Zone` — for discovery
   - `EnergyConsumptionData`
   - Enums: `RointeProduct`, `DeviceMode`, `Preset`, `ScheduleMode`

5. **Client** (`client.rs`)
   - `RointeClient` — high-level API
   - `new(email, password) -> Result<Self>`
   - `get_installations() -> Result<Vec<Installation>>`
   - `discover_devices(installation_id) -> Result<Vec<String>>`
   - `get_device(device_id) -> Result<RointeDevice>`
   - `set_temperature(device_id, temp) -> Result<()>`
   - `set_preset(device_id, preset: Preset) -> Result<()>`
   - `set_mode(device_id, mode: HvacMode) -> Result<()>`
   - `get_energy_stats(device_id) -> Result<EnergyConsumptionData>`

6. **Tests**
   - Unit tests for model deserialization (use sample JSON fixtures)
   - Unit tests for schedule mode calculation
   - Integration tests (behind a feature flag, require real credentials)

### Phase 2: SSE Streaming

**Goal:** Add real-time event streaming using Firebase RTDB's SSE support.

1. **Stream client** (`firebase/stream.rs`)
   - Uses `eventsource-client` crate
   - `subscribe(device_id) -> Result<DeviceEventStream>`
   - Emits `DeviceEvent` enum: `StateChanged(DeviceData)`, `FieldUpdated(String, Value)`, `KeepAlive`
   - Auto-reconnect on connection drop with backoff

2. **Client extensions**
   - `watch_device(device_id, callback)` — convenience method
   - `watch_all_devices(callback)` — subscribe to multiple devices

### Phase 3: CLI Tool (`rointe-cli`)

**Goal:** Command-line tool for testing and manual control.

```bash
rointe-cli login --email user@example.com --password secret
rointe-cli devices                        # List all devices
rointe-cli status <device_id>             # Show device state
rointe-cli set-temp <device_id> 21.5      # Set temperature
rointe-cli set-preset <device_id> comfort # Set preset
rointe-cli set-mode <device_id> auto      # Set HVAC mode
rointe-cli off <device_id>                # Turn off
rointe-cli watch <device_id>              # Stream real-time updates
rointe-cli energy <device_id>             # Show energy stats
```

Use `clap` for argument parsing. Store auth tokens in `~/.config/rointe-rs/auth.json`.

### Phase 4: MQTT Bridge Daemon (`rointe-mqtt`)

**Goal:** Long-running daemon that bridges Rointe devices to MQTT with Home Assistant auto-discovery.

1. **Config** (`config.rs`)
   ```toml
   # rointe-mqtt.toml
   [rointe]
   email = "user@example.com"
   password = "secret"
   poll_interval_secs = 60         # Fallback polling interval
   use_streaming = true            # Use SSE streaming (preferred)

   [mqtt]
   host = "localhost"
   port = 1883
   username = "mqtt_user"          # optional
   password = "mqtt_pass"          # optional
   client_id = "rointe-bridge"
   discovery_prefix = "homeassistant"
   topic_prefix = "rointe"
   ```

2. **HA MQTT Discovery** (`ha_discovery.rs`)
   - Publishes discovery config for each device as HA `climate` entity
   - Supports: current_temperature, target_temperature, mode (off/heat/auto), preset (comfort/eco/ice)
   - Also publishes `sensor` entities for energy data, firmware version
   - Publishes `update` entity for firmware update availability

3. **Bridge logic** (`bridge.rs`)
   - On startup: authenticate, discover devices, publish HA discovery
   - SSE streaming for real-time state → publish to MQTT state topics
   - Subscribe to MQTT command topics → translate to Rointe API calls
   - Handle reconnection for both Firebase SSE and MQTT connections
   - Graceful shutdown on SIGTERM/SIGINT

4. **MQTT Topics:**
   ```
   # Discovery (published once per device)
   homeassistant/climate/rointe/{device_id}/config

   # State (published on change)
   rointe/{device_id}/state          # JSON: { temp, target, mode, preset, power, ... }
   rointe/{device_id}/availability   # "online" / "offline"

   # Commands (subscribed)
   rointe/{device_id}/set/temperature    # float
   rointe/{device_id}/set/mode           # "off", "heat", "auto"
   rointe/{device_id}/set/preset         # "comfort", "eco", "ice"
   ```

5. **Deployment**
   - Single static binary
   - Example systemd unit file in `config/rointe-mqtt.service`
   - Can run on same host as HA, or separate (e.g., Raspberry Pi)

---

## Key Design Decisions

### Use `reqwest` directly instead of `firebase-rs` crate

While `firebase-rs` (v2.2.3) exists and has SSE support, the Rointe use case is specific enough that wrapping `reqwest` directly gives more control over:
- Connection pooling and timeout configuration
- Auth token injection (firebase-rs expects pre-authenticated tokens)
- Retry logic with exponential backoff
- Custom error handling

The SSE streaming can use `eventsource-client` directly (which `firebase-rs` uses internally anyway).

### Workspace with multiple crates

Separating into `rointe-core`, `rointe-mqtt`, and `rointe-cli` keeps the library reusable. The core crate has no MQTT or CLI dependencies, making it suitable for embedding in other projects (e.g., a future MCP server or web API).

### MQTT bridge instead of HA Python integration

Running as an external MQTT bridge means:
- No dependency on HA's Python runtime or HACS
- Can be updated independently of HA
- Runs as a proper system service with process supervision
- Connection management is fully under our control
- Works with any MQTT-compatible home automation platform

### Token persistence

Store the refresh_token (not the short-lived id_token) in a config file or separate auth cache. On restart, attempt to refresh the token before falling back to full re-authentication. This avoids needing to store the password after first login.

---

## Testing Strategy

- **Unit tests:** Model serialization/deserialization, schedule mode calculation, auth token expiry logic
- **Integration tests (feature-gated):** Require `ROINTE_EMAIL` and `ROINTE_PASSWORD` env vars, test against live Firebase
- **Mock tests:** Use `mockito` or `wiremock` to test HTTP request/response handling without hitting Firebase

---

## Notes for Claude Code

- Start with Phase 1 — get the core library compiling and tested with sample JSON
- Use `cargo workspace` for the multi-crate setup
- Run `cargo clippy` and `cargo test` frequently
- The Firebase API key is public (embedded in the Rointe Connect web app) — it's safe to include in source
- The two-step PATCH sequences for mode changes are important — don't simplify them
- Every PATCH must include `last_sync_datetime_app` with current epoch milliseconds
- The `installations2.json` query uses Firebase's `orderBy`/`equalTo` filtering — the field names must be quoted with escaped inner quotes in the query params

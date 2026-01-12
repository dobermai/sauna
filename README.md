# Klafs Sauna Control

A Rust library and CLI for controlling Klafs saunas via their cloud API.

## Overview

This project provides:

- **klafs-core** - A Rust library for interacting with the Klafs sauna API
- **klafs-cli** - A command-line tool for controlling your sauna

## Installation

### From Source

```bash
git clone https://github.com/yourusername/klafs.git
cd klafs
cargo install --path klafs-cli
```

## CLI Usage

### Global Flags

```bash
# Enable verbose logging
klafs --verbose <command>

# Enable HTTP debug output (shows all requests/responses)
klafs --debug <command>

# Save debug output to a file
klafs --debug-file debug.log <command>
```

### Login

Authenticate with your Klafs account. Credentials are stored securely in your system keyring.

```bash
klafs login -u your@email.com
# Password will be prompted securely
```

> **Warning**: Klafs locks accounts after 3 failed login attempts!

### Discover Saunas

List all saunas registered to your account:

```bash
# Human-readable output
klafs saunas

# JSON output
klafs saunas --json
```

Example output:

```
Registered Saunas

* My Home Sauna
    ID: 364cc9db-86f1-49d1-86cd-f6ef9b20a490
    (default)

  Guest House Sauna
    ID: a1b2c3d4-e5f6-7890-abcd-ef1234567890

Use 'klafs config --sauna-id <ID>' to set a default.
```

### Configure Defaults

Set your default sauna ID to avoid specifying it with every command:

```bash
klafs config --sauna-id "your-sauna-uuid"

# Store PIN for power control (stored in system keyring)
klafs config --pin "1234"

# View current configuration
klafs config --show
```

### Get Sauna Status

```bash
# Human-readable output
klafs status

# JSON output
klafs status --json

# Specify a different sauna
klafs status --sauna-id "another-sauna-uuid"
```

Example output:

```
Sauna Status

  Connection:     Connected
  Power:          Off
  Status:         Standby

  Mode:           Sanarium
  Current Temp:   18°C
  Target Temp:    70°C
  Current Humid:  0%
  Target Humid:   Level 7
```

### Power Control

```bash
# Power on immediately (requires PIN)
klafs power-on

# Schedule power on for a specific time
klafs power-on --at 18:30

# Power off
klafs power-off
```

### Temperature and Mode

```bash
# Set temperature (10-100°C)
klafs set-temp 85

# Set mode: sauna, sanarium, or infrared
klafs set-mode sauna
```

### Humidity Control

```bash
# Set humidity level (1-10, for Sanarium mode)
klafs set-humidity 7
```

### Scheduling

```bash
# Set scheduled start time (without starting)
klafs schedule 18:30

# Clear the schedule
klafs schedule --clear
```

### Profiles

Save and reuse sauna configurations:

```bash
# Create a profile
klafs profile create hot --mode sauna --temp 90
klafs profile create relaxed --mode sanarium --temp 60 --humidity 7

# List all profiles
klafs profile list

# Show profile details
klafs profile show hot

# Apply a profile (sets mode, temperature, humidity)
klafs profile apply hot

# Apply and start the sauna
klafs profile apply hot --start

# Delete a profile
klafs profile delete hot
```

Profiles are stored in `~/.config/klafs/profiles.toml`.

### Configure Multiple Settings

Set multiple parameters in one command:

```bash
# Set temperature and humidity
klafs configure --temp 85 --humidity 5

# Set temperature and schedule
klafs configure --temp 85 --time 18:30

# Set all at once
klafs configure --temp 85 --humidity 5 --time 18:30
```

## Library Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
klafs-core = { path = "klafs-core" }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

Example:

```rust
use klafs_core::KlafsClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = KlafsClient::new();

    // Login
    client.login("user@example.com", "password").await?;

    // Get sauna status
    let status = client.get_status("your-sauna-uuid").await?;

    println!("Connected: {}", status.is_connected);
    println!("Powered On: {}", status.is_powered_on);
    println!("Current Temperature: {}°C", status.current_temperature);
    println!("Target Temperature: {}°C", status.target_temperature());

    if let Some(mode) = status.current_mode() {
        println!("Mode: {}", mode);
    }

    Ok(())
}
```

## API Reference

Base URL: `https://sauna-app-19.klafs.com`

### Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/Account/Login` | POST | Authenticate (form-encoded) |
| `/SaunaApp/ChangeSettings` | GET | List registered saunas (HTML) |
| `/SaunaApp/GetData?id={id}` | GET | Get sauna status (JSON) |
| `/SaunaApp/StartCabin` | POST | Power on sauna (supports scheduling) |
| `/SaunaApp/StopCabin` | POST | Power off sauna |
| `/SaunaApp/ChangeTemperature` | POST | Set target temperature |
| `/SaunaApp/ChangeHumLevel` | POST | Set humidity level |
| `/SaunaApp/SetMode` | POST | Set operating mode |
| `/SaunaApp/SetSelectedTime` | POST | Set scheduled start time |
| `/SaunaApp/FavoriteSelected` | POST | Apply profile settings |
| `/SaunaApp/PostConfigChange` | POST | Configure multiple settings |

### Sauna Modes

| Mode | Value | Temperature Range |
|------|-------|-------------------|
| Sauna | 1 | 10-100°C |
| Sanarium | 2 | 40-75°C |
| Infrared | 3 | - |

### Status Codes

| Code | Meaning |
|------|---------|
| 0 | Off |
| 1 | Heating Up |
| 2 | Ready |
| 3 | Standby |

## Project Structure

```
klafs/
├── Cargo.toml              # Workspace manifest
├── klafs-core/             # Core library
│   ├── src/
│   │   ├── lib.rs          # Public API
│   │   ├── client.rs       # HTTP client
│   │   ├── debug.rs        # HTTP traffic debugging
│   │   ├── error.rs        # Error types
│   │   └── models.rs       # Data models
│   └── tests/
│       ├── fixtures/       # Test fixtures (HTML, JSON)
│       └── integration_tests.rs
└── klafs-cli/              # CLI application
    └── src/
        ├── main.rs         # CLI commands
        ├── config.rs       # Configuration & keyring
        └── profiles.rs     # Profile storage
```

## Security Notes

- Credentials are stored in your system's secure keyring (macOS Keychain, Windows Credential Manager, or Linux Secret Service)
- The PIN for power control is also stored securely in the keyring
- Session cookies are managed in-memory and not persisted to disk
- **Klafs locks accounts after 3 failed login attempts** - be careful with automated scripts

## Roadmap

- [x] Core library with login and status
- [x] CLI with credential storage
- [x] Sauna discovery (list registered saunas)
- [x] Power on/off commands
- [x] Temperature, mode, humidity control
- [x] Scheduling (immediate and timed start)
- [x] Profiles feature (save/apply configurations)
- [x] Combined configure command
- [x] HTTP traffic debugging
- [x] Integration tests with mock server
- [ ] UniFFI bindings for iOS/macOS Swift apps
- [ ] TUI interface (ratatui-based)

## Acknowledgments

Based on reverse-engineering work from:
- [dss-vdc-klafs](https://github.com/axe-world/dss-vdc-klafs)
- [IPSymconKlafsSaunaControl](https://github.com/Pommespanzer/IPSymconKlafsSaunaControl)

## License

MIT

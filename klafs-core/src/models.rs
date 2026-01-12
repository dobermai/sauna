use serde::{Deserialize, Serialize};

/// Information about a registered sauna
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaunaInfo {
    /// Unique identifier (UUID) for the sauna
    pub id: String,
    /// Display name of the sauna
    pub name: String,
}

/// Operating mode for the sauna
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub enum SaunaMode {
    /// Traditional sauna mode (dry heat, 10-100°C)
    Sauna = 1,
    /// Sanarium mode (humid heat, 40-75°C)
    Sanarium = 2,
    /// Infrared mode
    Infrared = 3,
}

impl TryFrom<u8> for SaunaMode {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(SaunaMode::Sauna),
            2 => Ok(SaunaMode::Sanarium),
            3 => Ok(SaunaMode::Infrared),
            _ => Err(format!("Invalid sauna mode: {}", value)),
        }
    }
}

impl From<SaunaMode> for u8 {
    fn from(mode: SaunaMode) -> Self {
        mode as u8
    }
}

impl std::fmt::Display for SaunaMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaunaMode::Sauna => write!(f, "Sauna"),
            SaunaMode::Sanarium => write!(f, "Sanarium"),
            SaunaMode::Infrared => write!(f, "Infrared"),
        }
    }
}

/// Status code returned by the API
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusCode {
    /// Sauna is off
    Off,
    /// Sauna is heating up
    HeatingUp,
    /// Sauna is ready for use
    Ready,
    /// Sauna is in standby/idle
    Standby,
    /// Unknown status
    Unknown(i32),
}

impl From<i32> for StatusCode {
    fn from(value: i32) -> Self {
        match value {
            0 => StatusCode::Off,
            1 => StatusCode::HeatingUp,
            2 => StatusCode::Ready,
            3 => StatusCode::Standby,
            other => StatusCode::Unknown(other),
        }
    }
}

impl std::fmt::Display for StatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StatusCode::Off => write!(f, "Off"),
            StatusCode::HeatingUp => write!(f, "Heating Up"),
            StatusCode::Ready => write!(f, "Ready"),
            StatusCode::Standby => write!(f, "Standby"),
            StatusCode::Unknown(code) => write!(f, "Unknown ({})", code),
        }
    }
}

/// Full sauna status as returned by the GetSaunaStatus API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaunaStatus {
    /// Unique identifier for this sauna
    pub sauna_id: String,

    /// Whether traditional sauna mode is selected
    pub sauna_selected: bool,

    /// Whether sanarium mode is selected
    pub sanarium_selected: bool,

    /// Whether infrared mode is selected
    pub ir_selected: bool,

    /// Target temperature for sauna mode (10-100°C)
    pub selected_sauna_temperature: i32,

    /// Target temperature for sanarium mode (40-75°C)
    pub selected_sanarium_temperature: i32,

    /// Target temperature for infrared mode
    pub selected_ir_temperature: i32,

    /// Humidity level for sanarium mode (1-10)
    pub selected_hum_level: i32,

    /// Infrared level (1-10)
    pub selected_ir_level: i32,

    /// Scheduled start hour (0-23)
    pub selected_hour: i32,

    /// Scheduled start minute (0-59)
    pub selected_minute: i32,

    /// Whether the sauna is connected to the network
    pub is_connected: bool,

    /// Whether the sauna is currently powered on
    pub is_powered_on: bool,

    /// Whether the sauna is ready for use (reached target temperature)
    pub is_ready_for_use: bool,

    /// Current measured temperature in °C
    pub current_temperature: i32,

    /// Current measured humidity percentage
    pub current_humidity: i32,

    /// Status code (see StatusCode enum)
    pub status_code: i32,

    /// Optional status message
    pub status_message: Option<String>,

    /// Whether to show bathing hours
    pub show_bathing_hour: bool,

    /// Remaining bathing hours
    pub bathing_hours: i32,

    /// Remaining bathing minutes
    pub bathing_minutes: i32,

    /// Current humidity status indicator
    pub current_humidity_status: i32,

    /// Current temperature status indicator
    pub current_temperature_status: i32,
}

impl SaunaStatus {
    /// Get the currently selected mode
    pub fn current_mode(&self) -> Option<SaunaMode> {
        if self.sauna_selected {
            Some(SaunaMode::Sauna)
        } else if self.sanarium_selected {
            Some(SaunaMode::Sanarium)
        } else if self.ir_selected {
            Some(SaunaMode::Infrared)
        } else {
            None
        }
    }

    /// Get the target temperature for the currently selected mode
    pub fn target_temperature(&self) -> i32 {
        if self.sauna_selected {
            self.selected_sauna_temperature
        } else if self.sanarium_selected {
            self.selected_sanarium_temperature
        } else if self.ir_selected {
            self.selected_ir_temperature
        } else {
            self.selected_sauna_temperature
        }
    }

    /// Get the status code as an enum
    pub fn status(&self) -> StatusCode {
        StatusCode::from(self.status_code)
    }

    /// Get remaining bathing time as a formatted string
    pub fn remaining_time(&self) -> String {
        format!("{}h {:02}m", self.bathing_hours, self.bathing_minutes)
    }
}

/// Request body for StartCabin endpoint
#[derive(Debug, Serialize)]
pub(crate) struct PowerControlRequest {
    pub id: String,
    pub pin: String,
    pub time_selected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sel_hour: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sel_min: Option<i32>,
}

/// Request body for setting temperature
#[derive(Debug, Serialize)]
pub(crate) struct SetTemperatureRequest {
    pub id: String,
    pub temperature: i32,
}

/// Request body for setting humidity level
#[derive(Debug, Serialize)]
pub(crate) struct SetHumidityRequest {
    pub id: String,
    pub level: i32,
}

/// Request body for setting mode
#[derive(Debug, Serialize)]
pub(crate) struct SetModeRequest {
    pub id: String,
    pub selected_mode: u8,
}

/// Configuration change request for PostConfigChange endpoint
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConfigChangeRequest {
    pub sauna_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_sauna_temperature: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_sanarium_temperature: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_hum_level: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_hour: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_minute: Option<i32>,
}

/// Request body for SetSelectedTime endpoint (schedule without starting)
#[derive(Debug, Serialize)]
pub(crate) struct SetSelectedTimeRequest {
    pub id: String,
    pub time_set: bool,
    pub hours: i32,
    pub minutes: i32,
}

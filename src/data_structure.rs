use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CoreProps {
    pub address: String,
}

#[derive(Debug, Serialize)]
pub struct GameMetadata {
    pub game: String,
    pub game_display_name: String,
    pub developer: String,
    pub deinitialize_timer_length_ms: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all(serialize = "kebab-case"))]
pub struct ScreenHandler {
    pub device_type: String,
    pub zone: String,
    pub mode: String,
    pub datas: Vec<ScreenHandlerData>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all(serialize = "kebab-case"))]
pub struct ScreenHandlerDataLine {
    pub has_text: bool,
    pub context_frame_key: String,
    pub wrap: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all(serialize = "kebab-case"))]
pub struct ScreenHandlerData {
    pub lines: Vec<ScreenHandlerDataLine>,
    pub icon_id: i32,
}

#[derive(Debug, Serialize)]
pub struct BindEventDefinition {
    pub game: String,
    pub event: String,
    pub icon_id: i32,
    pub value_optional: bool,
    pub handlers: Vec<ScreenHandler>,
    pub data_fields: Vec<DataField>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all(serialize = "kebab-case"))]
pub struct DataField {
    pub context_frame_key: String,
    pub label: String,
}

#[derive(Debug, Serialize)]
pub struct Event {
    pub game: String,
    pub event: String,
    pub data: EventData,
}

#[derive(Debug, Serialize)]
pub struct Game {
    pub game: String,
}

#[derive(Debug, Serialize)]
pub struct EventData {
    pub value: i32,
    pub frame: HashMap<String, String>,
}
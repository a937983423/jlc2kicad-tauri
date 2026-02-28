use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub name: String,
    pub description: String,
    pub package: Option<String>,
    pub manufacturer: Option<String>,
    pub category: Option<String>,
    pub price: Option<String>,
    pub stock: Option<String>,
}

#[derive(Error, Debug)]
pub enum JlcError {
    #[error("HTTP request failed: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("API returned error: {0}")]
    ApiError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Parse error: {0}")]
    ParseError(String),
}

impl Serialize for JlcError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSettings {
    pub easyeda_use_proxy: bool,
    pub lcsc_use_proxy: bool,
    pub proxy_address: String,
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            easyeda_use_proxy: true,
            lcsc_use_proxy: false,
            proxy_address: "http://127.0.0.1:10808".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentData {
    pub success: bool,
    pub result: Vec<ComponentResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentResult {
    #[serde(rename = "component_uuid")]
    pub component_uuid: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FootprintApiResponse {
    pub success: bool,
    pub result: FootprintResult,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FootprintResult {
    pub title: String,
    #[serde(rename = "dataStr")]
    pub data_str: FootprintDataStr,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FootprintDataStr {
    pub shape: Vec<String>,
    pub head: FootprintHead,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FootprintHead {
    pub x: f64,
    pub y: f64,
    #[serde(rename = "c_para")]
    pub c_para: Option<FootprintCPara>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FootprintCPara {
    pub link: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SymbolApiResponse {
    pub success: bool,
    pub result: SymbolResult,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SymbolResult {
    pub title: String,
    #[serde(rename = "dataStr")]
    pub data_str: SymbolDataStr,
    #[serde(rename = "packageDetail")]
    pub package_detail: PackageDetail,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SymbolDataStr {
    pub shape: Vec<String>,
    pub head: SymbolHead,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SymbolHead {
    pub x: f64,
    pub y: f64,
    #[serde(rename = "c_para")]
    pub c_para: SymbolCPara,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SymbolCPara {
    pub pre: String,
    #[serde(rename = "Resistance")]
    pub resistance: Option<String>,
    #[serde(rename = "Capacitance")]
    pub capacitance: Option<String>,
    #[serde(rename = "Inductance")]
    pub inductance: Option<String>,
    #[serde(rename = "Frequency")]
    pub frequency: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageDetail {
    #[serde(rename = "dataStr")]
    pub data_str: PackageDetailData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageDetailData {
    pub head: PackageDetailHead,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageDetailHead {
    #[serde(rename = "c_para")]
    pub c_para: SymbolCPara,
}

#[derive(Debug, Clone)]
pub struct FootprintInfo {
    pub max_x: f64,
    pub max_y: f64,
    pub min_x: f64,
    pub min_y: f64,
    pub footprint_name: String,
    pub output_dir: String,
    pub footprint_lib: String,
    pub model_base_variable: String,
    pub model_dir: String,
    pub origin: (f64, f64),
    pub models: Vec<String>,
}

impl Default for FootprintInfo {
    fn default() -> Self {
        Self {
            max_x: -10000.0,
            max_y: -10000.0,
            min_x: 10000.0,
            min_y: 10000.0,
            footprint_name: String::new(),
            output_dir: String::from("."),
            footprint_lib: String::from("footprint"),
            model_base_variable: String::new(),
            model_dir: String::from("packages3d"),
            origin: (0.0, 0.0),
            models: vec![String::from("STEP")],
        }
    }
}

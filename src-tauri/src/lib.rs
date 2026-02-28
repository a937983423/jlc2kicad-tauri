use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
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
    pub image_url: Option<String>,
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

const USER_AGENT: &str = "JLC2KiCad/1.0.0 (https://github.com/TousstNicolas/JLC2KiCad_lib)";
const EASYEDA_BASE_URLS: [&str; 2] = ["https://lceda.cn", "https://easyeda.com"];
const PRO_EASYEDA_BASE_URLS: [&str; 2] = ["https://pro.lceda.cn", "https://pro.easyeda.com"];
const MODEL_BASE_URLS: [&str; 2] = ["https://modules.lceda.cn", "https://modules.easyeda.com"];

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

static NETWORK_SETTINGS: OnceLock<Mutex<NetworkSettings>> = OnceLock::new();

fn network_settings_store() -> &'static Mutex<NetworkSettings> {
    NETWORK_SETTINGS.get_or_init(|| Mutex::new(NetworkSettings::default()))
}

pub fn get_network_settings() -> NetworkSettings {
    network_settings_store()
        .lock()
        .map(|s| s.clone())
        .unwrap_or_default()
}

pub fn set_network_settings(settings: NetworkSettings) -> Result<(), JlcError> {
    let proxy_addr = settings.proxy_address.trim();
    
    if settings.easyeda_use_proxy && !proxy_addr.is_empty() {
        reqwest::Proxy::all(proxy_addr)
            .map_err(|e| JlcError::ApiError(format!("代理地址无效: {}", e)))?;
    }

    if settings.lcsc_use_proxy && !proxy_addr.is_empty() {
        reqwest::Proxy::all(proxy_addr)
            .map_err(|e| JlcError::ApiError(format!("代理地址无效: {}", e)))?;
    }

    match network_settings_store().lock() {
        Ok(mut state) => {
            *state = settings;
            Ok(())
        }
        Err(_) => Err(JlcError::ApiError("无法写入网络设置".to_string())),
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

fn mil2mm(mils: f64) -> f64 {
    mils / 3.937
}

fn sanitize_footprint_name(title: &str) -> String {
    title
        .replace(" ", "_")
        .replace("/", "_")
        .replace("(", "_")
        .replace(")", "_")
}

fn extract_model_uuid_from_shape(shape: &[String]) -> Option<String> {
    for line in shape {
        let parts: Vec<&str> = line.split('~').filter(|s| !s.is_empty()).collect();
        if parts.first().copied() != Some("SVGNODE") {
            continue;
        }
        if let Some(raw) = parts.get(1) {
            if let Ok(json_data) = serde_json::from_str::<serde_json::Value>(raw) {
                if let Some(uuid) = json_data
                    .get("attrs")
                    .and_then(|a| a.get("uuid"))
                    .and_then(|u| u.as_str())
                {
                    return Some(uuid.to_string());
                }
            }
        }
    }
    None
}

fn uuid_first_part(value: &str) -> String {
    value.split('|').next().unwrap_or(value).to_string()
}

fn first_non_empty_str(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(v) = value.get(*key).and_then(|v| v.as_str()) {
            let trimmed = v.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn looks_like_uuidish(value: &str) -> bool {
    let s = value.trim();
    if s.is_empty() {
        return false;
    }

    let first = s.split('|').next().unwrap_or(s);
    let is_hex = |c: char| c.is_ascii_hexdigit();

    if first.len() == 32 && first.chars().all(is_hex) {
        return true;
    }

    if first.len() == 36 {
        let bytes = first.as_bytes();
        if bytes.get(8) == Some(&b'-')
            && bytes.get(13) == Some(&b'-')
            && bytes.get(18) == Some(&b'-')
            && bytes.get(23) == Some(&b'-')
            && first
                .chars()
                .enumerate()
                .all(|(i, ch)| matches!(i, 8 | 13 | 18 | 23) || is_hex(ch))
        {
            return true;
        }
    }

    false
}

fn first_readable_package(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(v) = value.get(*key).and_then(|v| v.as_str()) {
            let trimmed = v.trim();
            if !trimmed.is_empty() && !looks_like_uuidish(trimmed) {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn extract_package_name(value: &serde_json::Value) -> Option<String> {
    if let Some(v) = first_readable_package(
        value,
        &[
            "package_title",
            "packageTitle",
            "package_name",
            "packageName",
            "package",
            "footprint_title",
            "footprintTitle",
            "footprint_name",
            "footprintName",
        ],
    ) {
        return Some(v);
    }

    for key in ["footprint", "package", "packageDetail", "pkg"] {
        if let Some(obj) = value.get(key) {
            if let Some(s) = obj.as_str() {
                let trimmed = s.trim();
                if !trimmed.is_empty() && !looks_like_uuidish(trimmed) {
                    return Some(trimmed.to_string());
                }
            }

            if obj.is_object() {
                if let Some(v) = first_non_empty_str(
                    obj,
                    &[
                        "display_title",
                        "displayTitle",
                        "title",
                        "name",
                        "package_name",
                        "footprint_name",
                    ],
                ) {
                    if !looks_like_uuidish(&v) {
                        return Some(v);
                    }
                }
            }
        }
    }

    if let Some(attrs) = value.get("attributes") {
        if let Some(v) = first_readable_package(
            attrs,
            &[
                "Footprint Title",
                "Supplier Footprint",
                "Package Name",
                "Package",
                "Device Package",
                "封装名称",
                "封装",
                "Footprint",
            ],
        ) {
            return Some(v);
        }

        if let Some(footprint) = attrs.get("footprint") {
            if let Some(v) = first_non_empty_str(
                footprint,
                &["display_title", "title", "name", "package_name", "footprint_name"],
            ) {
                if !looks_like_uuidish(&v) {
                    return Some(v);
                }
            }
        }
    }

    None
}

fn extract_manufacturer_name(value: &serde_json::Value) -> Option<String> {
    if let Some(v) = first_non_empty_str(
        value,
        &[
            "manufacturer",
            "Manufacturer",
            "brand",
            "Brand",
            "mfr",
            "vendor",
            "supplier",
            "制造商",
            "品牌",
        ],
    ) {
        return Some(v);
    }

    if let Some(attrs) = value.get("attributes") {
        if let Some(v) = first_non_empty_str(
            attrs,
            &[
                "Manufacturer",
                "Brand",
                "Supplier",
                "Vendor",
                "制造商",
                "品牌",
            ],
        ) {
            return Some(v);
        }
    }

    None
}

fn extract_brief_desc(value: &serde_json::Value) -> Option<String> {
    if let Some(v) = first_non_empty_str(
        value,
        &[
            "description",
            "Description",
            "comment",
            "Comment",
            "product_name",
            "display_title",
            "title",
            "描述",
        ],
    ) {
        return Some(v);
    }

    if let Some(attrs) = value.get("attributes") {
        if let Some(v) = first_non_empty_str(
            attrs,
            &["Description", "Comment", "Value", "描述", "备注"],
        ) {
            return Some(v);
        }
    }

    None
}

fn normalize_display_name(raw: Option<String>, fallback_id: &str, package_hint: Option<&str>) -> String {
    let candidate = raw.unwrap_or_default().trim().to_string();
    if candidate.is_empty() || looks_like_uuidish(&candidate) || candidate.len() > 100 {
        if let Some(pkg) = package_hint {
            let p = pkg.trim();
            if !p.is_empty() && !looks_like_uuidish(p) {
                return p.to_string();
            }
        }
        return fallback_id.to_string();
    }
    candidate
}

fn extract_preferred_local_id(device: &serde_json::Value) -> Option<String> {
    let attrs = device.get("attributes").unwrap_or(device);

    // 1) Prefer explicit C-code style IDs from common keys.
    let direct = first_non_empty_str(
        device,
        &["product_code", "productCode", "code", "lcsc", "partNumber", "part_number"],
    )
    .or_else(|| {
        first_non_empty_str(
            attrs,
            &[
                "product_code",
                "Product Code",
                "LCSC",
                "LCSC Part",
                "LCSC Part #",
                "Part Number",
                "Code",
            ],
        )
    });
    if let Some(v) = direct.and_then(|s| normalize_component_token(&s)) {
        if v.to_uppercase().starts_with('C') {
            return Some(v);
        }
    }

    // 2) Regex scan entire JSON value for Cxxxx, covers odd schema variants.
    if let Ok(text) = serde_json::to_string(device) {
        if let Some(m) = component_id_regex().find(&text) {
            if let Some(id) = normalize_component_token(m.as_str()) {
                return Some(id);
            }
        }
    }

    // 3) Fallback to UUID-style IDs only if no C-code exists.
    first_non_empty_str(device, &["id", "uuid"])
        .or_else(|| first_non_empty_str(attrs, &["uuid"]))
        .and_then(|s| normalize_component_token(&s))
}

#[allow(dead_code)]
fn get_user_agent() -> String {
    USER_AGENT.to_string()
}

pub struct JlcClient {
    easyeda_primary_client: reqwest::Client,
    easyeda_fallback_client: reqwest::Client,
    lcsc_client: reqwest::Client,
}

impl JlcClient {
    fn build_client(proxy: Option<&str>) -> Result<reqwest::Client, reqwest::Error> {
        let mut builder = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(10));

        if let Some(proxy_url) = proxy {
            if !proxy_url.trim().is_empty() {
                builder = builder.proxy(reqwest::Proxy::all(proxy_url.trim())?);
            }
        }

        builder.build()
    }

    pub fn new() -> Self {
        let settings = get_network_settings();

        let lcsc_proxy = if settings.lcsc_use_proxy {
            Some(settings.proxy_address.as_str())
        } else {
            None
        };
        let lcsc_client = Self::build_client(lcsc_proxy).unwrap_or_else(|e| {
            log::warn!("Failed to create LCSC client: {}", e);
            reqwest::Client::new()
        });

        let easyeda_proxy = if settings.easyeda_use_proxy {
            Some(settings.proxy_address.as_str())
        } else {
            None
        };

        let easyeda_primary_client = Self::build_client(easyeda_proxy).unwrap_or_else(|e| {
            log::warn!(
                "Failed to create EasyEDA proxy client, fallback to direct: {}",
                e
            );
            Self::build_client(None).unwrap_or_else(|_| reqwest::Client::new())
        });

        let easyeda_fallback_proxy = if settings.easyeda_use_proxy {
            None
        } else {
            Some(settings.proxy_address.as_str())
        };
        let easyeda_fallback_client =
            Self::build_client(easyeda_fallback_proxy).unwrap_or_else(|e| {
                log::warn!("Failed to create EasyEDA fallback client: {}", e);
                Self::build_client(None).unwrap_or_else(|_| reqwest::Client::new())
            });

        Self {
            easyeda_primary_client,
            easyeda_fallback_client,
            lcsc_client,
        }
    }

    async fn easyeda_get_text_url(&self, url: &str) -> Result<String, JlcError> {
        let primary = self
            .easyeda_primary_client
            .get(url)
            .send()
            .await
            .and_then(|r| r.error_for_status());

        match primary {
            Ok(resp) => Ok(resp.text().await?),
            Err(primary_err) => {
                log::warn!("EasyEDA primary request failed: {}", primary_err);
                let fallback_resp = self
                    .easyeda_fallback_client
                    .get(url)
                    .send()
                    .await?
                    .error_for_status()?;
                Ok(fallback_resp.text().await?)
            }
        }
    }

    async fn easyeda_get_text_path(&self, path: &str) -> Result<String, JlcError> {
        let mut last_err: Option<JlcError> = None;
        for base in EASYEDA_BASE_URLS {
            let url = format!("{}{}", base, path);
            match self.easyeda_get_text_url(&url).await {
                Ok(text) => return Ok(text),
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap_or_else(|| JlcError::ApiError("EasyEDA 请求失败".to_string())))
    }

    async fn easyeda_get_text_pro_path(&self, path: &str) -> Result<String, JlcError> {
        let mut last_err: Option<JlcError> = None;
        for base in PRO_EASYEDA_BASE_URLS {
            let url = format!("{}{}", base, path);
            match self.easyeda_get_text_url(&url).await {
                Ok(text) => return Ok(text),
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap_or_else(|| JlcError::ApiError("EasyEDA Pro 请求失败".to_string())))
    }

    async fn easyeda_get_bytes_url(&self, url: &str) -> Result<Vec<u8>, JlcError> {
        let primary = self
            .easyeda_primary_client
            .get(url)
            .send()
            .await
            .and_then(|r| r.error_for_status());

        match primary {
            Ok(resp) => Ok(resp.bytes().await?.to_vec()),
            Err(primary_err) => {
                log::warn!("EasyEDA primary request failed: {}", primary_err);
                let fallback_resp = self
                    .easyeda_fallback_client
                    .get(url)
                    .send()
                    .await?
                    .error_for_status()?;
                Ok(fallback_resp.bytes().await?.to_vec())
            }
        }
    }

    async fn easyeda_get_bytes_with_bases(
        &self,
        bases: &[&str],
        path: &str,
    ) -> Result<Vec<u8>, JlcError> {
        let mut last_err: Option<JlcError> = None;
        for base in bases {
            let url = format!("{}{}", base, path);
            match self.easyeda_get_bytes_url(&url).await {
                Ok(bytes) => return Ok(bytes),
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap_or_else(|| JlcError::ApiError("EasyEDA 请求失败".to_string())))
    }

    async fn easyeda_post_form_json(
        &self,
        path: &str,
        form: &[(&str, String)],
    ) -> Result<serde_json::Value, JlcError> {
        let mut last_err: Option<JlcError> = None;
        for base in PRO_EASYEDA_BASE_URLS {
            let url = format!("{}{}", base, path);
            let primary = self
                .easyeda_primary_client
                .post(&url)
                .form(form)
                .send()
                .await
                .and_then(|r| r.error_for_status());

            let text = match primary {
                Ok(resp) => resp.text().await?,
                Err(primary_err) => {
                    log::warn!("EasyEDA primary POST failed on {}: {}", base, primary_err);
                    match self
                        .easyeda_fallback_client
                        .post(&url)
                        .form(form)
                        .send()
                        .await
                        .and_then(|r| r.error_for_status())
                    {
                        Ok(resp) => resp.text().await?,
                        Err(e) => {
                            last_err = Some(JlcError::RequestError(e));
                            continue;
                        }
                    }
                }
            };

            match serde_json::from_str(&text) {
                Ok(v) => return Ok(v),
                Err(e) => last_err = Some(JlcError::JsonError(e)),
            }
        }

        Err(last_err.unwrap_or_else(|| JlcError::ApiError("EasyEDA 请求失败".to_string())))
    }

    async fn get_pro_device_detail(
        &self,
        device_uuid: &str,
    ) -> Result<serde_json::Value, JlcError> {
        let text = self
            .easyeda_get_text_pro_path(&format!("/api/devices/{}", device_uuid))
            .await?;
        let json: serde_json::Value = serde_json::from_str(&text)?;
        Ok(json)
    }

    pub async fn search_components(&self, query: &str) -> Result<Vec<SearchResult>, JlcError> {
        let path = format!("/api/products/{}/svgs", query);
        let text = self.easyeda_get_text_path(&path).await?;
        
        let data: ComponentData = match serde_json::from_str(&text) {
            Ok(d) => d,
            Err(_) => {
                return Err(JlcError::ApiError("Invalid response format".to_string()));
            }
        };
        
        if !data.success || data.result.is_empty() {
            return Ok(vec![]);
        }
        
        let footprint_uuid = &data.result.last().unwrap().component_uuid;
        let footprint_data = self.get_footprint_data(footprint_uuid).await?;
        
        let name = footprint_data.result.title.clone();
        
        Ok(vec![SearchResult {
            id: query.to_string(),
            name: name,
            description: "".to_string(),
            package: None,
            manufacturer: None,
            category: None,
            price: None,
            stock: None,
            image_url: Some(format!("https://wmsc.lcsc.com/wmsc/upload/file/eec/image/{}.jpg", query)),
        }])
    }

    pub async fn search_easyeda_pro(&self, query: &str) -> Result<Vec<SearchResult>, JlcError> {
        let mut results = Vec::new();
        let mut seen = HashSet::new();
        let q = query.trim();

        // Same API family as jlc-kicad-lib-loader plugin:
        // https://pro.easyeda.com/api/v2/devices/searchByCodes
        if q.to_uppercase().starts_with('C') {
            let by_codes = self
                .easyeda_post_form_json(
                    "/api/v2/devices/searchByCodes",
                    &[("codes[]", q.to_string())],
                )
                .await?;

            if by_codes
                .get("success")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                if let Some(arr) = by_codes.get("result").and_then(|v| v.as_array()) {
                    for item in arr {
                        let device_uuid = item
                            .get("uuid")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .trim()
                            .to_string();
                        let id = item
                            .get("product_code")
                            .or_else(|| item.get("code"))
                            .or_else(|| item.get("uuid"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .trim()
                            .to_string();

                        if id.is_empty() || seen.contains(&id) {
                            continue;
                        }
                        seen.insert(id.clone());

                        let mut name = first_non_empty_str(
                            item,
                            &["display_title", "title", "name", "product_name"],
                        )
                        .unwrap_or_else(|| id.clone());
                        let mut package_value = extract_package_name(item);
                        let mut manufacturer_value = extract_manufacturer_name(item);
                        let mut brief_desc_value = extract_brief_desc(item);

                        // For C-code queries, some responses only return code + uuid.
                        // Enrich with device detail so UI can show name and basic info.
                        if (!device_uuid.is_empty())
                            && (name == id
                                || package_value.is_none()
                                || manufacturer_value.is_none()
                                || brief_desc_value.is_none())
                        {
                            if let Ok(device_json) = self.get_pro_device_detail(&device_uuid).await {
                                let result = device_json.get("result").unwrap_or(&device_json);

                                if name == id {
                                    if let Some(detail_name) = first_non_empty_str(
                                        result,
                                        &["display_title", "title", "name"],
                                    ) {
                                        name = detail_name;
                                    }
                                }

                                if package_value.is_none() {
                                    package_value = extract_package_name(result);
                                }
                                if manufacturer_value.is_none() {
                                    manufacturer_value = extract_manufacturer_name(result);
                                }
                                if brief_desc_value.is_none() {
                                    brief_desc_value = extract_brief_desc(result);
                                }
                            }
                        }
                        let description = format!(
                            "封装: {} | 制造商: {} | 描述: {}",
                            package_value.clone().unwrap_or_else(|| "未知".to_string()),
                            manufacturer_value.clone().unwrap_or_else(|| "未知".to_string()),
                            brief_desc_value.unwrap_or_else(|| "未知".to_string())
                        );

                        results.push(SearchResult {
                            id,
                            name,
                            description,
                            package: package_value,
                            manufacturer: manufacturer_value,
                            category: None,
                            price: None,
                            stock: None,
                            image_url: None,
                        });
                    }
                }
            }
        }

        if !results.is_empty() {
            return Ok(results);
        }

        // Same API family as jlc-kicad-lib-loader plugin:
        // https://pro.easyeda.com/api/v2/devices/search
        let search_data = self
            .easyeda_post_form_json(
                "/api/v2/devices/search",
                &[
                    ("page", "1".to_string()),
                    ("pageSize", "20".to_string()),
                    ("wd", q.to_string()),
                    ("returnListStyle", "classifyarr".to_string()),
                ],
            )
            .await?;

        if !search_data
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            return Err(JlcError::ApiError("EasyEDA 搜索失败".to_string()));
        }

        if let Some(lists) = search_data
            .get("result")
            .and_then(|v| v.get("lists"))
            .and_then(|v| v.as_object())
        {
            for group in lists.values() {
                if let Some(items) = group.as_array() {
                    for item in items {
                        let id = item
                            .get("product_code")
                            .or_else(|| item.get("uuid"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .trim()
                            .to_string();

                        if id.is_empty() || seen.contains(&id) {
                            continue;
                        }
                        seen.insert(id.clone());

                        let device_uuid = item
                            .get("uuid")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .trim()
                            .to_string();
                        let name = item
                            .get("display_title")
                            .or_else(|| item.get("title"))
                            .or_else(|| item.get("name"))
                            .or_else(|| item.get("product_name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or(&id)
                            .to_string();
                        let mut package_value = extract_package_name(item);
                        let mut manufacturer_value = extract_manufacturer_name(item);
                        let mut brief_desc_value = extract_brief_desc(item);
                        // Keep keyword search results consistent with C-code search:
                        // if list payload has little metadata, enrich by device detail.
                        if !device_uuid.is_empty()
                            && (package_value.is_none()
                                || manufacturer_value.is_none()
                                || brief_desc_value.is_none())
                        {
                            if let Ok(device_json) = self.get_pro_device_detail(&device_uuid).await {
                                let result = device_json.get("result").unwrap_or(&device_json);
                                if package_value.is_none() {
                                    package_value = extract_package_name(result);
                                }
                                if manufacturer_value.is_none() {
                                    manufacturer_value = extract_manufacturer_name(result);
                                }
                                if brief_desc_value.is_none() {
                                    brief_desc_value = extract_brief_desc(result);
                                }
                            }
                        }
                        let description = format!(
                            "封装: {} | 制造商: {} | 描述: {}",
                            package_value.clone().unwrap_or_else(|| "未知".to_string()),
                            manufacturer_value.clone().unwrap_or_else(|| "未知".to_string()),
                            brief_desc_value.unwrap_or_else(|| "未知".to_string())
                        );

                        results.push(SearchResult {
                            id,
                            name,
                            description,
                            package: package_value,
                            manufacturer: manufacturer_value,
                            category: None,
                            price: None,
                            stock: None,
                            image_url: None,
                        });
                    }
                }
            }
        }

        Ok(results)
    }

    pub async fn get_component_data(&self, component_id: &str) -> Result<ComponentData, JlcError> {
        let path = format!("/api/products/{}/svgs", component_id);
        let text = self.easyeda_get_text_path(&path).await?;
        let data: ComponentData = serde_json::from_str(&text)?;
        if !data.success {
            return Err(JlcError::ApiError(format!(
                "Failed to get component {} data",
                component_id
            )));
        }
        Ok(data)
    }

    pub async fn get_footprint_data(
        &self,
        footprint_uuid: &str,
    ) -> Result<FootprintApiResponse, JlcError> {
        let path = format!("/api/components/{}", footprint_uuid);
        let text = self.easyeda_get_text_path(&path).await?;
        let data: FootprintApiResponse = serde_json::from_str(&text)?;
        if !data.success {
            return Err(JlcError::ApiError(format!(
                "Failed to get footprint {} data",
                footprint_uuid
            )));
        }
        Ok(data)
    }

    pub async fn get_symbol_data(&self, symbol_uuid: &str) -> Result<SymbolApiResponse, JlcError> {
        let path = format!("/api/components/{}", symbol_uuid);
        let text = self.easyeda_get_text_path(&path).await?;
        let data: SymbolApiResponse = serde_json::from_str(&text)?;
        if !data.success {
            return Err(JlcError::ApiError(format!(
                "Failed to get symbol {} data",
                symbol_uuid
            )));
        }
        Ok(data)
    }

    pub async fn download_step_model(
        &self,
        component_uuid: &str,
        output_path: &str,
    ) -> Result<(), JlcError> {
        let path = format!("/qAxj6KHrDKw4blvCG8QJPs7Y/{}", component_uuid);
        let content = self
            .easyeda_get_bytes_with_bases(&MODEL_BASE_URLS, &path)
            .await?;
        if !content.is_empty() {
            let mut file = File::create(output_path)?;
            file.write_all(&content)?;
            Ok(())
        } else {
            Err(JlcError::ApiError("Failed to download STEP model: empty response".to_string()))
        }
    }

    pub async fn get_wrl_model(&self, component_uuid: &str) -> Result<String, JlcError> {
        let path = format!("/analyzer/api/3dmodel/{}", component_uuid);
        self.easyeda_get_text_path(&path).await
    }

    pub async fn resolve_step_uuid_via_pro_api(
        &self,
        component_id: &str,
    ) -> Result<Option<String>, JlcError> {
        let code = component_id.trim();
        if !code.to_uppercase().starts_with('C') {
            return Ok(None);
        }

        let by_codes = self
            .easyeda_post_form_json("/api/v2/devices/searchByCodes", &[("codes[]", code.to_string())])
            .await?;

        let device_uuid = by_codes
            .get("result")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("uuid"))
            .and_then(|v| v.as_str());

        let Some(device_uuid) = device_uuid else {
            return Ok(None);
        };

        let device_text = self
            .easyeda_get_text_pro_path(&format!("/api/devices/{}", device_uuid))
            .await?;
        let device_json: serde_json::Value = serde_json::from_str(&device_text)?;

        let model_uuid = device_json
            .get("result")
            .and_then(|v| v.get("attributes"))
            .and_then(|v| v.get("3D Model"))
            .and_then(|v| v.as_str())
            .map(uuid_first_part);

        let Some(model_uuid) = model_uuid else {
            return Ok(None);
        };

        let model_text = self
            .easyeda_get_text_pro_path(&format!("/api/v2/components/{}", model_uuid))
            .await?;
        let model_json: serde_json::Value = serde_json::from_str(&model_text)?;

        let direct_uuid = model_json
            .get("result")
            .and_then(|v| v.get("dataStr"))
            .and_then(|v| v.as_str())
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .and_then(|v| v.get("model").and_then(|m| m.as_str()).map(|m| m.to_string()));

        Ok(direct_uuid.or(Some(model_uuid)))
    }
}

pub async fn create_component(
    component_id: &str,
    output_dir: &str,
    footprint_lib: &str,
    symbol_lib: &str,
    symbol_path: &str,
    model_dir: &str,
    models: Vec<String>,
    create_footprint: bool,
    create_symbol: bool,
) -> Result<String, JlcError> {
    let client = JlcClient::new();

    // Get component UUIDs from EasyEDA
    let component_data = client.get_component_data(component_id).await?;
    
    if component_data.result.is_empty() {
        return Err(JlcError::ApiError(format!(
            "No results found for component {}",
            component_id
        )));
    }

    let footprint_uuid = &component_data.result.last().unwrap().component_uuid;
    let symbol_uuids: Vec<String> = component_data.result[..component_data.result.len() - 1]
        .iter()
        .map(|r| r.component_uuid.clone())
        .collect();

    let mut footprint_name = String::new();
    let mut datasheet_link = String::new();
    let mut step_model_downloaded = false;
    let mut step_model_error: Option<String> = None;

    // Download 3D model if requested, even without creating footprint
    if !models.is_empty() && !create_footprint && !create_symbol {
        // User only wants 3D model, need to get footprint data
        let fp_data = client.get_footprint_data(footprint_uuid).await?;
        footprint_name = fp_data.result.title.replace(" ", "_")
            .replace("/", "_")
            .replace("(", "_")
            .replace(")", "_");
        
        // Download STEP model using the same chain as Python plugins:
        // searchByCodes -> devices/{uuid} -> components/{3DModelUuid} -> dataStr.model
        if models.contains(&"STEP".to_string()) {
            let step_dir = PathBuf::from(output_dir)
                .join(footprint_lib)
                .join(model_dir);
            fs::create_dir_all(&step_dir)?;
            
            let step_path = step_dir.join(format!("{}.step", footprint_name));
            let mut model_candidates: Vec<String> = Vec::new();
            if let Ok(Some(uuid)) = client.resolve_step_uuid_via_pro_api(component_id).await {
                model_candidates.push(uuid);
            }
            if let Some(uuid) = extract_model_uuid_from_shape(&fp_data.result.data_str.shape) {
                model_candidates.push(uuid);
            }
            model_candidates.push(footprint_uuid.to_string());
            model_candidates.dedup();

            let mut last_error: Option<String> = None;
            for model_uuid in model_candidates {
                match client
                    .download_step_model(&model_uuid, step_path.to_str().unwrap())
                    .await
                {
                    Ok(_) => {
                        step_model_downloaded = true;
                        log::info!("Downloaded STEP model to {:?}", step_path);
                        break;
                    }
                    Err(e) => {
                        last_error = Some(format!(
                            "3D 模型下载失败（模型UUID: {}）: {}",
                            model_uuid, e
                        ));
                    }
                }
            }

            if !step_model_downloaded {
                step_model_error = last_error;
            }
        }
    }

    if !create_footprint
        && !create_symbol
        && models.contains(&"STEP".to_string())
        && !step_model_downloaded
    {
        return Err(JlcError::ApiError(
            step_model_error
                .clone()
                .unwrap_or_else(|| "3D 模型下载失败".to_string()),
        ));
    }

    // Create footprint
    if create_footprint {
        let result = create_footprint_internal(
            &client,
            footprint_uuid,
            component_id,
            output_dir,
            footprint_lib,
            model_dir,
            &models,
        )
        .await?;
        footprint_name = result.0;
        datasheet_link = result.1;
        step_model_downloaded |= result.2;
        if step_model_error.is_none() {
            step_model_error = result.3;
        }
    } else if create_symbol && footprint_name.is_empty() {
        // Still need to get footprint info for symbol
        let fp_data = client.get_footprint_data(footprint_uuid).await?;
        footprint_name = fp_data.result.title.replace(" ", "_")
            .replace("/", "_")
            .replace("(", "_")
            .replace(")", "_");
        datasheet_link = fp_data.result.data_str.head.c_para
            .and_then(|c| c.link)
            .unwrap_or_default();
    }

    // Create symbol
    if create_symbol && !symbol_uuids.is_empty() {
        create_symbol_internal(
            &client,
            &symbol_uuids,
            &footprint_name,
            &datasheet_link,
            component_id,
            output_dir,
            symbol_lib,
            symbol_path,
        )
        .await?;
    }

    let model_status = if step_model_downloaded {
        "downloaded"
    } else if !models.is_empty() {
        "failed"
    } else {
        "skipped"
    };
    let model_error_line = if model_status == "failed" {
        step_model_error
            .map(|e| format!("\n3D Error: {}", e))
            .unwrap_or_default()
    } else {
        String::new()
    };

    Ok(format!(
        "Component {} created successfully!\nFootprint: {}\nSymbol: {}\n3D Model: {}{}",
        component_id,
        if create_footprint { "created" } else { "skipped" },
        if create_symbol { "created" } else { "skipped" },
        model_status,
        model_error_line
    ))
}

async fn download_step_only_online(
    component_id: &str,
    model_name: &str,
    output_dir: &str,
    footprint_lib: &str,
    model_dir: &str,
) -> Result<(), JlcError> {
    let client = JlcClient::new();
    let step_uuid = client
        .resolve_step_uuid_via_pro_api(component_id)
        .await?
        .ok_or_else(|| JlcError::ApiError("未获取到3D模型UUID".to_string()))?;

    let step_dir = PathBuf::from(output_dir).join(footprint_lib).join(model_dir);
    fs::create_dir_all(&step_dir)?;
    let preferred = sanitize_footprint_name(model_name);
    let fallback = sanitize_footprint_name(component_id);
    let file_base = if preferred.is_empty() { fallback } else { preferred };
    let step_path = step_dir.join(format!("{}.step", file_base));
    client
        .download_step_model(&step_uuid, step_path.to_string_lossy().as_ref())
        .await
}

fn get_symbol_data_by_uuid<'a>(bundle: &'a OfflineBundle, symbol_uuid: &str) -> Option<&'a String> {
    if let Some(v) = bundle.symbol_data.get(symbol_uuid) {
        return Some(v);
    }
    let short = uuid_first_part(symbol_uuid);
    if let Some(v) = bundle.symbol_data.get(&short) {
        return Some(v);
    }
    bundle.symbol_data.iter().find_map(|(k, v)| {
        if uuid_first_part(k) == short {
            Some(v)
        } else {
            None
        }
    })
}

fn get_footprint_title_by_uuid(bundle: &OfflineBundle, footprint_uuid: &str) -> Option<String> {
    if let Some(v) = bundle.footprint_titles.get(footprint_uuid) {
        return Some(v.clone());
    }
    let short = uuid_first_part(footprint_uuid);
    if let Some(v) = bundle.footprint_titles.get(&short) {
        return Some(v.clone());
    }
    bundle.footprint_titles.iter().find_map(|(k, v)| {
        if uuid_first_part(k) == short {
            Some(v.clone())
        } else {
            None
        }
    })
}

pub async fn search_components(query: &str) -> Result<Vec<SearchResult>, JlcError> {
    let client = JlcClient::new();
    match client.search_components(query).await {
        Ok(results) if !results.is_empty() => Ok(results),
        Ok(_) | Err(_) => search_lcsc(query).await,
    }
}

pub async fn search_easyeda(query: &str) -> Result<Vec<SearchResult>, JlcError> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Err(JlcError::ApiError("请输入搜索关键字".to_string()));
    }

    let client = JlcClient::new();
    match client.search_easyeda_pro(trimmed).await {
        Ok(results) if !results.is_empty() => Ok(results),
        Ok(_) => {
            // Fallback to legacy endpoint for C-code lookups.
            if trimmed.to_uppercase().starts_with('C') {
                if let Ok(results) = client.search_components(trimmed).await {
                    if !results.is_empty() {
                        return Ok(results);
                    }
                }
            }
            Err(JlcError::ApiError(format!("EasyEDA 未找到元件 {}", trimmed)))
        }
        Err(JlcError::RequestError(e)) => {
            // pro.easyeda may be blocked/unreachable in some networks, retry legacy endpoint.
            if trimmed.to_uppercase().starts_with('C') {
                if let Ok(results) = client.search_components(trimmed).await {
                    if !results.is_empty() {
                        return Ok(results);
                    }
                }
            }
            Err(JlcError::ApiError(format!(
                "无法连接 EasyEDA（{}）。已尝试 pro.easyeda 与旧接口，请检查网络链路或代理策略",
                e
            )))
        }
        Err(e) => Err(e),
    }
}

pub async fn search_lcsc(query: &str) -> Result<Vec<SearchResult>, JlcError> {
    let client = JlcClient::new();

    // 1) Same method as python plugin easyeda_lib_loader.py:
    // POST /api/v2/devices/search with uid/path = "lcsc"
    if let Ok(found) = client
        .easyeda_post_form_json(
            "/api/v2/devices/search",
            &[
                ("page", "1".to_string()),
                ("pageSize", "50".to_string()),
                ("wd", query.to_string()),
                ("returnListStyle", "classifyarr".to_string()),
                ("uid", "lcsc".to_string()),
                ("path", "lcsc".to_string()),
            ],
        )
        .await
    {
        if found
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            let mut results = Vec::new();
            if let Some(lists) = found
                .get("result")
                .and_then(|v| v.get("lists"))
                .and_then(|v| v.as_object())
            {
                for group in lists.values() {
                    if let Some(items) = group.as_array() {
                        for item in items.iter().take(50) {
                            let id = first_non_empty_str(
                                item,
                                &["product_code", "productCode", "code", "uuid"],
                            )
                            .unwrap_or_default();
                            if id.is_empty() {
                                continue;
                            }

                            let name = first_non_empty_str(
                                item,
                                &["display_title", "title", "name", "product_name"],
                            )
                            .unwrap_or_else(|| id.clone());

                            let manufacturer =
                                extract_manufacturer_name(item).unwrap_or_else(|| "未知".to_string());
                            let package =
                                extract_package_name(item).unwrap_or_else(|| "未知".to_string());
                            let brief_desc =
                                extract_brief_desc(item).unwrap_or_else(|| "未知".to_string());

                            results.push(SearchResult {
                                id,
                                name,
                                description: format!(
                                    "封装: {} | 制造商: {} | 描述: {} | 来源: EasyEDA-LCSC",
                                    package, manufacturer, brief_desc
                                ),
                                package: Some(package),
                                manufacturer: Some(manufacturer),
                                category: None,
                                price: None,
                                stock: None,
                                image_url: None,
                            });
                        }
                    }
                }
            }

            if !results.is_empty() {
                return Ok(results);
            }
        }
    }

    // Try public search endpoint used by community tools.
    // Example payload keys: productSearchResultVO.productList[].productCode
    let public_resp = client
        .lcsc_client
        .get("https://wwwapi.lcsc.com/v1/search/global-search")
        .query(&[("keyword", query)])
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36",
        )
        .header(reqwest::header::REFERER, "https://www.lcsc.com/")
        .send()
        .await;

    if let Ok(resp) = public_resp {
        if resp.status().is_success() {
            let text = resp.text().await?;
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
                let list = data
                    .get("productSearchResultVO")
                    .and_then(|v| v.get("productList"))
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();

                let mut results = Vec::new();
                for product in list.iter().take(20) {
                    let id = first_non_empty_str(
                        product,
                        &[
                            "productCode",
                            "product_code",
                            "lcscPartNumber",
                            "partNumber",
                            "productModel",
                        ],
                    )
                    .unwrap_or_default();
                    if id.is_empty() {
                        continue;
                    }

                    let name = first_non_empty_str(
                        product,
                        &[
                            "productModel",
                            "productNameEn",
                            "productName",
                            "productDescEn",
                            "productIntroEn",
                        ],
                    )
                    .unwrap_or_else(|| id.clone());

                    let mut details = Vec::new();
                    if let Some(v) = first_non_empty_str(product, &["brandNameEn", "brandName"]) {
                        details.push(format!("制造商: {}", v));
                    }
                    if let Some(v) = first_non_empty_str(
                        product,
                        &["encap", "encapsulation", "packageType", "package"],
                    ) {
                        details.push(format!("封装: {}", v));
                    }
                    if let Some(v) = first_non_empty_str(product, &["stockNumber", "stock"]) {
                        details.push(format!("库存: {}", v));
                    }
                    if let Some(v) = first_non_empty_str(
                        product,
                        &["productDescEn", "productDesc", "productIntroEn", "description"],
                    ) {
                        details.push(format!("描述: {}", v));
                    }

                    results.push(SearchResult {
                        id: id.clone(),
                        name,
                        description: if details.is_empty() {
                            "LCSC Public Search".to_string()
                        } else {
                            details.join(" | ")
                        },
                        package: None,
                        manufacturer: None,
                        category: None,
                        price: None,
                        stock: None,
                        image_url: Some(format!("https://wmsc.lcsc.com/wmsc/upload/file/eec/image/{}.jpg", id)),
                    });
                }

                if !results.is_empty() {
                    return Ok(results);
                }
            }
        }
    }

    // Legacy endpoint fallback.
    let legacy_resp = client
        .lcsc_client
        .get("https://wmsc.lcsc.com/wmsc/product/detail")
        .query(&[("search", query)])
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36",
        )
        .header(reqwest::header::REFERER, "https://www.lcsc.com/")
        .send()
        .await;

    if let Ok(resp) = legacy_resp {
        if resp.status().is_success() {
            let text = resp.text().await?;
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
                let mut results = Vec::new();
                if let Some(products) = data.get("products").and_then(|p| p.as_array()) {
                    for product in products.iter().take(20) {
                        let id = first_non_empty_str(
                            product,
                            &["product_code", "productCode", "mfr_part", "part_number"],
                        )
                        .unwrap_or_default();
                        if id.is_empty() {
                            continue;
                        }

                        let name = first_non_empty_str(
                            product,
                            &["product_name", "description", "productName"],
                        )
                        .unwrap_or_else(|| id.clone());

                        let package_value = first_non_empty_str(
                            product,
                            &["package", "encapsulation", "encap"],
                        );
                        let manufacturer_value = first_non_empty_str(
                            product,
                            &["manufacturer", "brand", "mfr"],
                        );
                        let brief_desc = first_non_empty_str(
                            product,
                            &["description", "product_name", "description_en"],
                        );
                        let description = format!(
                            "封装: {} | 制造商: {} | 描述: {}",
                            package_value.clone().unwrap_or_else(|| "未知".to_string()),
                            manufacturer_value.clone().unwrap_or_else(|| "未知".to_string()),
                            brief_desc.unwrap_or_else(|| "未知".to_string())
                        );

                        results.push(SearchResult {
                            id: id.clone(),
                            name,
                            description,
                            package: package_value,
                            manufacturer: manufacturer_value,
                            category: None,
                            price: None,
                            stock: None,
                            image_url: Some(format!("https://wmsc.lcsc.com/wmsc/upload/file/eec/image/{}.jpg", id)),
                        });
                    }
                }

                if !results.is_empty() {
                    return Ok(results);
                }
            }
        }
    }

    Err(JlcError::ApiError(
        "立创商城公开搜索受限；官方API需申请 key/secret 并签名调用（详见 LCSC API 文档）。请使用 EasyEDA、配置官方API，或改用本地文件".to_string(),
    ))
}

pub async fn import_local_model_for_component(
    component_id: &str,
    model_path: &str,
    output_dir: &str,
    footprint_lib: &str,
    model_dir: &str,
) -> Result<String, JlcError> {
    let src_path = PathBuf::from(model_path);
    if !src_path.exists() || !src_path.is_file() {
        return Err(JlcError::ApiError("本地3D模型文件不存在".to_string()));
    }

    let ext = src_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext != "step" && ext != "stp" && ext != "wrl" {
        return Err(JlcError::ApiError(
            "仅支持 STEP/STP/WRL 格式的3D模型".to_string(),
        ));
    }

    let client = JlcClient::new();
    let component_data = client.get_component_data(component_id).await?;
    if component_data.result.is_empty() {
        return Err(JlcError::ApiError(format!("未找到元件 {}", component_id)));
    }

    let footprint_uuid = &component_data.result.last().unwrap().component_uuid;
    let fp_data = client.get_footprint_data(footprint_uuid).await?;
    let footprint_name = sanitize_footprint_name(&fp_data.result.title);

    let normalized_ext = if ext == "stp" { "step" } else { &ext };
    let dest_dir = PathBuf::from(output_dir).join(footprint_lib).join(model_dir);
    fs::create_dir_all(&dest_dir)?;
    let dest_path = dest_dir.join(format!("{}.{}", footprint_name, normalized_ext));
    fs::copy(&src_path, &dest_path)?;

    // If footprint already exists, inject/replace model reference automatically.
    let footprint_path = PathBuf::from(output_dir)
        .join(footprint_lib)
        .join(format!("{}.kicad_mod", footprint_name));
    if footprint_path.exists() {
        let mut content = fs::read_to_string(&footprint_path)?;
        let model_ref = format!("{}/{}.{}", model_dir, footprint_name, normalized_ext);
        let model_line = format!(
            "  (model {} (at (xyz 0 0 0)) (rotate (xyz 0 0 0)))\n",
            model_ref
        );

        if !content.contains(&format!("(model {}", model_ref)) {
            // remove old model lines to avoid duplicates
            let filtered: String = content
                .lines()
                .filter(|l| !l.trim_start().starts_with("(model "))
                .map(|l| format!("{}\n", l))
                .collect();
            content = filtered;

            if let Some(pos) = content.rfind("\n  )\n)\n") {
                content.insert_str(pos + 1, &model_line);
            } else if let Some(pos) = content.rfind("\n)\n") {
                content.insert_str(pos + 1, &model_line);
            } else {
                content.push_str(&model_line);
            }
            fs::write(&footprint_path, content)?;
        }
    }

    Ok(format!(
        "本地3D模型已导入: {}\n目标路径: {}",
        component_id,
        dest_path.to_string_lossy()
    ))
}

fn component_id_regex() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"\bC\d{3,}\b").unwrap())
}

fn looks_like_hex_uuid(value: &str) -> bool {
    let s = value.trim();
    if s.len() == 32 {
        return s.chars().all(|c| c.is_ascii_hexdigit());
    }
    if s.len() == 36 {
        let bytes = s.as_bytes();
        return bytes.get(8) == Some(&b'-')
            && bytes.get(13) == Some(&b'-')
            && bytes.get(18) == Some(&b'-')
            && bytes.get(23) == Some(&b'-')
            && s.chars()
                .enumerate()
                .all(|(i, ch)| matches!(i, 8 | 13 | 18 | 23) || ch.is_ascii_hexdigit());
    }
    false
}

fn normalize_component_token(value: &str) -> Option<String> {
    let token = value.trim().trim_matches('"').trim_matches('\'').trim();
    if token.is_empty() {
        return None;
    }

    let upper = token.to_uppercase();
    if upper.starts_with('C')
        && upper.len() > 1
        && upper[1..].chars().all(|c| c.is_ascii_digit())
    {
        return Some(upper);
    }

    if looks_like_hex_uuid(token) {
        return Some(token.to_string());
    }

    None
}

fn extract_component_ids_from_text(content: &str, ids: &mut HashSet<String>) {
    for m in component_id_regex().find_iter(content) {
        if let Some(id) = normalize_component_token(m.as_str()) {
            ids.insert(id);
        }
    }
}

fn extract_component_ids_from_json_value(value: &serde_json::Value, ids: &mut HashSet<String>) {
    match value {
        serde_json::Value::String(s) => {
            if let Some(id) = normalize_component_token(s) {
                ids.insert(id);
            }
            extract_component_ids_from_text(s, ids);
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                extract_component_ids_from_json_value(item, ids);
            }
        }
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let key = k.to_lowercase();
                if [
                    "component_id",
                    "lcsc",
                    "product_code",
                    "productcode",
                    "code",
                    "partnumber",
                    "part_number",
                    "id",
                    "uuid",
                    "component_uuid",
                ]
                .contains(&key.as_str())
                {
                    if let Some(s) = v.as_str() {
                        if let Some(id) = normalize_component_token(s) {
                            ids.insert(id);
                        }
                    }
                }
                extract_component_ids_from_json_value(v, ids);
            }
        }
        _ => {}
    }
}

fn gather_input_files(path: &Path) -> Result<Vec<PathBuf>, JlcError> {
    if !path.exists() {
        return Err(JlcError::ApiError("路径不存在".to_string()));
    }

    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }

    if !path.is_dir() {
        return Err(JlcError::ApiError("无效路径".to_string()));
    }

    let mut files = Vec::new();
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.is_file() {
                files.push(p);
            }
        }
    }
    Ok(files)
}

fn detect_local_bundle_kind(path: &Path) -> String {
    if path.is_file() {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if ext == "elibz2" {
            return "elibz2".to_string();
        }
        if ext == "elibz" {
            return "elibz".to_string();
        }
    }

    if let Ok(files) = gather_input_files(path) {
        if files.iter().any(|f| {
            f.extension()
                .and_then(|e| e.to_str())
                .map(|s| s.eq_ignore_ascii_case("elibz2"))
                .unwrap_or(false)
        }) {
            return "elibz2".to_string();
        }
        if files.iter().any(|f| {
            f.extension()
                .and_then(|e| e.to_str())
                .map(|s| s.eq_ignore_ascii_case("elibz"))
                .unwrap_or(false)
        }) {
            return "elibz".to_string();
        }
    }

    "elibz".to_string()
}

fn extract_component_ids_from_elibz(path: &Path) -> Result<HashSet<String>, JlcError> {
    let mut ids = HashSet::new();
    let file = File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| JlcError::ApiError(format!("无法解析库文件 {}: {}", path.display(), e)))?;

    let mut content = String::new();
    let mut found = false;
    {
        if let Ok(mut f) = archive.by_name("device.json") {
            f.read_to_string(&mut content)?;
            found = true;
        }
    }
    if !found {
        if let Ok(mut f) = archive.by_name("device2.json") {
            f.read_to_string(&mut content)?;
            found = true;
        }
    }
    if found {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            extract_component_ids_from_json_value(&json, &mut ids);
        }
    }

    Ok(ids)
}

fn extract_data_str_from_component_blob(content: &str) -> Option<String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(ds) = v.get("dataStr") {
            if let Some(s) = ds.as_str() {
                return Some(s.to_string());
            }
            if ds.is_object() || ds.is_array() {
                return Some(ds.to_string());
            }
        }

        if v.get("shape").is_some() {
            return Some(v.to_string());
        }
    }

    Some(trimmed.to_string())
}

fn parse_elibz_components(path: &Path) -> Result<BTreeMap<String, SearchResult>, JlcError> {
    let mut out = BTreeMap::new();
    let file = File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| JlcError::ApiError(format!("无法解析库文件 {}: {}", path.display(), e)))?;

    let mut content = String::new();
    let mut found = false;
    {
        if let Ok(mut f) = archive.by_name("device.json") {
            f.read_to_string(&mut content)?;
            found = true;
        }
    }
    if !found {
        if let Ok(mut f) = archive.by_name("device2.json") {
            f.read_to_string(&mut content)?;
            found = true;
        }
    }
    if !found {
        return Ok(out);
    }

    let json: serde_json::Value = serde_json::from_str(&content)?;
    let mut footprint_titles: BTreeMap<String, String> = BTreeMap::new();
    if let Some(footprints) = json.get("footprints").and_then(|v| v.as_object()) {
        for (uuid, fp) in footprints {
            if let Some(title) = first_non_empty_str(fp, &["title", "display_title", "name"]) {
                footprint_titles.insert(uuid.clone(), title);
            }
        }
    }
    if let Ok(mut fp_manifest) = archive.by_name("footprint.json") {
        let mut fp_content = String::new();
        fp_manifest.read_to_string(&mut fp_content)?;
        if let Ok(fp_json) = serde_json::from_str::<serde_json::Value>(&fp_content) {
            if let Some(obj) = fp_json.as_object() {
                for (uuid, fp) in obj {
                    if let Some(title) = first_non_empty_str(
                        fp,
                        &["title", "display_title", "displayTitle", "name"],
                    ) {
                        footprint_titles.entry(uuid.clone()).or_insert(title);
                    }
                }
            }
        }
    }
    let devices = json
        .get("devices")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    for (_, device) in devices {
        let id = extract_preferred_local_id(&device).unwrap_or_default();
        if id.is_empty() {
            continue;
        }

        let raw_name = first_non_empty_str(&device, &["display_title", "title", "name"]);
        let attrs = device.get("attributes").unwrap_or(&device);
        // Match python plugin behavior: prefer device.footprint.display_title.
        let package_from_device_footprint = device
            .get("footprint")
            .and_then(|fp| {
                first_non_empty_str(
                    fp,
                    &["display_title", "displayTitle", "title", "name", "package_name"],
                )
            });
        let package_from_uuid_map = attrs
            .get("Footprint")
            .and_then(|v| v.as_str())
            .and_then(|s| split_uuid_first(Some(s)))
            .and_then(|uuid| footprint_titles.get(&uuid).cloned());
        let package = package_from_device_footprint
            .or(package_from_uuid_map)
            .or_else(|| extract_package_name(&device))
            .or_else(|| extract_package_name(attrs))
            .unwrap_or_else(|| "未知".to_string());
        let name = normalize_display_name(raw_name, &id, Some(&package));
        let manufacturer = extract_manufacturer_name(&device)
            .or_else(|| extract_manufacturer_name(attrs))
            .unwrap_or_else(|| "未知".to_string());
        let brief_desc = extract_brief_desc(&device)
            .or_else(|| extract_brief_desc(attrs))
            .unwrap_or_else(|| "未知".to_string());
        let desc = format!(
            "封装: {} | 制造商: {} | 描述: {}",
            package,
            manufacturer,
            brief_desc
        );

        out.insert(
            id.clone(),
            SearchResult {
                id,
                name,
                description: format!("{} | 来源: {}", desc, path.to_string_lossy()),
                package: None,
                manufacturer: None,
                category: None,
                price: None,
                stock: None,
                image_url: None,
            },
        );
    }

    Ok(out)
}

fn extract_component_ids_from_file(path: &Path) -> HashSet<String> {
    let mut ids = HashSet::new();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "json" => {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    extract_component_ids_from_json_value(&json, &mut ids);
                }
                extract_component_ids_from_text(&content, &mut ids);
            }
        }
        "txt" | "csv" | "tsv" | "list" | "eda" | "lcsc" => {
            if let Ok(content) = fs::read_to_string(path) {
                extract_component_ids_from_text(&content, &mut ids);
            }
            if ids.is_empty() {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Some(id) = normalize_component_token(stem) {
                        ids.insert(id);
                    }
                }
            }
        }
        "elibz" | "elibz2" => {
            if let Ok(found) = extract_component_ids_from_elibz(path) {
                ids.extend(found);
            }
        }
        _ => {}
    }

    ids
}

fn collect_local_component_map(path: &Path) -> Result<BTreeMap<String, SearchResult>, JlcError> {
    let files = gather_input_files(path)?;
    let mut map: BTreeMap<String, SearchResult> = BTreeMap::new();
    let mut ids = HashSet::new();

    for file in files {
        let ext = file
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext == "elibz" || ext == "elibz2" {
            if let Ok(comps) = parse_elibz_components(&file) {
                for (k, v) in comps {
                    ids.insert(k.clone());
                    map.entry(k)
                        .and_modify(|old| {
                            let looks_placeholder =
                                old.name == old.id || old.description.starts_with("本地文件:");
                            if looks_placeholder {
                                *old = v.clone();
                            }
                        })
                        .or_insert(v);
                }
            }
            continue;
        }

        let found = extract_component_ids_from_file(&file);
        for id in found {
            ids.insert(id.clone());
            map.entry(id.clone()).or_insert(SearchResult {
                id: id.clone(),
                name: id.clone(),
                description: format!("本地文件: {}", file.to_string_lossy()),
                package: None,
                manufacturer: None,
                category: None,
                price: None,
                stock: None,
                image_url: None,
            });
        }
    }

    // Prefer C-code IDs for conversion; keep UUID only when no C-code found.
    let has_c = ids.iter().any(|id| id.to_uppercase().starts_with('C'));
    if has_c {
        map.retain(|k, _| k.to_uppercase().starts_with('C'));
    }

    if map.is_empty() {
        return Err(JlcError::ApiError(
            "未找到可转换的元件编号（支持 C编号/UUID，文件支持 json/txt/csv/eda/lcsc/elibz/elibz2）"
                .to_string(),
        ));
    }

    Ok(map)
}

fn collect_component_ids_from_path(path: &Path) -> Result<HashSet<String>, JlcError> {
    let map = collect_local_component_map(path)?;
    Ok(map.keys().cloned().collect())
}

#[derive(Debug, Clone)]
struct OfflineDevice {
    id: String,
    name: String,
    footprint_uuid: Option<String>,
    symbol_uuids: Vec<String>,
    model_title: Option<String>,
}

#[derive(Debug, Default)]
struct OfflineBundle {
    devices: BTreeMap<String, OfflineDevice>,
    footprint_data: BTreeMap<String, String>,
    symbol_data: BTreeMap<String, String>,
    footprint_titles: BTreeMap<String, String>,
    symbol_titles: BTreeMap<String, String>,
    symbol_prefix: BTreeMap<String, String>,
}

fn split_uuid_first(value: Option<&str>) -> Option<String> {
    value
        .map(|s| s.split('|').next().unwrap_or(s).trim().to_string())
        .filter(|s| !s.is_empty())
}

#[derive(Debug, Clone, Default)]
struct ElibuSymbolPin {
    x: f64,
    y: f64,
    rotation: f64,
    pin_num: String,
    pin_name: String,
    pin_type: String,
}

#[derive(Debug, Default)]
struct ElibuDocAccumulator {
    doc_type: String,
    uuid: String,
    lines: Vec<String>,
    pins: BTreeMap<String, ElibuSymbolPin>,
}

fn json_num(v: Option<&serde_json::Value>) -> Option<f64> {
    match v {
        Some(serde_json::Value::Number(n)) => n.as_f64(),
        Some(serde_json::Value::String(s)) => s.parse::<f64>().ok(),
        _ => None,
    }
}

fn json_str(v: Option<&serde_json::Value>) -> Option<String> {
    match v {
        Some(serde_json::Value::String(s)) => Some(s.trim().to_string()),
        Some(serde_json::Value::Number(n)) => Some(n.to_string()),
        _ => None,
    }
}

fn elibu_pin_type_to_code(pin_type: &str) -> &'static str {
    match pin_type.to_lowercase().as_str() {
        "input" => "1",
        "output" => "2",
        "bidirectional" => "3",
        "power" | "power_in" => "4",
        _ => "0",
    }
}

fn extract_polyline_numbers(path: &serde_json::Value) -> Vec<f64> {
    let mut out = Vec::new();
    let Some(arr) = path.as_array() else {
        return out;
    };
    for item in arr {
        match item {
            serde_json::Value::Number(n) => {
                if let Some(v) = n.as_f64() {
                    out.push(v);
                }
            }
            serde_json::Value::String(s) => {
                if let Ok(v) = s.parse::<f64>() {
                    out.push(v);
                }
            }
            _ => {}
        }
    }
    out
}

fn flush_elibu_doc(acc: &mut ElibuDocAccumulator, bundle: &mut OfflineBundle) {
    if acc.uuid.is_empty() {
        acc.doc_type.clear();
        acc.lines.clear();
        acc.pins.clear();
        return;
    }

    if acc.doc_type.eq_ignore_ascii_case("SYMBOL") {
        for pin in acc.pins.values() {
            let pin_num = if pin.pin_num.is_empty() {
                "0".to_string()
            } else {
                pin.pin_num.clone()
            };
            let line = format!(
                "P~1~{}~{}~{}~{}~{}~0~0~0~0~0~0~0~{}",
                elibu_pin_type_to_code(&pin.pin_type),
                pin_num,
                pin.x,
                pin.y,
                pin.rotation,
                pin.pin_name
            );
            acc.lines.push(line);
        }
        if !acc.lines.is_empty() {
            bundle
                .symbol_data
                .entry(acc.uuid.clone())
                .or_insert_with(|| acc.lines.join("\n"));
        }
    } else if acc.doc_type.eq_ignore_ascii_case("FOOTPRINT") {
        if !acc.lines.is_empty() {
            bundle
                .footprint_data
                .entry(acc.uuid.clone())
                .or_insert_with(|| acc.lines.join("\n"));
        }
    }

    acc.doc_type.clear();
    acc.uuid.clear();
    acc.lines.clear();
    acc.pins.clear();
}

fn parse_elibu_content(content: &str, bundle: &mut OfflineBundle) -> Result<(), JlcError> {
    let mut acc = ElibuDocAccumulator::default();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || !line.contains("||") {
            continue;
        }

        let mut seg = line.splitn(2, "||");
        let left = seg.next().unwrap_or("").trim().trim_end_matches('|');
        let right = seg.next().unwrap_or("").trim().trim_end_matches('|');
        if left.is_empty() {
            continue;
        }

        let header: serde_json::Value = match serde_json::from_str(left) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let event_type = header.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let event_id = header
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let payload: serde_json::Value = if right.is_empty() {
            serde_json::Value::Null
        } else {
            match serde_json::from_str(right) {
                Ok(v) => v,
                Err(_) => continue,
            }
        };

        if event_type == "DOCHEAD" {
            flush_elibu_doc(&mut acc, bundle);
            acc.doc_type = payload
                .get("docType")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            acc.uuid = payload
                .get("uuid")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            continue;
        }

        if acc.doc_type.eq_ignore_ascii_case("SYMBOL") {
            match event_type {
                "PIN" => {
                    let pin = ElibuSymbolPin {
                        x: json_num(payload.get("x")).unwrap_or(0.0),
                        y: json_num(payload.get("y")).unwrap_or(0.0),
                        rotation: json_num(payload.get("rotation")).unwrap_or(0.0),
                        ..Default::default()
                    };
                    if !event_id.is_empty() {
                        acc.pins.insert(event_id, pin);
                    }
                }
                "ATTR" => {
                    let parent_id = payload
                        .get("parentId")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let key = payload.get("key").and_then(|v| v.as_str()).unwrap_or("");
                    let value = payload.get("value").and_then(|v| v.as_str()).unwrap_or("");
                    if !parent_id.is_empty() {
                        if let Some(pin) = acc.pins.get_mut(&parent_id) {
                            match key {
                                "Pin Name" => pin.pin_name = value.to_string(),
                                "Pin Number" => pin.pin_num = value.to_string(),
                                "Pin Type" => pin.pin_type = value.to_string(),
                                _ => {}
                            }
                        }
                    }
                }
                "RECT" => {
                    let x1 = json_num(payload.get("dotX1")).unwrap_or(0.0);
                    let y1 = json_num(payload.get("dotY1")).unwrap_or(0.0);
                    let x2 = json_num(payload.get("dotX2")).unwrap_or(x1);
                    let y2 = json_num(payload.get("dotY2")).unwrap_or(y1);
                    acc.lines.push(format!(
                        "R~{}~{}~0~0~{}~{}",
                        x1,
                        y1,
                        x2 - x1,
                        y2 - y1
                    ));
                }
                "ELLIPSE" => {
                    let cx = json_num(payload.get("centerX")).unwrap_or(0.0);
                    let cy = json_num(payload.get("centerY")).unwrap_or(0.0);
                    let rx = json_num(payload.get("radiusX")).unwrap_or(0.0).abs();
                    let ry = json_num(payload.get("radiusY")).unwrap_or(0.0).abs();
                    let r = if rx > 0.0 { rx } else { ry };
                    acc.lines.push(format!("E~{}~{}~{}", cx, cy, r));
                }
                "POLY" => {
                    if let Some(path) = payload.get("path") {
                        let points = extract_polyline_numbers(path);
                        if points.len() >= 4 {
                            let pt = points
                                .chunks(2)
                                .filter_map(|xy| {
                                    if xy.len() == 2 {
                                        Some(format!("{} {}", xy[0], xy[1]))
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join(" ");
                            if !pt.is_empty() {
                                acc.lines.push(format!("PL~{}", pt));
                            }
                        }
                    }
                }
                _ => {}
            }
            continue;
        }

        if acc.doc_type.eq_ignore_ascii_case("FOOTPRINT") {
            match event_type {
                "PAD" => {
                    let default_pad = payload.get("defaultPad").unwrap_or(&payload);
                    let shape = default_pad
                        .get("padType")
                        .and_then(|v| v.as_str())
                        .unwrap_or("OVAL");
                    let x = json_num(payload.get("centerX")).unwrap_or(0.0);
                    let y = json_num(payload.get("centerY")).unwrap_or(0.0);
                    let sx = json_num(default_pad.get("width")).unwrap_or(1.0);
                    let sy = json_num(default_pad.get("height")).unwrap_or(1.0);
                    let mut layer = json_str(payload.get("layerId")).unwrap_or_else(|| "1".to_string());
                    let mut drill = 0.0_f64;
                    if let Some(hole) = payload.get("hole") {
                        if !hole.is_null() {
                            drill = json_num(hole.get("radius"))
                                .or_else(|| json_num(hole.get("diameter")).map(|d| d / 2.0))
                                .or_else(|| json_num(hole.get("width")).map(|d| d / 2.0))
                                .unwrap_or(0.0);
                            if drill > 0.0 {
                                layer = "11".to_string();
                            }
                        }
                    }
                    let pad_num =
                        json_str(payload.get("num")).unwrap_or_else(|| "1".to_string());
                    let rot = json_num(payload.get("padAngle"))
                        .or_else(|| json_num(payload.get("relativeAngle")))
                        .unwrap_or(0.0);
                    acc.lines.push(format!(
                        "PAD~{}~{}~{}~{}~{}~{}~0~{}~{}~0~{}",
                        shape, x, y, sx, sy, layer, pad_num, drill, rot
                    ));
                }
                "POLY" | "FILL" => {
                    let layer = json_str(payload.get("layerId")).unwrap_or_else(|| "3".to_string());
                    let width = json_num(payload.get("width")).unwrap_or(0.2);
                    if let Some(path) = payload.get("path") {
                        if let Some(arr) = path.as_array() {
                            if let Some(serde_json::Value::String(kind)) = arr.first() {
                                if kind == "CIRCLE" && arr.len() >= 4 {
                                    let cx = json_num(arr.get(1)).unwrap_or(0.0);
                                    let cy = json_num(arr.get(2)).unwrap_or(0.0);
                                    let r = json_num(arr.get(3)).unwrap_or(0.0).abs();
                                    acc.lines.push(format!(
                                        "CIRCLE~{}~{}~{}~{}~{}",
                                        cx, cy, r, width, layer
                                    ));
                                    continue;
                                }
                            }
                        }
                        let points = extract_polyline_numbers(path);
                        if points.len() >= 4 {
                            let pt = points
                                .chunks(2)
                                .filter_map(|xy| {
                                    if xy.len() == 2 {
                                        Some(format!("{} {}", xy[0], xy[1]))
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join(" ");
                            if !pt.is_empty() {
                                acc.lines
                                    .push(format!("TRACK~{}~{}~0~{}", width, layer, pt));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    flush_elibu_doc(&mut acc, bundle);
    Ok(())
}

fn load_offline_bundle_from_elibz(path: &Path) -> Result<OfflineBundle, JlcError> {
    let mut bundle = OfflineBundle::default();
    let file = File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| JlcError::ApiError(format!("无法解析库文件 {}: {}", path.display(), e)))?;

    let mut content = String::new();
    let mut found = false;
    {
        if let Ok(mut f) = archive.by_name("device.json") {
            f.read_to_string(&mut content)?;
            found = true;
        }
    }
    if !found {
        if let Ok(mut f) = archive.by_name("device2.json") {
            f.read_to_string(&mut content)?;
            found = true;
        }
    }
    if !found {
        return Ok(bundle);
    }

    {
        let json: serde_json::Value = serde_json::from_str(&content)?;

        if let Some(footprints) = json.get("footprints").and_then(|v| v.as_object()) {
            for (uuid, fp) in footprints {
                if let Some(title) = first_non_empty_str(fp, &["title", "display_title", "name"]) {
                    bundle.footprint_titles.insert(uuid.clone(), title);
                }
            }
        }
        if let Some(symbols) = json.get("symbols").and_then(|v| v.as_object()) {
            for (uuid, sym) in symbols {
                if let Some(title) = first_non_empty_str(sym, &["title", "display_title", "name"]) {
                    bundle.symbol_titles.insert(uuid.clone(), title);
                }

                if let Some(pre) = sym
                    .get("head")
                    .and_then(|h| h.get("c_para"))
                    .and_then(|c| c.get("pre"))
                    .and_then(|v| v.as_str())
                {
                    bundle.symbol_prefix.insert(uuid.clone(), pre.to_string());
                }
            }
        }

        if let Some(devices) = json.get("devices").and_then(|v| v.as_object()) {
            for (_, dev) in devices {
                let id = extract_preferred_local_id(dev).unwrap_or_default();
                if id.is_empty() {
                    continue;
                }

                let attrs = dev.get("attributes").unwrap_or(dev);
                let footprint_uuid = split_uuid_first(attrs.get("Footprint").and_then(|v| v.as_str()));
                let symbol_uuid = split_uuid_first(attrs.get("Symbol").and_then(|v| v.as_str()));
                let symbol_uuids = symbol_uuid.into_iter().collect();

                let name = first_non_empty_str(
                    dev,
                    &["display_title", "title", "name", "product_name"],
                )
                .unwrap_or_else(|| id.clone());

                let model_title = first_non_empty_str(attrs, &["3D Model Title", "Model Title"]);

                bundle.devices.insert(
                    id.clone(),
                    OfflineDevice {
                        id,
                        name,
                        footprint_uuid,
                        symbol_uuids,
                        model_title,
                    },
                );
            }
        }
    }

    for i in 0..archive.len() {
        let mut f = archive
            .by_index(i)
            .map_err(|e| JlcError::ApiError(format!("读取库文件失败: {}", e)))?;
        let name = f.name().to_string();
        if name.ends_with(".efoo") {
            let uuid = Path::new(&name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            if !uuid.is_empty() {
                let mut ds = String::new();
                f.read_to_string(&mut ds)?;
                if let Some(normalized) = extract_data_str_from_component_blob(&ds) {
                    bundle.footprint_data.insert(uuid, normalized);
                }
            }
        } else if name.ends_with(".esym") {
            let uuid = Path::new(&name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            if !uuid.is_empty() {
                let mut ds = String::new();
                f.read_to_string(&mut ds)?;
                if let Some(normalized) = extract_data_str_from_component_blob(&ds) {
                    bundle.symbol_data.insert(uuid, normalized);
                }
            }
        }
    }

    if bundle.footprint_data.is_empty() || bundle.symbol_data.is_empty() {
        for i in 0..archive.len() {
            let mut f = archive
                .by_index(i)
                .map_err(|e| JlcError::ApiError(format!("读取库文件失败: {}", e)))?;
            let name = f.name().to_string();
            if !name.ends_with(".elibu") {
                continue;
            }
            let mut text = String::new();
            f.read_to_string(&mut text)?;
            parse_elibu_content(&text, &mut bundle)?;
        }
    }

    Ok(bundle)
}

fn load_offline_bundle(path: &Path) -> Result<Option<OfflineBundle>, JlcError> {
    let files = gather_input_files(path)?;
    let mut merged = OfflineBundle::default();
    let mut found = false;

    for file in files {
        let ext = file
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if ext != "elibz" && ext != "elibz2" {
            continue;
        }
        found = true;
        let part = load_offline_bundle_from_elibz(&file)?;
        merged.devices.extend(part.devices);
        merged.footprint_data.extend(part.footprint_data);
        merged.symbol_data.extend(part.symbol_data);
        merged.footprint_titles.extend(part.footprint_titles);
        merged.symbol_titles.extend(part.symbol_titles);
        merged.symbol_prefix.extend(part.symbol_prefix);
    }

    if found {
        Ok(Some(merged))
    } else {
        Ok(None)
    }
}

fn parse_local_data_str(ds: &str) -> Option<(Vec<String>, f64, f64)> {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(ds) {
        let shape = v
            .get("shape")
            .and_then(|s| s.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let x = v
            .get("head")
            .and_then(|h| h.get("x"))
            .and_then(|n| n.as_f64())
            .unwrap_or(0.0);
        let y = v
            .get("head")
            .and_then(|h| h.get("y"))
            .and_then(|n| n.as_f64())
            .unwrap_or(0.0);
        if !shape.is_empty() {
            return Some((shape, x, y));
        }
    }

    let lines: Vec<String> = ds
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && l.contains('~'))
        .map(|l| l.to_string())
        .collect();
    if lines.is_empty() {
        return None;
    }
    Some((lines, 0.0, 0.0))
}

fn index_local_models(path: &Path) -> Result<BTreeMap<String, PathBuf>, JlcError> {
    let mut map = BTreeMap::new();
    let files = gather_input_files(path)?;
    for f in files {
        let ext = f
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if ext == "step" || ext == "stp" || ext == "wrl" {
            if let Some(stem) = f.file_stem().and_then(|s| s.to_str()) {
                map.entry(stem.to_lowercase()).or_insert(f);
            }
        }
    }
    Ok(map)
}

fn create_footprint_from_offline(
    device: &OfflineDevice,
    footprint_name_hint: Option<&str>,
    footprint_ds: &str,
    output_dir: &str,
    footprint_lib: &str,
    model_dir: &str,
    models: &[String],
    model_index: &BTreeMap<String, PathBuf>,
) -> Result<bool, JlcError> {
    let (shape, origin_x, origin_y) = parse_local_data_str(footprint_ds)
        .ok_or_else(|| JlcError::ParseError("无法解析本地封装 dataStr".to_string()))?;

    let footprint_name = sanitize_footprint_name(
        footprint_name_hint
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(&device.name),
    );
    let mut footprint_info = FootprintInfo {
        footprint_name: footprint_name.clone(),
        output_dir: output_dir.to_string(),
        footprint_lib: footprint_lib.to_string(),
        model_dir: model_dir.to_string(),
        origin: (origin_x, origin_y),
        models: models.to_vec(),
        ..Default::default()
    };

    let mut kicad_mod_content = String::new();
    kicad_mod_content.push_str("(kicad_mod (version 20220214)\n");
    kicad_mod_content.push_str(&format!(
        "  (footprint {} (identifier {}) (user {})\n",
        footprint_name, footprint_name, footprint_name
    ));

    for line in &shape {
        let parts: Vec<&str> = line.split('~').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            continue;
        }
        let model = parts[0];
        let args: Vec<&str> = parts[1..].to_vec();
        match model {
            "PAD" => {
                if let Some(s) = parse_pad(&args, &mut footprint_info) {
                    kicad_mod_content.push_str(&s);
                }
            }
            "TRACK" => {
                if let Some(s) = parse_track(&args, &mut footprint_info) {
                    kicad_mod_content.push_str(&s);
                }
            }
            "CIRCLE" => {
                if let Some(s) = parse_circle(&args) {
                    kicad_mod_content.push_str(&s);
                }
            }
            "ARC" => {
                if let Some(s) = parse_arc(&args) {
                    kicad_mod_content.push_str(&s);
                }
            }
            "RECT" => {
                if let Some(s) = parse_rect(&args, &mut footprint_info) {
                    kicad_mod_content.push_str(&s);
                }
            }
            "HOLE" => {
                if let Some(s) = parse_hole(&args) {
                    kicad_mod_content.push_str(&s);
                }
            }
            "SOLIDREGION" => {
                if let Some(s) = parse_solid_region(&args) {
                    kicad_mod_content.push_str(&s);
                }
            }
            "TEXT" => {
                if let Some(s) = parse_text(&args) {
                    kicad_mod_content.push_str(&s);
                }
            }
            _ => {}
        }
    }

    let mut model_copied = false;
    if models.contains(&"STEP".to_string()) {
        let mut candidate_keys = vec![device.id.to_lowercase(), footprint_name.to_lowercase()];
        if let Some(mt) = &device.model_title {
            candidate_keys.push(mt.to_lowercase());
        }
        for key in candidate_keys {
            if let Some(src_model) = model_index.get(&key) {
                let ext = src_model
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("step")
                    .to_lowercase();
                let ext = if ext == "stp" { "step" } else { ext.as_str() };
                let model_out_dir = PathBuf::from(output_dir).join(footprint_lib).join(model_dir);
                fs::create_dir_all(&model_out_dir)?;
                let dst_model = model_out_dir.join(format!("{}.{}", footprint_name, ext));
                fs::copy(src_model, &dst_model)?;
                kicad_mod_content.push_str(&format!(
                    "  (model {}/{}.{} (at (xyz 0 0 0)) (rotate (xyz 0 0 0)))\n",
                    model_dir, footprint_name, ext
                ));
                model_copied = true;
                break;
            }
        }
    }

    let center_x = (footprint_info.min_x + footprint_info.max_x) / 2.0;
    let center_y = (footprint_info.min_y + footprint_info.max_y) / 2.0;
    kicad_mod_content.push_str(&format!(
        "  (fp_text reference REF** (at {} {}) (layer F.SilkS)\n    (effects (font (size 1 1)))\n  )\n",
        center_x, footprint_info.min_y - 2.0
    ));
    kicad_mod_content.push_str(&format!(
        "  (fp_text value {} (at {} {}) (layer F.Fab)\n    (effects (font (size 1 1)))\n  )\n",
        footprint_name, center_x, footprint_info.max_y + 2.0
    ));
    kicad_mod_content.push_str(&format!(
        "  (fp_text user ${{REFERENCE}} (at {} {}) (layer F.Fab)\n    (effects (font (size 0.5 0.5)))\n  )\n",
        center_x, center_y
    ));
    kicad_mod_content.push_str("  )\n)\n");

    let output_path = PathBuf::from(output_dir).join(footprint_lib);
    fs::create_dir_all(&output_path)?;
    let file_path = output_path.join(format!("{}.kicad_mod", footprint_name));
    let mut file = File::create(file_path)?;
    file.write_all(kicad_mod_content.as_bytes())?;

    Ok(model_copied)
}

fn symbol_prefix_from_ds(ds: &str) -> String {
    serde_json::from_str::<serde_json::Value>(ds)
        .ok()
        .and_then(|v| {
            v.get("head")
                .and_then(|h| h.get("c_para"))
                .and_then(|c| c.get("pre"))
                .and_then(|p| p.as_str())
                .map(|s| s.replace('?', ""))
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "U".to_string())
}

fn create_symbols_from_offline(
    devices: &[OfflineDevice],
    bundle: &OfflineBundle,
    output_dir: &str,
    symbol_lib: &str,
    symbol_path: &str,
) -> Result<usize, JlcError> {
    let mut lib_content = String::new();
    lib_content.push_str("(kicad_symbol_lib (version 20210201) (generator JLC2KiCad)\n");
    let mut created = 0usize;

    for device in devices {
        for (idx, symbol_uuid) in device.symbol_uuids.iter().enumerate() {
            let Some(ds) = get_symbol_data_by_uuid(bundle, symbol_uuid) else {
                continue;
            };
            let Some((shape, origin_x, origin_y)) = parse_local_data_str(ds) else {
                continue;
            };

            let title = bundle
                .symbol_titles
                .get(symbol_uuid)
                .cloned()
                .unwrap_or_else(|| device.name.clone());
            let component_name = title
                .replace(" ", "_")
                .replace(".", "_")
                .replace("/", "{slash}")
                .replace("\\", "{backslash}")
                .replace("<", "{lt}")
                .replace(">", "{gt}")
                .replace(":", "{colon}")
                .replace('"', "{dblquote}");
            let sym_name = if idx == 0 {
                format!("{}_{}", component_name, device.id)
            } else {
                format!("{}_{}_{}", component_name, device.id, idx)
            };
            let prefix = bundle
                .symbol_prefix
                .get(symbol_uuid)
                .cloned()
                .unwrap_or_else(|| symbol_prefix_from_ds(ds));

            lib_content.push_str(&format!(
                "  (symbol \"{}\" (pin_names hide) (pin_numbers hide) (in_bom yes) (on_board yes)\n",
                sym_name
            ));
            lib_content.push_str(&format!(
                "    (property \"Reference\" \"{}\" (id 0) (at 0 1.27 0)\n      (effects (font (size 1.27 1.27)))\n    )\n",
                prefix
            ));
            lib_content.push_str(&format!(
                "    (property \"Value\" \"{}\" (id 1) (at 0 -2.54 0)\n      (effects (font (size 1.27 1.27)))\n    )\n",
                title
            ));
            lib_content.push_str(&format!(
                "    (property \"LCSC\" \"{}\" (id 5) (at 0 0 0)\n      (effects (font (size 1.27 1.27)) hide)\n    )\n",
                device.id
            ));

            for line in &shape {
                let parts: Vec<&str> = line.split('~').filter(|s| !s.is_empty()).collect();
                if parts.is_empty() {
                    continue;
                }
                let model = parts[0];
                let args: Vec<&str> = parts[1..].to_vec();
                match model {
                    "P" => {
                        if let Some(s) = parse_symbol_pin(&args, origin_x, origin_y) {
                            lib_content.push_str(&s);
                        }
                    }
                    "R" => {
                        if let Some(s) = parse_symbol_rect(&args, origin_x, origin_y) {
                            lib_content.push_str(&s);
                        }
                    }
                    "E" => {
                        if let Some(s) = parse_symbol_circle(&args, origin_x, origin_y) {
                            lib_content.push_str(&s);
                        }
                    }
                    "T" => {
                        if let Some(s) = parse_symbol_text(&args, origin_x, origin_y) {
                            lib_content.push_str(&s);
                        }
                    }
                    "PL" | "PG" => {
                        if let Some(s) = parse_symbol_poly(&args, origin_x, origin_y) {
                            lib_content.push_str(&s);
                        }
                    }
                    _ => {}
                }
            }

            lib_content.push_str("  )\n");
            created += 1;
        }
    }

    lib_content.push_str(")\n");
    let output_path = PathBuf::from(output_dir).join(symbol_path);
    fs::create_dir_all(&output_path)?;
    let file_path = output_path.join(format!("{}.kicad_sym", symbol_lib));
    let mut file = File::create(file_path)?;
    file.write_all(lib_content.as_bytes())?;
    Ok(created)
}

pub async fn load_local_folder(path: &str) -> Result<Vec<SearchResult>, JlcError> {
    let source = Path::new(path);
    let map = collect_local_component_map(source)?;
    Ok(map.into_values().collect())
}

pub async fn convert_local_folder(
    path: &str,
    output_dir: &str,
    footprint_lib: &str,
    symbol_lib: &str,
    symbol_path: &str,
    model_dir: &str,
    models: Vec<String>,
    create_footprint: bool,
    create_symbol: bool,
) -> Result<String, JlcError> {
    let source_path = Path::new(path);
    let bundle_kind = detect_local_bundle_kind(source_path);

    if let Some(bundle) = load_offline_bundle(source_path)? {
        let offline_can_export_footprint = !bundle.footprint_data.is_empty();
        let offline_can_export_symbol = !bundle.symbol_data.is_empty();
        let need_offline_data = (create_footprint && !offline_can_export_footprint)
            || (create_symbol && !offline_can_export_symbol);

        if need_offline_data {
            // New elibz2 bundles may only include device2.json + .elibu.
            // In this case keep local-ID discovery, then fall back to online conversion path.
            let component_ids = collect_component_ids_from_path(source_path)?;
            let mut success = 0usize;
            let mut failed: Vec<String> = Vec::new();

            for component_id in component_ids {
                match create_component(
                    &component_id,
                    output_dir,
                    footprint_lib,
                    symbol_lib,
                    symbol_path,
                    model_dir,
                    models.clone(),
                    create_footprint,
                    create_symbol,
                )
                .await
                {
                    Ok(_) => success += 1,
                    Err(e) => failed.push(format!("{}: {}", component_id, e)),
                }
            }

            if failed.is_empty() {
                return Ok(format!(
                    "本地转换完成（检测到 elibz2，已使用在线补全），成功 {} 个元件",
                    success
                ));
            } else {
                return Ok(format!(
                    "本地转换完成（检测到 elibz2，已使用在线补全），成功 {} 个，失败 {} 个\n{}",
                    success,
                    failed.len(),
                    failed.join("\n")
                ));
            }
        }

        let component_ids = collect_component_ids_from_path(source_path)?;
        let model_index = index_local_models(source_path).unwrap_or_default();
        let mut success = 0usize;
        let mut failed: Vec<String> = Vec::new();
        let mut selected_devices: Vec<OfflineDevice> = Vec::new();

        for component_id in component_ids {
            let Some(device) = bundle.devices.get(&component_id).cloned() else {
                failed.push(format!("{}: 本地库缺少 device 元数据", component_id));
                continue;
            };
            let model_name = device
                .footprint_uuid
                .as_ref()
                .and_then(|u| get_footprint_title_by_uuid(&bundle, u))
                .unwrap_or_else(|| device.name.clone());
            selected_devices.push(device.clone());

            if create_footprint {
                if let Some(fp_uuid) = &device.footprint_uuid {
                    if let Some(ds) = bundle.footprint_data.get(fp_uuid) {
                        match create_footprint_from_offline(
                            &device,
                            device
                                .footprint_uuid
                                .as_ref()
                                .and_then(|u| get_footprint_title_by_uuid(&bundle, u))
                                .as_deref(),
                            ds,
                            output_dir,
                            footprint_lib,
                            model_dir,
                            &models,
                            &model_index,
                        ) {
                            Ok(_) => {
                                // Local libraries usually do not include 3D models.
                                // If STEP is requested, fetch it online directly.
                                if models.contains(&"STEP".to_string()) {
                                    match download_step_only_online(
                                        &component_id,
                                        &model_name,
                                        output_dir,
                                        footprint_lib,
                                        model_dir,
                                    )
                                    .await
                                    {
                                        Ok(_) => success += 1,
                                        Err(e) => failed.push(format!(
                                            "{}: 封装已导出，但在线拉取3D失败: {}",
                                            component_id, e
                                        )),
                                    }
                                } else {
                                    success += 1;
                                }
                            }
                            Err(e) => failed.push(format!("{}: {}", component_id, e)),
                        }
                    } else {
                        failed.push(format!("{}: 本地库缺少封装数据 {}", component_id, fp_uuid));
                    }
                } else {
                    failed.push(format!("{}: 本地库未提供封装UUID", component_id));
                }
            } else if models.contains(&"STEP".to_string()) && !create_symbol {
                // 3D-only mode: always fetch online (do not search local files).
                match download_step_only_online(
                    &component_id,
                    &model_name,
                    output_dir,
                    footprint_lib,
                    model_dir,
                )
                .await
                {
                    Ok(_) => success += 1,
                    Err(e) => failed.push(format!("{}: 在线拉取3D失败: {}", component_id, e)),
                }
            } else {
                success += 1;
            }
        }

        if create_symbol {
            match create_symbols_from_offline(
                &selected_devices,
                &bundle,
                output_dir,
                symbol_lib,
                symbol_path,
            ) {
                Ok(0) => failed.push("符号导出失败: 本地库未解析到可用符号数据".to_string()),
                Ok(_) => {}
                Err(e) => failed.push(format!("符号导出失败: {}", e)),
            }
        }

        if failed.is_empty() {
            if create_symbol {
                let symbol_file = PathBuf::from(output_dir)
                    .join(symbol_path)
                    .join(format!("{}.kicad_sym", symbol_lib));
                return Ok(format!(
                    "本地离线转换完成（{}），成功 {} 个元件\n器件库文件: {}",
                    bundle_kind,
                    success,
                    symbol_file.display()
                ));
            }
            return Ok(format!("本地离线转换完成（{}），成功 {} 个元件", bundle_kind, success));
        } else {
            return Ok(format!(
                "本地离线转换完成（{}），成功 {} 个，失败 {} 个\n{}",
                bundle_kind,
                success,
                failed.len(),
                failed.join("\n")
            ));
        }
    }

    let component_ids = collect_component_ids_from_path(Path::new(path))?;

    let mut success = 0usize;
    let mut failed: Vec<String> = Vec::new();

    for component_id in component_ids {
        match create_component(
            &component_id,
            output_dir,
            footprint_lib,
            symbol_lib,
            symbol_path,
            model_dir,
            models.clone(),
            create_footprint,
            create_symbol,
        )
        .await
        {
            Ok(_) => success += 1,
            Err(e) => failed.push(format!("{}: {}", component_id, e)),
        }
    }

    if failed.is_empty() {
        Ok(format!("本地转换完成，成功 {} 个元件", success))
    } else {
        Ok(format!(
            "本地转换完成，成功 {} 个，失败 {} 个\n{}",
            success,
            failed.len(),
            failed.join("\n")
        ))
    }
}

async fn create_footprint_internal(
    client: &JlcClient,
    footprint_uuid: &str,
    component_id: &str,
    output_dir: &str,
    footprint_lib: &str,
    model_dir: &str,
    models: &[String],
) -> Result<(String, String, bool, Option<String>), JlcError> {
    let data = client.get_footprint_data(footprint_uuid).await?;

    let title = &data.result.title;
    let footprint_name = title
        .replace(" ", "_")
        .replace("/", "_")
        .replace("(", "_")
        .replace(")", "_");

    let shape = &data.result.data_str.shape;
    let (origin_x, origin_y) = (data.result.data_str.head.x, data.result.data_str.head.y);
    let datasheet_link = data
        .result
        .data_str
        .head
        .c_para
        .and_then(|c| c.link)
        .unwrap_or_default();

    let mut footprint_info = FootprintInfo {
        footprint_name: footprint_name.clone(),
        output_dir: output_dir.to_string(),
        footprint_lib: footprint_lib.to_string(),
        model_dir: model_dir.to_string(),
        origin: (origin_x, origin_y),
        models: models.iter().map(|s| s.clone()).collect(),
        ..Default::default()
    };
    let mut svg_model_uuid: Option<String> = None;
    let mut step_model_downloaded = false;
    let mut step_model_error: Option<String> = None;

    let mut kicad_mod_content = String::new();

    // Generate KiCad footprint header
    kicad_mod_content.push_str(&format!(
        "(kicad_mod (version {})\n",
        "20220214"
    ));
    kicad_mod_content.push_str(&format!("  (footprint {} (identifier {}) (user {})\n",
        footprint_name, footprint_name, footprint_name));

    // Parse shape and generate footprint elements
    for line in shape {
        let parts: Vec<&str> = line.split('~').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            continue;
        }

        let model = parts[0];
        let args: Vec<&str> = parts[1..].to_vec();

        match model {
            "PAD" => {
                if let Some(pad_str) = parse_pad(&args, &mut footprint_info) {
                    kicad_mod_content.push_str(&pad_str);
                }
            }
            "TRACK" => {
                if let Some(track_str) = parse_track(&args, &mut footprint_info) {
                    kicad_mod_content.push_str(&track_str);
                }
            }
            "CIRCLE" => {
                if let Some(circle_str) = parse_circle(&args) {
                    kicad_mod_content.push_str(&circle_str);
                }
            }
            "ARC" => {
                if let Some(arc_str) = parse_arc(&args) {
                    kicad_mod_content.push_str(&arc_str);
                }
            }
            "RECT" => {
                if let Some(rect_str) = parse_rect(&args, &mut footprint_info) {
                    kicad_mod_content.push_str(&rect_str);
                }
            }
            "HOLE" => {
                if let Some(hole_str) = parse_hole(&args) {
                    kicad_mod_content.push_str(&hole_str);
                }
            }
            "SOLIDREGION" => {
                if let Some(solid_str) = parse_solid_region(&args) {
                    kicad_mod_content.push_str(&solid_str);
                }
            }
            "TEXT" => {
                if let Some(text_str) = parse_text(&args) {
                    kicad_mod_content.push_str(&text_str);
                }
            }
            "SVGNODE" => {
                if let Ok(json_data) = serde_json::from_str::<serde_json::Value>(args[0]) {
                    if let Some(uuid) = json_data
                        .get("attrs")
                        .and_then(|a| a.get("uuid"))
                        .and_then(|u| u.as_str())
                    {
                        svg_model_uuid = Some(uuid.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    if models.contains(&"STEP".to_string()) {
        let step_dir = PathBuf::from(output_dir).join(footprint_lib).join(model_dir);
        fs::create_dir_all(&step_dir)?;
        let step_path = step_dir.join(format!("{}.step", footprint_name));

        let mut model_candidates: Vec<String> = Vec::new();
        if let Ok(Some(uuid)) = client.resolve_step_uuid_via_pro_api(component_id).await {
            model_candidates.push(uuid);
        }
        if let Some(uuid) = svg_model_uuid {
            model_candidates.push(uuid);
        }
        model_candidates.push(footprint_uuid.to_string());
        model_candidates.dedup();

        for uuid in model_candidates {
            match client.download_step_model(&uuid, step_path.to_str().unwrap()).await {
                Ok(_) => {
                    step_model_downloaded = true;
                    let path_name = format!("{}/{}.step", model_dir, footprint_name);
                    kicad_mod_content.push_str(&format!(
                        "  (model {} (at (xyz 0 0 0)) (rotate (xyz 0 0 0)))\n",
                        path_name
                    ));
                    break;
                }
                Err(e) => {
                    step_model_error = Some(format!(
                        "3D 模型下载失败（模型UUID: {}）: {}",
                        uuid, e
                    ));
                }
            }
        }
    }

    // Add reference, value text
    let center_x = (footprint_info.min_x + footprint_info.max_x) / 2.0;
    let center_y = (footprint_info.min_y + footprint_info.max_y) / 2.0;

    kicad_mod_content.push_str(&format!(
        "  (fp_text reference REF** (at {} {}) (layer F.SilkS)\n    (effects (font (size 1 1)))\n  )\n",
        center_x, footprint_info.min_y - 2.0
    ));
    kicad_mod_content.push_str(&format!(
        "  (fp_text value {} (at {} {}) (layer F.Fab)\n    (effects (font (size 1 1)))\n  )\n",
        footprint_name, center_x, footprint_info.max_y + 2.0
    ));
    kicad_mod_content.push_str(&format!(
        "  (fp_text user ${{REFERENCE}} (at {} {}) (layer F.Fab)\n    (effects (font (size 0.5 0.5)))\n  )\n",
        center_x, center_y
    ));

    // Close footprint and root node
    kicad_mod_content.push_str("  )\n");
    kicad_mod_content.push_str(")\n");

    // Write to file
    let output_path = PathBuf::from(output_dir).join(footprint_lib);
    fs::create_dir_all(&output_path)?;
    let file_path = output_path.join(format!("{}.kicad_mod", footprint_name));
    let mut file = File::create(file_path)?;
    file.write_all(kicad_mod_content.as_bytes())?;

    Ok((footprint_name, datasheet_link, step_model_downloaded, step_model_error))
}

fn layer_map(layer_id: &str) -> &'static str {
    match layer_id {
        "1" => "F.Cu",
        "2" => "B.Cu",
        "3" => "F.SilkS",
        "4" => "B.SilkS",
        "5" => "F.Paste",
        "6" => "B.Paste",
        "7" => "F.Mask",
        "8" => "B.Mask",
        "10" => "Edge.Cuts",
        "11" => "F.Fab",
        "12" => "F.Fab",
        "99" => "F.Fab",
        "100" => "F.Fab",
        "101" => "F.Fab",
        _ => "F.SilkS",
    }
}

fn parse_pad(args: &[&str], info: &mut FootprintInfo) -> Option<String> {
    // args: [shape, x, y, size_x, size_y, layer, ..., pad_num, drill, ..., rotation, ...]
    if args.len() < 9 {
        return None;
    }

    let shape = args[0];
    let x = mil2mm(args[1].parse().unwrap_or(0.0));
    let y = mil2mm(args[2].parse().unwrap_or(0.0));
    let size_x = mil2mm(args[3].parse().unwrap_or(1.0));
    let size_y = mil2mm(args[4].parse().unwrap_or(1.0));
    let layer = args[5];
    let pad_num = args[7];
    let drill_diameter = mil2mm(args[8].parse::<f64>().unwrap_or(0.0)) * 2.0;
    let rotation: f64 = args.get(10).and_then(|s| s.parse().ok()).unwrap_or(0.0);

    // Update footprint bounds
    info.max_x = info.max_x.max(x);
    info.min_x = info.min_x.min(x);
    info.max_y = info.max_y.max(y);
    info.min_y = info.min_y.min(y);

    let pad_type = if layer == "11" {
        "thru_hole"
    } else {
        "smd"
    };

    let ki_shape = match shape {
        "OVAL" => "oval",
        "RECT" => "rect",
        "ELLIPSE" => "circle",
        "CIRCLE" => "circle",
        _ => "oval",
    };

    let layers = if layer == "11" {
        "*.Cu *.Mask"
    } else if layer == "1" {
        "F.Cu F.Paste F.Mask"
    } else {
        "B.Cu B.Paste B.Mask"
    };

    let drill = if pad_type == "thru_hole" && drill_diameter > 0.0 {
        format!(" (drill {})", drill_diameter)
    } else {
        String::new()
    };

    Some(format!(
        "  (pad {} {} {} (at {} {} {}) (size {} {}){} (layers {}))\n",
        pad_num, pad_type, ki_shape, x, y, rotation, size_x, size_y, drill, layers
    ))
}

fn parse_track(args: &[&str], info: &mut FootprintInfo) -> Option<String> {
    if args.len() < 4 {
        return None;
    }

    let width = mil2mm(args[0].parse().unwrap_or(0.2));
    let layer = layer_map(args[1]);
    let points_str = args[3];
    let points: Vec<f64> = points_str
        .split(' ')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .map(|v| mil2mm(v))
        .collect();

    if points.len() < 4 {
        return None;
    }

    let mut result = String::new();
    for i in (0..points.len() - 2).step_by(2) {
        let x1 = points[i];
        let y1 = points[i + 1];
        let x2 = points[i + 2];
        let y2 = points[i + 3];

        // Update bounds
        info.max_x = info.max_x.max(x1).max(x2);
        info.min_x = info.min_x.min(x1).min(x2);
        info.max_y = info.max_y.max(y1).max(y2);
        info.min_y = info.min_y.min(y1).min(y2);

        result.push_str(&format!(
            "  (fp_line (start {} {}) (end {} {}) (layer {}) (width {}))\n",
            x1, y1, x2, y2, layer, width
        ));
    }

    Some(result)
}

fn parse_circle(args: &[&str]) -> Option<String> {
    if args.len() < 4 {
        return None;
    }

    let cx = mil2mm(args[0].parse().unwrap_or(0.0));
    let cy = mil2mm(args[1].parse().unwrap_or(0.0));
    let r = mil2mm(args[2].parse().unwrap_or(0.0));
    let width = mil2mm(args[3].parse().unwrap_or(0.2));
    let layer = layer_map(args.get(4).unwrap_or(&"3"));

    // Skip circles on pad layer
    if args.get(4).map(|s| *s == "100").unwrap_or(false) {
        return None;
    }

    Some(format!(
        "  (fp_circle (center {} {}) (end {} {}) (layer {}) (width {}))\n",
        cx, cy, cx + r, cy, layer, width
    ))
}

fn parse_arc(args: &[&str]) -> Option<String> {
    if args.len() < 4 {
        return None;
    }

    let _layer = layer_map(args.get(1).unwrap_or(&"3"));
    let _width = mil2mm(args[0].parse().unwrap_or(0.2));

    Some(String::new())
}

fn parse_rect(args: &[&str], info: &mut FootprintInfo) -> Option<String> {
    if args.len() < 8 {
        return None;
    }

    let x1 = mil2mm(args[0].parse().unwrap_or(0.0));
    let y1 = mil2mm(args[1].parse().unwrap_or(0.0));
    let dx = mil2mm(args[2].parse().unwrap_or(0.0));
    let dy = mil2mm(args[3].parse().unwrap_or(0.0));
    let x2 = x1 + dx;
    let y2 = y1 + dy;
    let layer = layer_map(args.get(4).unwrap_or(&"3"));
    let width = mil2mm(args.get(7).unwrap_or(&"0").parse().unwrap_or(0.2));

    info.max_x = info.max_x.max(x1).max(x2);
    info.min_x = info.min_x.min(x1).min(x2);
    info.max_y = info.max_y.max(y1).max(y2);
    info.min_y = info.min_y.min(y1).min(y2);

    if width == 0.0 {
        Some(format!(
            "  (fp_rect (start {} {}) (end {} {}) (layer {}))\n",
            x1, y1, x2, y2, layer
        ))
    } else {
        Some(format!(
            "  (fp_line (start {} {}) (end {} {}) (layer {}) (width {}))\n",
            x1, y1, x2, y1, layer, width
        ))
    }
}

fn parse_hole(args: &[&str]) -> Option<String> {
    if args.len() < 3 {
        return None;
    }

    let x = mil2mm(args[0].parse().unwrap_or(0.0));
    let y = mil2mm(args[1].parse().unwrap_or(0.0));
    let r = mil2mm(args[2].parse().unwrap_or(0.0)) * 2.0;

    Some(format!(
        "  (pad \"\" np_thru_hole circle (at {} {}) (size {} {}) (drill {}))\n",
        x, y, r, r, r
    ))
}

fn parse_solid_region(_args: &[&str]) -> Option<String> {
    Some(String::new())
}

fn parse_text(args: &[&str]) -> Option<String> {
    if args.len() < 12 {
        return None;
    }

    let x = mil2mm(args[1].parse().unwrap_or(0.0));
    let y = mil2mm(args[2].parse().unwrap_or(0.0));
    let text = args.get(11).unwrap_or(&"");

    Some(format!(
        "  (fp_text user {} (at {} {}) (layer F.SilkS)\n    (effects (font (size 1 1)))\n  )\n",
        text, x, y
    ))
}

async fn create_symbol_internal(
    client: &JlcClient,
    symbol_uuids: &[String],
    footprint_name: &str,
    datasheet_link: &str,
    component_id: &str,
    output_dir: &str,
    symbol_lib: &str,
    symbol_path: &str,
) -> Result<(), JlcError> {
    let mut lib_content = String::new();
    lib_content.push_str("(kicad_symbol_lib (version 20210201) (generator JLC2KiCad)\n");

    for (idx, symbol_uuid) in symbol_uuids.iter().enumerate() {
        let data = client.get_symbol_data(symbol_uuid).await?;
        
        let title = &data.result.title;
        let component_name = title
            .replace(" ", "_")
            .replace(".", "_")
            .replace("/", "{slash}")
            .replace("\\", "{backslash}")
            .replace("<", "{lt}")
            .replace(">", "{gt}")
            .replace(":", "{colon}")
            .replace('"', "{dblquote}");

        let prefix = data.result.package_detail.data_str.head.c_para.pre.replace("?", "");

        let shape = &data.result.data_str.shape;
        let (origin_x, origin_y) = (data.result.data_str.head.x, data.result.data_str.head.y);

        let sym_name = if idx == 0 {
            format!("{}_0", component_name)
        } else {
            component_name.clone()
        };

        lib_content.push_str(&format!(
            "  (symbol \"{}\" (pin_names hide) (pin_numbers hide) (in_bom yes) (on_board yes)\n",
            sym_name
        ));

        lib_content.push_str(&format!(
            "    (property \"Reference\" \"{}\" (id 0) (at 0 1.27 0)\n      (effects (font (size 1.27 1.27)))\n    )\n",
            prefix
        ));

        lib_content.push_str(&format!(
            "    (property \"Value\" \"{}\" (id 1) (at 0 -2.54 0)\n      (effects (font (size 1.27 1.27)))\n    )\n",
            title
        ));

        lib_content.push_str(&format!(
            "    (property \"Footprint\" \"{}\" (id 2) (at 0 -10.16 0)\n      (effects (font (size 1.27 1.27) italic) hide)\n    )\n",
            footprint_name
        ));

        lib_content.push_str(&format!(
            "    (property \"Datasheet\" \"{}\" (id 3) (at -2.286 0.127 0)\n      (effects (font (size 1.27 1.27)) (justify left) hide)\n    )\n",
            datasheet_link
        ));

        lib_content.push_str(&format!(
            "    (property \"ki_keywords\" \"{}\" (id 4) (at 0 0 0)\n      (effects (font (size 1.27 1.27)) hide)\n    )\n",
            component_id
        ));

        lib_content.push_str(&format!(
            "    (property \"LCSC\" \"{}\" (id 5) (at 0 0 0)\n      (effects (font (size 1.27 1.27)) hide)\n    )\n",
            component_id
        ));

        // Parse symbol shapes
        for line in shape {
            let parts: Vec<&str> = line.split('~').filter(|s| !s.is_empty()).collect();
            if parts.is_empty() {
                continue;
            }

            let model = parts[0];
            let args: Vec<&str> = parts[1..].to_vec();

            match model {
                "P" => {
                    if let Some(pin_str) = parse_symbol_pin(&args, origin_x, origin_y) {
                        lib_content.push_str(&pin_str);
                    }
                }
                "R" => {
                    if let Some(rect_str) = parse_symbol_rect(&args, origin_x, origin_y) {
                        lib_content.push_str(&rect_str);
                    }
                }
                "E" => {
                    if let Some(circle_str) = parse_symbol_circle(&args, origin_x, origin_y) {
                        lib_content.push_str(&circle_str);
                    }
                }
                "T" => {
                    if let Some(text_str) = parse_symbol_text(&args, origin_x, origin_y) {
                        lib_content.push_str(&text_str);
                    }
                }
                "PL" | "PG" => {
                    if let Some(poly_str) = parse_symbol_poly(&args, origin_x, origin_y) {
                        lib_content.push_str(&poly_str);
                    }
                }
                "A" => {
                    // Arc - simplified
                }
                _ => {}
            }
        }

        lib_content.push_str("  )\n");
    }

    lib_content.push_str(")\n");

    // Write to file
    let output_path = PathBuf::from(output_dir).join(symbol_path);
    fs::create_dir_all(&output_path)?;
    let file_path = output_path.join(format!("{}.kicad_sym", symbol_lib));
    let mut file = File::create(file_path)?;
    file.write_all(lib_content.as_bytes())?;

    Ok(())
}

fn parse_symbol_pin(args: &[&str], origin_x: f64, origin_y: f64) -> Option<String> {
    if args.len() < 14 {
        return None;
    }

    let electrical_type = match args[1] {
        "0" => "unspecified",
        "1" => "input",
        "2" => "output",
        "3" => "bidirectional",
        "4" => "power_in",
        _ => "unspecified",
    };

    let pin_num = args[2];
    let x = mil2mm(args[3].parse::<f64>().unwrap_or(0.0) - origin_x);
    let y = -mil2mm(args[4].parse::<f64>().unwrap_or(0.0) - origin_y);
    let rotation: i32 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
    let rotation = (rotation + 180) % 360;
    let pin_name = args.get(13).unwrap_or(&"");

    let length = 2.54;

    Some(format!(
        "    (pin {} line (at {} {} {}) (length {})\n      (name \"{}\" (effects (font (size 1 1))))\n      (number \"{}\" (effects (font (size 1 1))))\n    )\n",
        electrical_type, x, y, rotation, length, pin_name, pin_num
    ))
}

fn parse_symbol_rect(args: &[&str], origin_x: f64, origin_y: f64) -> Option<String> {
    if args.len() < 6 {
        return None;
    }

    let x1 = mil2mm(args[0].parse::<f64>().unwrap_or(0.0) - origin_x);
    let y1 = -mil2mm(args[1].parse::<f64>().unwrap_or(0.0) - origin_y);
    let width = mil2mm(args[4].parse::<f64>().unwrap_or(0.0));
    let length = mil2mm(args[5].parse::<f64>().unwrap_or(0.0));
    let x2 = x1 + width;
    let y2 = y1 - length;

    Some(format!(
        "    (rectangle (start {} {}) (end {} {}) (stroke (width 0) (type default)) (fill (type background)))\n",
        x1, y1, x2, y2
    ))
}

fn parse_symbol_circle(args: &[&str], origin_x: f64, origin_y: f64) -> Option<String> {
    if args.len() < 3 {
        return None;
    }

    let x = mil2mm(args[0].parse::<f64>().unwrap_or(0.0) - origin_x);
    let y = -mil2mm(args[1].parse::<f64>().unwrap_or(0.0) - origin_y);
    let r = mil2mm(args[2].parse::<f64>().unwrap_or(0.0));

    Some(format!(
        "    (circle (center {} {}) (radius {}) (stroke (width 0) (type default)) (fill (type background)))\n",
        x, y, r
    ))
}

fn parse_symbol_text(args: &[&str], origin_x: f64, origin_y: f64) -> Option<String> {
    if args.len() < 12 {
        return None;
    }

    let x = mil2mm(args[1].parse::<f64>().unwrap_or(0.0) - origin_x);
    let y = -mil2mm(args[2].parse::<f64>().unwrap_or(0.0) - origin_y);
    let rotation: i32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);
    let rotation = (rotation + 180) % 360 * 10;
    let text = args.get(11).unwrap_or(&"");

    Some(format!(
        "    (text \"{}\" (at {} {} {}) (effects (font (size 1.27 1.27))))\n",
        text, x, y, rotation
    ))
}

fn parse_symbol_poly(args: &[&str], origin_x: f64, origin_y: f64) -> Option<String> {
    if args.is_empty() {
        return None;
    }

    let points_str = args[0];
    let points: Vec<f64> = points_str
        .split(' ')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();

    if points.len() < 4 {
        return None;
    }

    let mut pts_str = String::new();
    for i in (0..points.len()).step_by(2) {
        if i + 1 < points.len() {
            let x = mil2mm(points[i] - origin_x);
            let y = -mil2mm(points[i + 1] - origin_y);
            pts_str.push_str(&format!("(xy {} {}) ", x, y));
        }
    }

    Some(format!(
        "    (polyline (pts {}) (stroke (width 0) (type default)) (fill (type none)))\n",
        pts_str
    ))
}

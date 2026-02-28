use crate::types::FootprintInfo;
use once_cell::sync::Lazy;
use regex::Regex;

pub fn mil2mm(mils: f64) -> f64 {
    mils / 3.937
}

pub fn sanitize_footprint_name(title: &str) -> String {
    title
        .replace(" ", "_")
        .replace("/", "_")
        .replace("(", "_")
        .replace(")", "_")
}

pub fn extract_model_uuid_from_shape(shape: &[String]) -> Option<String> {
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

pub fn uuid_first_part(value: &str) -> String {
    value.split('|').next().unwrap_or(value).to_string()
}

pub fn first_non_empty_str(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
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

pub fn looks_like_uuidish(value: &str) -> bool {
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

pub fn first_readable_package(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
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

pub fn extract_package_name(value: &serde_json::Value) -> Option<String> {
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
                &[
                    "display_title",
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

    None
}

pub fn extract_manufacturer_name(value: &serde_json::Value) -> Option<String> {
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

pub fn extract_brief_desc(value: &serde_json::Value) -> Option<String> {
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
        if let Some(v) =
            first_non_empty_str(attrs, &["Description", "Comment", "Value", "描述", "备注"])
        {
            return Some(v);
        }
    }

    None
}

pub fn normalize_display_name(
    raw: Option<String>,
    fallback_id: &str,
    package_hint: Option<&str>,
) -> String {
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

pub fn extract_preferred_local_id(device: &serde_json::Value) -> Option<String> {
    let attrs = device.get("attributes").unwrap_or(device);

    let direct = first_non_empty_str(
        device,
        &[
            "product_code",
            "productCode",
            "code",
            "lcsc",
            "partNumber",
            "part_number",
        ],
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

    if let Ok(text) = serde_json::to_string(device) {
        if let Some(m) = component_id_regex().find(&text) {
            if let Some(id) = normalize_component_token(m.as_str()) {
                return Some(id);
            }
        }
    }

    first_non_empty_str(device, &["id", "uuid"])
        .or_else(|| first_non_empty_str(attrs, &["uuid"]))
        .and_then(|s| normalize_component_token(&s))
}

static COMPONENT_ID_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)C\d+").unwrap());

pub fn component_id_regex() -> &'static Regex {
    &COMPONENT_ID_REGEX
}

pub fn normalize_component_token(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    let upper = trimmed.to_uppercase();
    if upper.starts_with('C') {
        let alphanum: String = upper
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect();
        if !alphanum.is_empty() && alphanum.len() <= 20 {
            return Some(alphanum);
        }
    }
    None
}

pub fn calculate_footprint_bounds(info: &mut FootprintInfo, x: f64, y: f64) {
    info.max_x = info.max_x.max(x);
    info.max_y = info.max_y.max(y);
    info.min_x = info.min_x.min(x);
    info.min_y = info.min_y.min(y);
}

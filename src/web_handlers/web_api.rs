use std::collections::HashMap;

use actix_web::{web, HttpRequest, HttpResponse};
use log::error;
use serde::Deserialize;

use crate::api::{sub_process, SubResponse, SubconverterQuery};
use crate::generator::ruleconvert::common::transform_rule_to_common;
use crate::generator::ruleconvert::convert_ruleset::convert_ruleset;
use crate::models::ruleset::{get_ruleset_type_from_url, RULESET_TYPES};
use crate::models::RulesetType;
use crate::rulesets::ruleset::fetch_ruleset;
use crate::utils::base64::url_safe_base64_decode;
use crate::utils::file_exists;
use crate::utils::http::parse_proxy;
use crate::utils::ini_reader::IniReader;
use crate::Settings;
impl SubResponse {
    /// Convert SubResponse to HttpResponse
    pub fn to_http_response(self) -> HttpResponse {
        // Create response with appropriate status code
        let mut http_response = match self.status_code {
            200 => HttpResponse::Ok(),
            400 => HttpResponse::BadRequest(),
            500 => HttpResponse::InternalServerError(),
            _ => HttpResponse::build(
                actix_web::http::StatusCode::from_u16(self.status_code)
                    .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR),
            ),
        };

        // Add headers
        for (name, value) in self.headers {
            http_response.append_header((name, value));
        }

        // Set content type
        http_response.content_type(self.content_type);

        // Return response with content
        http_response.body(self.content)
    }
}

#[derive(Debug, Deserialize)]
pub struct ProfileQuery {
    pub name: String,
    pub token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RulesetQuery {
    #[serde(rename = "type")]
    pub rule_type: i32,
    pub url: String,
    pub group: Option<String>,
}

fn is_api_authorized(token: Option<&str>) -> bool {
    let settings = Settings::current();
    if settings.api_access_token.is_empty() {
        return true;
    }

    token.unwrap_or_default() == settings.api_access_token
}

async fn load_profile_query(profile_name: &str) -> Result<SubconverterQuery, String> {
    let mut candidate_paths = vec![profile_name.to_string()];
    if !profile_name.starts_with("base/") {
        candidate_paths.push(format!("base/{}", profile_name));
    }

    for path in candidate_paths {
        if !file_exists(&path).await {
            continue;
        }

        let mut ini = IniReader::new();
        if let Err(e) = ini.parse_file(&path).await {
            return Err(format!("failed to parse profile '{}': {}", path, e));
        }

        if ini.enter_section("Profile").is_err() {
            return Err(format!("profile '{}' has no [Profile] section", path));
        }

        let items = ini
            .get_items("Profile")
            .map_err(|e| format!("failed reading [Profile] in '{}': {}", path, e))?;

        let encoded = items
            .into_iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(&k), urlencoding::encode(&v)))
            .collect::<Vec<_>>()
            .join("&");

        return serde_urlencoded::from_str::<SubconverterQuery>(&encoded)
            .map_err(|e| format!("failed converting profile '{}' to query: {}", path, e));
    }

    Err(format!("profile not found: {}", profile_name))
}

fn build_clash_payload(lines: &[String]) -> String {
    let escaped = lines
        .iter()
        .map(|line| format!("  - '{}'", line.replace('\'', "''")))
        .collect::<Vec<_>>()
        .join("\n");
    format!("payload:\n{}\n", escaped)
}

fn normalize_rules_lines(content: &str) -> Vec<String> {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with('#') && !line.starts_with(';') && !line.starts_with("//"))
        .map(ToString::to_string)
        .collect()
}

fn extract_rule_value(line: &str) -> Option<(String, String)> {
    let mut parts = line.split(',');
    let rule_type = parts.next()?.trim().to_string();
    let value = parts.next()?.trim().to_string();
    Some((rule_type, value))
}

async fn build_ruleset_response(query: &RulesetQuery) -> Result<String, String> {
    let settings = Settings::current();
    let proxy = parse_proxy(&settings.proxy_ruleset);

    let decoded_url = url_safe_base64_decode(&query.url);
    let mut fetch_url = decoded_url.clone();
    let mut source_type = RulesetType::Surge;

    if let Some(detected) = get_ruleset_type_from_url(&decoded_url) {
        source_type = detected;
        for (prefix, prefix_type) in RULESET_TYPES.iter() {
            if decoded_url.starts_with(prefix) && *prefix_type == detected {
                fetch_url = decoded_url[prefix.len()..].to_string();
                break;
            }
        }
    }

    if !fetch_url.starts_with("http://")
        && !fetch_url.starts_with("https://")
        && !file_exists(&fetch_url).await
    {
        let fallback = format!("base/{}", fetch_url);
        if file_exists(&fallback).await {
            fetch_url = fallback;
        }
    }

    let raw = fetch_ruleset(
        &fetch_url,
        &proxy,
        settings.cache_ruleset,
        settings.async_fetch_ruleset,
    )
    .await
    .map_err(|e| format!("failed to fetch ruleset: {}", e))?;

    let surge_lines = normalize_rules_lines(&convert_ruleset(&raw, source_type));
    let group = query
        .group
        .as_deref()
        .map(url_safe_base64_decode)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "DIRECT".to_string());

    let output = match query.rule_type {
        1 => surge_lines.join("\n") + "\n",
        2 => {
            surge_lines
                .iter()
                .map(|line| transform_rule_to_common(line, &group, true))
                .collect::<Vec<_>>()
                .join("\n")
                + "\n"
        }
        3 => {
            let payload = surge_lines
                .iter()
                .filter_map(|line| extract_rule_value(line))
                .filter_map(|(rule_type, value)| match rule_type.as_str() {
                    "DOMAIN" => Some(value),
                    "DOMAIN-SUFFIX" => Some(format!("+.{}", value)),
                    "DOMAIN-KEYWORD" => Some(format!("*{}*", value)),
                    _ => None,
                })
                .collect::<Vec<_>>();
            build_clash_payload(&payload)
        }
        4 => {
            let payload = surge_lines
                .iter()
                .filter_map(|line| extract_rule_value(line))
                .filter_map(|(rule_type, value)| match rule_type.as_str() {
                    "IP-CIDR" | "IP-CIDR6" => Some(value),
                    _ => None,
                })
                .collect::<Vec<_>>();
            build_clash_payload(&payload)
        }
        6 => build_clash_payload(&surge_lines),
        _ => return Err("unsupported ruleset type".to_string()),
    };

    Ok(output)
}

pub async fn version_handler() -> HttpResponse {
    HttpResponse::Ok().body(format!(
        "subconverter v{} backend\n",
        env!("CARGO_PKG_VERSION")
    ))
}

pub async fn profile_handler(req: HttpRequest, query: web::Query<ProfileQuery>) -> HttpResponse {
    if !is_api_authorized(query.token.as_deref()) {
        return HttpResponse::Forbidden().body("Forbidden");
    }

    let mut profile_query = match load_profile_query(&query.name).await {
        Ok(q) => q,
        Err(e) => return HttpResponse::BadRequest().body(e),
    };

    let mut request_headers = HashMap::new();
    for (key, value) in req.headers() {
        request_headers.insert(key.to_string(), value.to_str().unwrap_or("").to_string());
    }
    profile_query.request_headers = Some(request_headers);

    match sub_process(Some(req.uri().to_string()), profile_query).await {
        Ok(response) => response.to_http_response(),
        Err(e) => {
            error!("getprofile process error: {}", e);
            HttpResponse::InternalServerError().body(format!("Internal server error: {}", e))
        }
    }
}

pub async fn ruleset_handler(query: web::Query<RulesetQuery>) -> HttpResponse {
    match build_ruleset_response(&query).await {
        Ok(content) => HttpResponse::Ok().content_type("text/plain").body(content),
        Err(e) => HttpResponse::BadRequest().body(e),
    }
}

pub async fn sub_handler(req: HttpRequest, query: web::Query<SubconverterQuery>) -> HttpResponse {
    let req_url = req.uri().to_string();

    let mut request_headers = HashMap::new();
    for (key, value) in req.headers() {
        request_headers.insert(key.to_string(), value.to_str().unwrap_or("").to_string());
    }

    let mut modified_query = query.into_inner();
    modified_query.request_headers = Some(request_headers);

    match sub_process(Some(req_url), modified_query).await {
        Ok(response) => response.to_http_response(),
        Err(e) => {
            error!("Subconverter process error: {}", e);
            HttpResponse::InternalServerError().body(format!("Internal server error: {}", e))
        }
    }
}

/// Handler for simple conversion (no rules)
pub async fn simple_handler(
    req: HttpRequest,
    path: web::Path<(String,)>,
    query: web::Query<SubconverterQuery>,
) -> HttpResponse {
    let target_type = &path.0;
    let req_url = req.uri().to_string();

    // Set appropriate target based on path
    match target_type.as_str() {
        "clash" | "clashr" | "surge" | "quan" | "quanx" | "loon" | "ss" | "ssr" | "ssd"
        | "v2ray" | "trojan" | "mixed" | "singbox" => {
            // Create a modified query with the target set
            let mut modified_query = query.into_inner();
            modified_query.target = Some(target_type.clone());

            // Reuse the sub_handler logic
            match sub_process(Some(req_url), modified_query).await {
                Ok(response) => response.to_http_response(),
                Err(e) => {
                    error!("Subconverter process error: {}", e);
                    HttpResponse::InternalServerError()
                        .body(format!("Internal server error: {}", e))
                }
            }
        }
        _ => HttpResponse::BadRequest().body(format!("Unsupported target type: {}", target_type)),
    }
}

/// Handler for Clash from Surge configuration
pub async fn surge_to_clash_handler(
    req: HttpRequest,
    query: web::Query<SubconverterQuery>,
) -> HttpResponse {
    let req_url = req.uri().to_string();

    // Create a modified query with the target set to Clash
    let mut modified_query = query.into_inner();
    modified_query.target = Some("clash".to_string());

    // Set nodelist to true for this special case
    modified_query.list = Some(true);

    // Reuse the sub_process logic
    match sub_process(Some(req_url), modified_query).await {
        Ok(response) => response.to_http_response(),
        Err(e) => {
            error!("Subconverter process error: {}", e);
            HttpResponse::InternalServerError().body(format!("Internal server error: {}", e))
        }
    }
}

/// Register the API endpoints with Actix Web
pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("/version", web::get().to(version_handler))
        .route("/sub", web::get().to(sub_handler))
        .route("/surge2clash", web::get().to(surge_to_clash_handler))
        .route("/getprofile", web::get().to(profile_handler))
        .route("/getruleset", web::get().to(ruleset_handler))
        .route("/{target_type}", web::get().to(simple_handler));
}

use crate::generator::config::group::group_generate;
use crate::generator::config::remark::process_remark;
use crate::generator::ruleconvert::convert_ruleset::convert_ruleset;
use crate::generator::ruleconvert::ruleset_to_clash_str;
use crate::generator::yaml::clash::clash_output::ClashProxyOutput;
use crate::generator::yaml::proxy_group_output::convert_proxy_groups;
use crate::models::{ExtraSettings, Proxy, ProxyGroupConfigs, ProxyType, RulesetContent};
use crate::utils::base64::url_safe_base64_encode;
use log::error;
use serde_yaml::{self, Mapping, Sequence, Value as YamlValue};
use std::collections::{HashMap, HashSet};

// Lists of supported protocols and encryption methods for filtering in ClashR
lazy_static::lazy_static! {
    static ref CLASH_SSR_CIPHERS: HashSet<&'static str> = {
        let mut ciphers = HashSet::new();
        ciphers.insert("aes-128-cfb");
        ciphers.insert("aes-192-cfb");
        ciphers.insert("aes-256-cfb");
        ciphers.insert("aes-128-ctr");
        ciphers.insert("aes-192-ctr");
        ciphers.insert("aes-256-ctr");
        ciphers.insert("aes-128-ofb");
        ciphers.insert("aes-192-ofb");
        ciphers.insert("aes-256-ofb");
        ciphers.insert("des-cfb");
        ciphers.insert("bf-cfb");
        ciphers.insert("cast5-cfb");
        ciphers.insert("rc4-md5");
        ciphers.insert("chacha20");
        ciphers.insert("chacha20-ietf");
        ciphers.insert("salsa20");
        ciphers.insert("camellia-128-cfb");
        ciphers.insert("camellia-192-cfb");
        ciphers.insert("camellia-256-cfb");
        ciphers.insert("idea-cfb");
        ciphers.insert("rc2-cfb");
        ciphers.insert("seed-cfb");
        ciphers
    };

    static ref CLASHR_PROTOCOLS: HashSet<&'static str> = {
        let mut protocols = HashSet::new();
        protocols.insert("origin");
        protocols.insert("auth_sha1_v4");
        protocols.insert("auth_aes128_md5");
        protocols.insert("auth_aes128_sha1");
        protocols.insert("auth_chain_a");
        protocols.insert("auth_chain_b");
        protocols
    };

    static ref CLASHR_OBFS: HashSet<&'static str> = {
        let mut obfs = HashSet::new();
        obfs.insert("plain");
        obfs.insert("http_simple");
        obfs.insert("http_post");
        obfs.insert("random_head");
        obfs.insert("tls1.2_ticket_auth");
        obfs.insert("tls1.2_ticket_fastauth");
        obfs
    };
}

/// Convert proxies to Clash format
///
/// This function converts a list of proxies to the Clash configuration format,
/// using a base configuration as a template and applying rules from ruleset_content_array.
///
/// # Arguments
/// * `nodes` - List of proxy nodes to convert
/// * `base_conf` - Base Clash configuration as a string
/// * `ruleset_content_array` - Array of ruleset contents to apply
/// * `extra_proxy_group` - Extra proxy group configurations
/// * `clash_r` - Whether to use ClashR format
/// * `ext` - Extra settings for conversion
pub fn proxy_to_clash(
    nodes: &mut Vec<Proxy>,
    base_conf: &str,
    ruleset_content_array: &mut Vec<RulesetContent>,
    extra_proxy_group: &ProxyGroupConfigs,
    clash_r: bool,
    ext: &mut ExtraSettings,
) -> String {
    // Parse the base configuration
    let mut yaml_node: YamlValue = match serde_yaml::from_str(base_conf) {
        Ok(node) => node,
        Err(e) => {
            error!("Clash base loader failed with error: {}", e);
            return String::new();
        }
    };

    if yaml_node.is_null() {
        yaml_node = YamlValue::Mapping(Mapping::new());
    }

    // Apply conversion to the YAML node
    proxy_to_clash_yaml(
        nodes,
        &mut yaml_node,
        ruleset_content_array,
        extra_proxy_group,
        clash_r,
        ext,
    );

    // If nodelist mode is enabled, just return the YAML node
    if ext.nodelist {
        return match serde_yaml::to_string(&yaml_node) {
            Ok(result) => result,
            Err(_) => String::new(),
        };
    }

    // Handle rule generation if enabled
    if !ext.enable_rule_generator {
        return match serde_yaml::to_string(&yaml_node) {
            Ok(result) => result,
            Err(_) => String::new(),
        };
    }

    // Handle clash script mode
    if ext.clash_script {
        // Set mode if it exists
        if yaml_node.get("mode").is_some() {
            if let Some(ref mut map) = yaml_node.as_mapping_mut() {
                map.insert(
                    YamlValue::String("mode".to_string()),
                    YamlValue::String(
                        if ext.clash_script {
                            if ext.clash_new_field_name {
                                "script"
                            } else {
                                "Script"
                            }
                        } else {
                            "Rule"
                        }
                        .to_string(),
                    ),
                );
            }
        }

        if !ext.managed_config_prefix.is_empty() {
            let (rule_providers, script_code) =
                build_clash_script_parts(ruleset_content_array, &ext.managed_config_prefix, 86400);

            if let Some(map) = yaml_node.as_mapping_mut() {
                map.insert(
                    YamlValue::String("rule-providers".to_string()),
                    YamlValue::Mapping(rule_providers),
                );
                let mut script_map = Mapping::new();
                script_map.insert(
                    YamlValue::String("code".to_string()),
                    YamlValue::String(script_code),
                );
                map.insert(
                    YamlValue::String("script".to_string()),
                    YamlValue::Mapping(script_map),
                );
            }
        }

        return match serde_yaml::to_string(&yaml_node) {
            Ok(result) => result,
            Err(_) => String::new(),
        };
    }

    // Generate rules and return combined output
    if let Some(map) = yaml_node.as_mapping_mut() {
        for key in ["rules", "Rule"] {
            let yaml_key = YamlValue::String(key.to_string());
            if map.get(&yaml_key).is_some_and(|v| v.is_null()) {
                map.remove(&yaml_key);
            }
        }
    }

    let rules_str = ruleset_to_clash_str(
        &yaml_node,
        ruleset_content_array,
        ext.overwrite_original_rules,
        ext.clash_new_field_name,
    );

    let yaml_output = match serde_yaml::to_string(&yaml_node) {
        Ok(result) => result,
        Err(_) => String::new(),
    };

    format!("{}{}", yaml_output, rules_str)
}

#[derive(Clone)]
struct ScriptRuleProvider {
    name: String,
    behavior: &'static str,
    request_type: u8,
    group: String,
    label: &'static str,
    typed_path: String,
    interval: u32,
}

#[derive(Clone, Default)]
struct ScriptRuleLayout {
    classical: Option<ScriptRuleProvider>,
    domain: Option<ScriptRuleProvider>,
    ipcidr: Option<ScriptRuleProvider>,
}

fn build_clash_script_parts(
    ruleset_content_array: &[RulesetContent],
    managed_config_prefix: &str,
    default_interval: u32,
) -> (Mapping, String) {
    let mut providers = Vec::<ScriptRuleProvider>::new();
    let mut layouts = Vec::<ScriptRuleLayout>::new();
    let mut geoips: Vec<(String, String)> = Vec::new();
    let mut final_group = "DIRECT".to_string();

    for ruleset in ruleset_content_array {
        let content = ruleset.get_rule_content();
        if content.is_empty() {
            continue;
        }

        if content.starts_with("[]") {
            let inline = content[2..].trim();
            if inline.starts_with("GEOIP,") {
                let mut parts = inline.split(',');
                let _ = parts.next();
                if let Some(code) = parts.next() {
                    geoips.push((code.trim().to_string(), ruleset.group.clone()));
                }
            } else if inline == "FINAL" || inline == "MATCH" {
                final_group = ruleset.group.clone();
            }
            continue;
        }

        let converted = convert_ruleset(&content, ruleset.rule_type);
        if converted.trim().is_empty() {
            continue;
        }

        let mut has_domain = false;
        let mut has_ipcidr = false;

        for raw in converted.lines() {
            let line = raw.trim();
            if line.is_empty()
                || line.starts_with('#')
                || line.starts_with(';')
                || line.starts_with("//")
            {
                continue;
            }
            let rule_type = line.split(',').next().unwrap_or("").trim();
            match rule_type {
                "DOMAIN" | "DOMAIN-SUFFIX" | "DOMAIN-KEYWORD" => has_domain = true,
                "IP-CIDR" => has_ipcidr = true,
                _ => {}
            }
        }

        let provider_base_name = ruleset
            .rule_path
            .rsplit('/')
            .next()
            .unwrap_or(&ruleset.rule_path)
            .strip_suffix(".list")
            .unwrap_or_else(|| {
                ruleset
                    .rule_path
                    .rsplit('/')
                    .next()
                    .unwrap_or(&ruleset.rule_path)
            })
            .to_string();

        let typed_path = ruleset.rule_path_typed.clone();
        let interval = if ruleset.update_interval > 0 {
            ruleset.update_interval
        } else {
            default_interval
        };

        let force_classical = provider_base_name == "MOO" || provider_base_name == "Download";
        if force_classical || (!has_domain && !has_ipcidr) {
            let provider = ScriptRuleProvider {
                name: provider_base_name,
                behavior: "classical",
                request_type: 6,
                group: ruleset.group.clone(),
                label: "rule",
                typed_path,
                interval,
            };
            providers.push(provider.clone());
            layouts.push(ScriptRuleLayout {
                classical: Some(provider),
                ..Default::default()
            });
            continue;
        }

        let mut layout = ScriptRuleLayout::default();
        if has_domain {
            let provider = ScriptRuleProvider {
                name: format!("{}_domain", provider_base_name),
                behavior: "domain",
                request_type: 3,
                group: ruleset.group.clone(),
                label: "DOMAIN rule",
                typed_path: typed_path.clone(),
                interval,
            };
            providers.push(provider.clone());
            layout.domain = Some(provider);
        }
        if has_ipcidr && provider_base_name != "Apple" {
            let provider = ScriptRuleProvider {
                name: format!("{}_ipcidr", provider_base_name),
                behavior: "ipcidr",
                request_type: 4,
                group: ruleset.group.clone(),
                label: "IP rule",
                typed_path,
                interval,
            };
            providers.push(provider.clone());
            layout.ipcidr = Some(provider);
        }
        layouts.push(layout);
    }

    let mut providers_map = Mapping::new();
    for p in &providers {
        let mut item = Mapping::new();
        item.insert(
            YamlValue::String("type".to_string()),
            YamlValue::String("http".to_string()),
        );
        item.insert(
            YamlValue::String("behavior".to_string()),
            YamlValue::String(p.behavior.to_string()),
        );
        item.insert(
            YamlValue::String("url".to_string()),
            YamlValue::String(format!(
                "{}/getruleset?type={}&url={}",
                managed_config_prefix,
                p.request_type,
                url_safe_base64_encode(&p.typed_path)
            )),
        );
        item.insert(
            YamlValue::String("path".to_string()),
            YamlValue::String(format!("./providers/rule-provider_{}.yaml", p.name)),
        );
        item.insert(
            YamlValue::String("interval".to_string()),
            YamlValue::Number(serde_yaml::Number::from(p.interval as i64)),
        );
        providers_map.insert(YamlValue::String(p.name.clone()), YamlValue::Mapping(item));
    }

    let mut code = String::from("def main(ctx, md):\n  host = md[\"host\"]\n\n");
    for layout in &layouts {
        if let Some(p) = &layout.classical {
            code.push_str(&format!(
                "  if ctx.rule_providers[\"{}\"].match(md):\n    ctx.log('[Script] matched {} {}')\n    return \"{}\"\n\n",
                p.name, p.group, p.label, p.group
            ));
            continue;
        }

        if let Some(p) = &layout.domain {
            code.push_str(&format!(
                "  if ctx.rule_providers[\"{}\"].match(md):\n    ctx.log('[Script] matched {} {}')\n    return \"{}\"\n\n",
                p.name, p.group, p.label, p.group
            ));
        } else {
            code.push_str("\n\n");
        }

        if let Some(p) = &layout.ipcidr {
            code.push_str(&format!(
                "  if ctx.rule_providers[\"{}\"].match(md):\n    ctx.log('[Script] matched {} {}')\n    return \"{}\"\n\n",
                p.name, p.group, p.label, p.group
            ));
        } else {
            code.push_str("\n\n");
        }
    }

    code.push('\n');
    code.push_str("  geoips = {");
    if !geoips.is_empty() {
        code.push(' ');
        for (idx, (code_name, group)) in geoips.iter().enumerate() {
            if idx > 0 {
                code.push_str(", ");
            }
            code.push_str(&format!("\"{}\": \"{}\"", code_name, group));
        }
        code.push(' ');
    }
    code.push_str("}\n");
    code.push_str(
        "  ip = md[\"dst_ip\"]\n  if ip == \"\":\n    ip = ctx.resolve_ip(host)\n    if ip == \"\":\n      ctx.log('[Script] dns lookup error use ",
    );
    code.push_str(&final_group);
    code.push_str("')\n      return \"");
    code.push_str(&final_group);
    code.push_str("\"\n  for key in geoips:\n    if ctx.geoip(ip) == key:\n      return geoips[key]\n  return \"");
    code.push_str(&final_group);
    code.push('"');

    (providers_map, code)
}

/// Convert proxies to Clash format with YAML node
///
/// This function modifies a YAML node in place to add Clash configuration
/// for the provided proxy nodes.
///
/// # Arguments
/// * `nodes` - List of proxy nodes to convert
/// * `yaml_node` - YAML node to modify
/// * `ruleset_content_array` - Array of ruleset contents to apply
/// * `extra_proxy_group` - Extra proxy group configurations
/// * `clash_r` - Whether to use ClashR format
/// * `ext` - Extra settings for conversion
pub fn proxy_to_clash_yaml(
    nodes: &mut Vec<Proxy>,
    yaml_node: &mut serde_yaml::Value,
    _ruleset_content_array: &Vec<RulesetContent>,
    extra_proxy_group: &ProxyGroupConfigs,
    clash_r: bool,
    ext: &mut ExtraSettings,
) {
    // Style settings - in C++ this is used to set serialization style but in Rust we have less control
    // over the serialization format. We keep them for compatibility but their actual effect may differ.
    let _proxy_block = ext.clash_proxies_style == "block";
    let _proxy_compact = ext.clash_proxies_style == "compact";
    let _group_block = ext.clash_proxy_groups_style == "block";
    let _group_compact = ext.clash_proxy_groups_style == "compact";

    // Create JSON structure for the proxies
    let mut proxies_json = Vec::new();
    let mut remarks_list = Vec::new();

    // Process each node
    for node in nodes.iter_mut() {
        // Create a local copy of the node for processing
        let mut remark = node.remark.clone();

        // Add proxy type prefix if enabled
        if ext.append_proxy_type {
            remark = format!("[{}] {}", node.proxy_type.to_string(), remark);
        }

        // Process remark with optional remarks list
        process_remark(&mut remark, &remarks_list, false);
        remarks_list.push(remark.clone());
        // Check if this proxy type should be skipped
        let should_skip = match node.proxy_type {
            // Skip Snell v4+ if exists - exactly matching C++ behavior
            ProxyType::Snell if node.snell_version >= 4 => {
                error!("Skipping Snell v4+ node: {}", remark);
                true
            }

            // Skip chacha20 encryption if filter_deprecated is enabled
            ProxyType::Shadowsocks
                if ext.filter_deprecated && node.encrypt_method.as_deref() == Some("chacha20") =>
            {
                error!(
                    "Skipping SS chacha20 node (filter_deprecated=true): {}",
                    remark
                );
                true
            }

            // Skip ShadowsocksR with deprecated features if filter_deprecated is enabled
            ProxyType::ShadowsocksR if ext.filter_deprecated => {
                let encrypt_method = node.encrypt_method.as_deref().unwrap_or("");
                let protocol = node.protocol.as_deref().unwrap_or("");
                let obfs = node.obfs.as_deref().unwrap_or("");

                if (!clash_r && !CLASH_SSR_CIPHERS.contains(encrypt_method))
                    || !CLASHR_PROTOCOLS.contains(protocol)
                    || !CLASHR_OBFS.contains(obfs)
                {
                    error!("Skipping SSR deprecated features node: {}", remark);
                    true
                } else {
                    false
                }
            }

            // Skip unsupported proxy types
            ProxyType::Unknown | ProxyType::HTTPS => {
                error!(
                    "Skipping Unknown/HTTPS node: {} (type: {:?})",
                    remark, node.proxy_type
                );
                true
            }

            // Process all other types
            _ => false,
        };

        if should_skip {
            continue;
        }

        // 创建代理副本，并应用所有必要的属性设置
        let proxy_copy = node.clone().set_remark(remark).apply_default_values(
            ext.udp,
            ext.tfo,
            ext.skip_cert_verify,
        );

        // 使用 From trait 自动转换为 ClashProxyOutput
        let clash_proxy = ClashProxyOutput::from(proxy_copy);

        // 添加到代理列表
        proxies_json.push(clash_proxy);
    }

    if ext.nodelist {
        let mut provider = YamlValue::Mapping(Mapping::new());
        provider["proxies"] =
            serde_yaml::to_value(&proxies_json).unwrap_or(YamlValue::Sequence(Vec::new()));
        *yaml_node = provider;
        return;
    }

    // Update the YAML node with proxies
    if let Some(ref mut map) = yaml_node.as_mapping_mut() {
        // Convert JSON proxies array to YAML
        let proxies_yaml_value =
            serde_yaml::to_value(&proxies_json).unwrap_or(YamlValue::Sequence(Vec::new()));
        if ext.clash_new_field_name {
            map.insert(YamlValue::String("proxies".to_string()), proxies_yaml_value);
        } else {
            map.insert(YamlValue::String("Proxy".to_string()), proxies_yaml_value);
        }
    }

    // Add proxy groups if present
    if !extra_proxy_group.is_empty() {
        // Get existing proxy groups if any
        let mut original_groups = if ext.clash_new_field_name {
            match yaml_node.get("proxy-groups") {
                Some(YamlValue::Sequence(seq)) => seq.clone(),
                _ => Sequence::new(),
            }
        } else {
            match yaml_node.get("Proxy Group") {
                Some(YamlValue::Sequence(seq)) => seq.clone(),
                _ => Sequence::new(),
            }
        };

        // Build filtered nodes map for each group
        let mut filtered_nodes_map = HashMap::new();
        for group in extra_proxy_group {
            let mut filtered_nodes = Vec::new();
            for proxy_name in &group.proxies {
                group_generate(proxy_name, nodes, &mut filtered_nodes, true, ext);
            }

            // Add DIRECT if empty
            if filtered_nodes.is_empty() && group.using_provider.is_empty() {
                filtered_nodes.push("DIRECT".to_string());
            }

            filtered_nodes_map.insert(group.name.clone(), filtered_nodes);
        }

        // Convert proxy groups using the new serialization
        let clash_proxy_groups = convert_proxy_groups(extra_proxy_group, Some(&filtered_nodes_map));

        // Merge with existing groups
        for group in clash_proxy_groups {
            // Check if this group should replace an existing one with the same name
            let mut replaced = false;
            for i in 0..original_groups.len() {
                if let Some(YamlValue::Mapping(map)) = original_groups.get(i) {
                    if let Some(YamlValue::String(name)) =
                        map.get(&YamlValue::String("name".to_string()))
                    {
                        if name == &group.name {
                            if let Some(elem) = original_groups.get_mut(i) {
                                // Convert the group to YAML and replace
                                if let Ok(group_yaml) = serde_yaml::to_value(&group) {
                                    *elem = group_yaml;
                                    replaced = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            // If not replaced, add to the list
            if !replaced {
                if let Ok(group_yaml) = serde_yaml::to_value(&group) {
                    original_groups.push(group_yaml);
                }
            }
        }

        // Update the YAML node with proxy groups
        if let Some(ref mut map) = yaml_node.as_mapping_mut() {
            if ext.clash_new_field_name {
                map.insert(
                    YamlValue::String("proxy-groups".to_string()),
                    YamlValue::Sequence(original_groups),
                );
            } else {
                map.insert(
                    YamlValue::String("Proxy Group".to_string()),
                    YamlValue::Sequence(original_groups),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_ssr_proxy(name: &str, cipher: &str, protocol: &str, obfs: &str) -> Proxy {
        Proxy {
            proxy_type: ProxyType::ShadowsocksR,
            remark: name.to_string(),
            hostname: "example.com".to_string(),
            port: 443,
            encrypt_method: Some(cipher.to_string()),
            password: Some("pwd".to_string()),
            protocol: Some(protocol.to_string()),
            obfs: Some(obfs.to_string()),
            ..Default::default()
        }
    }

    fn build_ss_proxy(name: &str, cipher: &str) -> Proxy {
        Proxy {
            proxy_type: ProxyType::Shadowsocks,
            remark: name.to_string(),
            hostname: "example.com".to_string(),
            port: 443,
            encrypt_method: Some(cipher.to_string()),
            password: Some("pwd".to_string()),
            ..Default::default()
        }
    }

    fn extract_proxy_names(yaml_node: &YamlValue) -> Vec<String> {
        yaml_node
            .get("proxies")
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|item| item.get("name").and_then(|x| x.as_str()))
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    #[test]
    fn clash_with_filter_deprecated_keeps_supported_ssr() {
        let mut nodes = vec![build_ssr_proxy(
            "ssr-ok",
            "aes-256-cfb",
            "auth_aes128_sha1",
            "tls1.2_ticket_auth",
        )];
        let mut yaml_node = YamlValue::Mapping(Mapping::new());
        let mut ext = ExtraSettings {
            filter_deprecated: true,
            clash_new_field_name: true,
            ..Default::default()
        };

        proxy_to_clash_yaml(
            &mut nodes,
            &mut yaml_node,
            &vec![],
            &vec![],
            false,
            &mut ext,
        );

        let names = extract_proxy_names(&yaml_node);
        assert_eq!(names, vec!["ssr-ok".to_string()]);
    }

    #[test]
    fn clashr_with_filter_deprecated_allows_non_clash_cipher_ssr() {
        let mut nodes = vec![build_ssr_proxy(
            "ssr-clashr-only",
            "none",
            "auth_aes128_sha1",
            "tls1.2_ticket_auth",
        )];
        let mut yaml_node = YamlValue::Mapping(Mapping::new());
        let mut ext = ExtraSettings {
            filter_deprecated: true,
            clash_new_field_name: true,
            ..Default::default()
        };

        proxy_to_clash_yaml(&mut nodes, &mut yaml_node, &vec![], &vec![], true, &mut ext);

        let names = extract_proxy_names(&yaml_node);
        assert_eq!(names, vec!["ssr-clashr-only".to_string()]);
    }

    #[test]
    fn filter_deprecated_still_filters_chacha20_ss() {
        let mut nodes = vec![build_ss_proxy("ss-chacha20", "chacha20")];
        let mut yaml_node = YamlValue::Mapping(Mapping::new());
        let mut ext = ExtraSettings {
            filter_deprecated: true,
            clash_new_field_name: true,
            ..Default::default()
        };

        proxy_to_clash_yaml(
            &mut nodes,
            &mut yaml_node,
            &vec![],
            &vec![],
            false,
            &mut ext,
        );

        let names = extract_proxy_names(&yaml_node);
        assert!(names.is_empty());
    }
}

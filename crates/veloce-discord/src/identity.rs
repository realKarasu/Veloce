use base64::Engine;
use serde_json::json;

/// User-Agent mimant un navigateur récent. À maintenir si Discord durcit ses contrôles.
pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
     (KHTML, like Gecko) discord/1.0.9028 Chrome/120.0.0.0 Electron/28.0.0 Safari/537.36";

/// Propriétés client envoyées via X-Super-Properties et dans IDENTIFY.
/// `client_build_number` évolue côté Discord ; le mettre à jour ici uniquement.
pub fn super_properties_json() -> serde_json::Value {
    json!({
        "os": "Windows",
        "browser": "Discord Client",
        "release_channel": "stable",
        "client_version": "1.0.9028",
        "os_version": "10.0.19045",
        "system_locale": "fr",
        "client_build_number": 9999,
        "native_build_number": 9999
    })
}

pub fn super_properties_b64() -> String {
    let s = serde_json::to_string(&super_properties_json()).expect("json valide");
    base64::engine::general_purpose::STANDARD.encode(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    #[test]
    fn super_properties_b64_se_decode_en_json_valide() {
        let b64 = super_properties_b64();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(v.get("os").is_some());
        assert!(v.get("browser").is_some());
        assert!(v.get("client_build_number").is_some());
    }

    #[test]
    fn user_agent_non_vide() {
        assert!(USER_AGENT.contains("Mozilla"));
    }
}

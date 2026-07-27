use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductInformation {
    pub product_name: &'static str,
    pub application_version: &'static str,
    pub prototype: bool,
    pub operating_system: &'static str,
    pub architecture: &'static str,
    pub backend_version: String,
}

#[tauri::command]
pub fn get_product_information() -> ProductInformation {
    ProductInformation {
        product_name: "Mam Voice Changer",
        application_version: env!("CARGO_PKG_VERSION"),
        prototype: true,
        operating_system: std::env::consts::OS,
        architecture: std::env::consts::ARCH,
        backend_version: format!("mam-voice-changer-rust {}", env!("CARGO_PKG_VERSION")),
    }
}

#[cfg(test)]
mod tests {
    use super::get_product_information;

    #[test]
    fn product_information_is_path_free_and_versioned() {
        let information = get_product_information();
        assert_eq!(information.product_name, "Mam Voice Changer");
        assert!(!information.application_version.is_empty());
        assert!(!information.operating_system.contains(['\\', '/']));
        assert!(!information.backend_version.contains(['\\', '/']));
    }
}

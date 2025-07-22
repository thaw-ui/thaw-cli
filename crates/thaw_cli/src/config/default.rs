pub fn default_public_dir() -> String {
    "public".to_string()
}

pub mod server {
    pub fn default_host() -> String {
        "localhost".to_string()
    }

    pub fn default_port() -> u32 {
        6321
    }

    pub fn default_erase_components() -> bool {
        false
    }
}

pub mod build {
    pub fn default_out_dir() -> String {
        "dist".to_string()
    }

    pub fn default_assets_dir() -> String {
        "assets".to_string()
    }

    pub fn default_assets_manganis() -> bool {
        true
    }
}

use crate::message_box;
use std::{error::Error, path::Path};

#[derive(Debug, Clone)]
pub struct DbDataConfig {
    pub app_id: u32,
    pub email: String,
    pub password: String,
}

impl DbDataConfig {
    pub fn load(base: &Path) -> Result<Self, Box<dyn Error>> {
        let ini_path = base.join("dbdata.ini");

        let ini = ini::Ini::load_from_file(&ini_path)
            .map_err(|e| format!("Failed to load dbdata.ini: {}", e))?;

        let section = ini
            .section(Some("Uplay"))
            .ok_or("Missing [Uplay] section in dbdata.ini")?;

        let email = section
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("email"))
            .map(|(_, v)| v.to_string())
            .unwrap_or_default();

        let password = section
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("password"))
            .map(|(_, v)| v.to_string())
            .unwrap_or_default();

        log::info!(
            "Loaded config: email={}, password_len={}",
            if email.is_empty() { "<empty>" } else { "<set>" },
            password.len()
        );

        Ok(Self {
            app_id: 0,
            email,
            password,
        })
    }

    pub fn has_credentials(&self) -> bool {
        !self.email.is_empty() && !self.password.is_empty()
    }

    pub fn exists(base: &Path) -> bool {
        base.join("dbdata.ini").exists()
    }

    pub fn create_default(base: &Path) -> Result<(), Box<dyn Error>> {
        let ini_path = base.join("dbdata.ini");
        let content = r#"[Uplay]
email=
password=
[token]
token=
ownership=
[settings]
dlcs=
"#;

        std::fs::write(&ini_path, content)?;
        log::info!("Created default dbdata.ini at {:?}", ini_path);
        Ok(())
    }
}

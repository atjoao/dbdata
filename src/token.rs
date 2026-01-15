use std::{error::Error, path::Path};

#[derive(Debug, Clone)]
pub struct Token {
    pub token: String,
    pub ownership: Option<String>,
}

impl Token {
    pub fn new(base: &Path) -> Result<Self, Box<dyn Error>> {
        let ini = ini::Ini::load_from_file(base.join("dbdata.ini"))?;

        Ok(Self {
            token: ini
                .section(Some("token"))
                .and_then(|sec| sec.get("token"))
                .ok_or("Token not found in dbdata.ini file")?
                .to_string(),
            ownership: ini
                .section(Some("token"))
                .and_then(|sec| sec.get("ownership"))
                .map(|s| s.to_string()),
        })
    }

    pub fn from_values(token: String, ownership: Option<String>) -> Self {
        Self { token, ownership }
    }

    pub fn save_with_dlcs(&self, base: &Path, dlcs: &[u32]) -> Result<(), Box<dyn Error>> {
        let mut ini = ini::Ini::new();

        ini.with_section(Some("token")).set("token", &self.token);

        if let Some(ref ownership) = self.ownership {
            ini.with_section(Some("token")).set("ownership", ownership);
        }

        if !dlcs.is_empty() {
            let dlcs_str = dlcs
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join(",");
            ini.with_section(Some("settings")).set("dlcs", dlcs_str);
        }

        ini.write_to_file(base.join("dbdata.ini"))?;
        log::info!("Saved tokens and {} DLCs to dbdata.ini", dlcs.len());

        Ok(())
    }
}

#[derive(Debug)]
pub struct Settings {
    pub dlcs: Vec<u32>,
    pub token: Token,
}

impl Settings {
    pub fn new(base: &Path) -> Result<Self, Box<dyn Error>> {
        let token = Token::new(base)?;

        match ini::Ini::load_from_file(base.join("dbdata.ini")) {
            Ok(settings) => {
                return Ok(Self {
                    dlcs: settings
                        .section(Some("settings"))
                        .and_then(|sec| sec.get("dlcs"))
                        .map(|s| {
                            s.split(',')
                                .filter_map(|d| d.trim().parse::<u32>().ok())
                                .collect()
                        })
                        .unwrap_or_else(|| vec![]),
                    token,
                });
            }
            Err(e) => {
                log::warn!("Could not read dbdata.ini: {}", e);

                if let Ok(content) = std::fs::read_to_string(base.join("upc_r2.ini")) {
                    let lines = content
                        .lines()
                        .skip_while(|line| !line.trim().eq_ignore_ascii_case("[DLC]"))
                        .skip(1)
                        .take_while(|line| !line.trim().is_empty())
                        .filter_map(|line| line.trim().parse::<u32>().ok())
                        .collect::<Vec<u32>>();

                    return Ok(Self { dlcs: lines, token });
                }
            }
        }

        Ok(Self {
            dlcs: vec![],
            token,
        })
    }
}

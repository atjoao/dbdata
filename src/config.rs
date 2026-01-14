use crate::message_box;
use std::{error::Error, path::Path};

#[derive(Debug, Clone)]
pub struct UplayConfig {
    pub app_id: u32,
    pub email: String,
    pub password: String,
}

impl UplayConfig {
    pub fn load(base: &Path) -> Result<Self, Box<dyn Error>> {
        let ini_path = base.join("uplay.ini");

        let ini = ini::Ini::load_from_file(&ini_path)
            .map_err(|e| format!("Failed to load uplay.ini: {}", e))?;

        let section = ini
            .section(Some("Uplay"))
            .ok_or("Missing [Uplay] section in uplay.ini")?;

        let email = section
            .get("Email")
            .ok_or("Missing Email in uplay.ini")?
            .to_string();

        let password = section
            .get("Password")
            .ok_or("Missing Password in uplay.ini")?
            .to_string();

        if email.is_empty() || password.is_empty() {
            message_box(
                "Setup Required",
                "Please edit 'uplay.ini' with your Ubisoft account credentials and restart the game.",
            );
            std::process::exit(0);
        }

        if email == "UplayEmu@rat43.com" || password == "UplayPassword74" {
            message_box(
                "Setup Required",
                "Please edit 'uplay.ini' with your Ubisoft account credentials and restart the game.",
            );
            std::process::exit(0);
        }

        Ok(Self {
            app_id: 0,
            email,
            password,
        })
    }

    pub fn exists(base: &Path) -> bool {
        base.join("uplay.ini").exists()
    }

    pub fn create_default(base: &Path, app_id: u32) -> Result<(), Box<dyn Error>> {
        let ini_path = base.join("uplay.ini");
        let content = format!(
            r#"[Uplay]
; Application ownership status (0 = not owned, 1 = owned)
IsAppOwned=1
; Connection mode (0 = online, 1 = offline)
UplayConnection=0
; Application ID (change this to match your game's App ID)
AppId={}
; User credential
Username=Rat
Email=UplayEmu@rat43.com
Password=UplayPassword74
; Game language (ISO language code)
Language=en-US
; CD Key for the game
CdKey=1111-2222-3333-4444
; User ID (UUID format)
UserId=c91c91c9-1c91-c91c-91c9-1c91c91c91c9
; Ticket ID for authentication
TicketId=noT456umPqRt

; Enable logging to uplay_emu.log or Console (0 = disabled, 1 = enabled)
Logging=0
EnableConsole=0

; Enable Friends/Party features (0 = disabled, 1 = enabled)
Friends=0
Party=0

; Steam integration 
; (if the game has this by default it wont be disabled.)
; (you can check this by checking if a game starts steam on exe open)
[Steam]
Enable=0
Id=0
"#,
            app_id
        );

        std::fs::write(&ini_path, content)?;
        log::info!("Created default uplay.ini at {:?}", ini_path);
        Ok(())
    }
}

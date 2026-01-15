mod auth;
mod config;
mod logging;
mod proto;
mod services;
mod token;

use std::{
    ffi::{CString, OsString, c_void},
    os::windows::ffi::OsStringExt,
    path::{Path, PathBuf},
    sync::{OnceLock, RwLock},
};

use winapi::{
    shared::minwindef::{DWORD, HINSTANCE, LPVOID},
    um::{libloaderapi::GetModuleFileNameW, winnt::DLL_PROCESS_ATTACH, winuser::MessageBoxA},
};

use crate::config::UplayConfig;
use crate::token::{Settings, Token};

static DLL_PATH: OnceLock<PathBuf> = OnceLock::new();
static APP_ID: OnceLock<u32> = OnceLock::new();
static SETTINGS: RwLock<Option<Settings>> = RwLock::new(None);
static UPLAY_CONFIG: OnceLock<Option<UplayConfig>> = OnceLock::new();

#[unsafe(no_mangle)]
extern "system" fn DllMain(module: HINSTANCE, reason: DWORD, _reserved: LPVOID) -> bool {
    match reason {
        DLL_PROCESS_ATTACH => {
            let dll_path = {
                let mut buffer = [0u16; 1024];
                let len =
                    unsafe { GetModuleFileNameW(module, buffer.as_mut_ptr(), buffer.len() as u32) };
                let path = OsString::from_wide(&buffer[..len as usize])
                    .into_string()
                    .unwrap();
                let path = Path::new(&path);
                path.parent().unwrap().to_path_buf()
            };
            DLL_PATH.set(dll_path.clone()).ok();

            logging::init_logger();
            logging::setup_panic_handler();

            if let Ok(settings) = Settings::new(&dll_path) {
                if let Ok(mut s) = SETTINGS.write() {
                    *s = Some(settings);
                }
            } else {
                log::info!("Could not read dbdata.ini - will try online auth if uplay.ini exists");
            }

            let uplay_config = UplayConfig::load(&dll_path)
                .map_err(|e| {
                    log::info!("Could not read uplay.ini: {}", e);
                })
                .ok();
            UPLAY_CONFIG.set(uplay_config).ok();
        }
        _ => {}
    }

    true
}

#[repr(C)]
pub struct IGameTokenInterface {
    vtable: *const IGameTokenInterfaceVtable,
}

#[repr(C, align(32))]
pub struct IGameTokenInterfaceVtable {
    is_token_loaded: *const c_void,
    return_0: *const c_void,
    get_cached_or_fresh_token: *const c_void,
    invalidate_cached_token: *const c_void,
    get_buffer: *const c_void,
    new_thread_get_burn_ticket_res: *const c_void,
    get_thread: *const c_void,
    get_ownership_buffer: *const c_void,
    get_dlcs: *const c_void,
    set_arg_to_0: *const c_void,
}

#[unsafe(export_name = "?getGameTokenInterface@@YAPEAVIGameTokenInterface@@PEAX_K@Z")]
pub extern "C" fn get_game_token_interface(
    app_id: *const i64,
    version: i64,
) -> *const IGameTokenInterface {
    let app_id = unsafe { *app_id };

    APP_ID.set(app_id as u32).ok();

    log::info!("getGameTokenInterface called {:?} {:?}", app_id, version);

    let vtable = Box::new(IGameTokenInterfaceVtable {
        is_token_loaded: is_token_loaded as *const c_void,
        return_0: return_0 as *const c_void,
        get_cached_or_fresh_token: get_cached_or_fresh_token as *const c_void,
        invalidate_cached_token: invalidate_cached_token as *const c_void,
        get_buffer: get_buffer as *const c_void,
        new_thread_get_burn_ticket_res: new_thread_get_burn_ticket_res as *const c_void,
        get_thread: get_thread as *const c_void,
        get_ownership_buffer: get_ownership_buffer as *const c_void,
        get_dlcs: get_dlcs as *const c_void,
        set_arg_to_0: set_arg_to_0 as *const c_void,
    });

    let example = Box::new(IGameTokenInterface {
        vtable: Box::into_raw(vtable),
    });

    Box::into_raw(example)
}

fn is_token_loaded(this: *const IGameTokenInterface) -> u32 {
    log::info!("is_token_loaded called {:?}", this);

    1
}

fn return_0() -> i64 {
    log::info!("return_0 called");

    0
}

fn get_cached_or_fresh_token(
    this: *mut IGameTokenInterface,
    token_buffer_ptr: *const c_void,
    length: i32,
) -> bool {
    log::info!(
        "get_cached_or_fresh_token called {:?} {:?} {:?}",
        this,
        token_buffer_ptr,
        length
    );

    let has_settings = SETTINGS.read().map(|s| s.is_some()).unwrap_or(false);
    if has_settings {
        log::info!("Using existing token from dbdata.ini");
        return true;
    }

    let request_token = unsafe {
        std::ffi::CStr::from_ptr(token_buffer_ptr as *const i8)
            .to_str()
            .unwrap()
    };
    let dll_path = DLL_PATH.get().unwrap();
    let app_id = *APP_ID.get().unwrap();

    if let Some(Some(uplay_config)) = UPLAY_CONFIG.get() {
        log::info!("Attempting online authentication with Ubisoft");

        let mut config = uplay_config.clone();
        config.app_id = app_id;

        match auth::authenticate_and_get_tokens(&config, request_token, vec![]) {
            Ok(result) => {
                log::info!("Authentication successful, saving tokens");

                let token =
                    Token::from_values(result.game_token.clone(), result.ownership_token.clone());

                if let Err(e) = token.save_with_dlcs(dll_path, &result.owned_dlcs) {
                    log::error!("Failed to save tokens: {}", e);
                }

                if let Ok(mut settings) = SETTINGS.write() {
                    *settings = Some(Settings {
                        dlcs: result.owned_dlcs,
                        token,
                    });
                }

                log::info!("Online authentication complete, game can continue");
                return true;
            }
            Err(e) => {
                log::error!("Authentication failed: {}", e);
                message_box(
                    "Authentication Failed",
                    &format!(
                        "Failed to authenticate: {}\n\nFalling back to token_req.txt",
                        e
                    ),
                );
            }
        }
    } else {
        if !UplayConfig::exists(dll_path) {
            if let Err(e) = UplayConfig::create_default(dll_path, app_id) {
                log::error!("Failed to create default uplay.ini: {}", e);
            } else {
                message_box(
                    "Setup Required",
                    "A default 'uplay.ini' has been created.\n\nPlease edit it with your Ubisoft account credentials and restart the game.",
                );
                std::process::exit(0);
            }
        }
    }

    let request_token = format!("{}|{}", request_token, app_id);
    let path = dll_path.join("token_req.txt");
    std::fs::write(path, request_token).unwrap();

    message_box(
        "Info",
        "Token request generated, please check the 'token_req.txt' file in the current directory.\nIf you already have a token, place it in a file named 'dbdata.ini' in the same directory.",
    );
    std::process::exit(0);
}

fn invalidate_cached_token(this: *const IGameTokenInterface) {
    log::info!("invalidate_cached_token called {:?}", this);

    message_box(
        "Info",
        "Denuvo tried to delete the token, if this error persists your token might have become invalid",
    );
}

fn get_buffer(this: *const IGameTokenInterface, length: *mut u64) -> *const c_void {
    log::info!("get_buffer called {:?} {:?}", this, length);

    let token = SETTINGS
        .read()
        .ok()
        .and_then(|s| s.as_ref().map(|s| s.token.token.clone()))
        .unwrap_or_else(|| {
            message_box("Error", "Token not found");
            "".to_string()
        });

    if !length.is_null() {
        unsafe {
            *length = token.len() as u64;
        }
    }

    Box::into_raw(token.into_boxed_str()) as *const c_void
}

fn new_thread_get_burn_ticket_res(this: *const IGameTokenInterface, param: i64) -> *const c_void {
    log::info!(
        "new_thread_get_burn_ticket_res called {:?} {:?}",
        this,
        param
    );

    std::ptr::null()
}

fn get_thread(this: *const IGameTokenInterface, param: *mut u64) -> *const c_void {
    log::info!("get_thread called {:?} {:?}", this, param);

    std::ptr::null()
}

fn get_ownership_buffer(this: *const IGameTokenInterface, length: *mut u64) -> *const c_void {
    log::info!("get_ownership_buffer called {:?} {:?}", this, length);

    let token = SETTINGS
        .read()
        .ok()
        .and_then(|s| s.as_ref().and_then(|t| t.token.ownership.clone()))
        .unwrap_or_else(|| "".to_string());

    if !length.is_null() {
        unsafe {
            *length = token.len() as u64;
        }
    }

    Box::into_raw(token.to_string().into_boxed_str()) as *const c_void
}

fn get_dlcs(this: *const IGameTokenInterface, arg: *mut i64) -> *const i32 {
    log::info!("get_dlcs called {:?} {:?}", this, arg);

    let dlcs = SETTINGS
        .read()
        .ok()
        .and_then(|s| s.as_ref().map(|s| s.dlcs.clone()))
        .unwrap_or_else(|| vec![]);

    if !arg.is_null() {
        unsafe {
            *arg = dlcs.len() as i64;
        }
    }

    Box::into_raw(dlcs.into_boxed_slice()) as *const i32
}

fn set_arg_to_0(this: *const IGameTokenInterface, arg: *mut u64) {
    log::info!("set_arg_to_0 called {:?} {:?}", this, arg);

    if !arg.is_null() {
        unsafe {
            *arg = 0;
        }
    }
}

pub fn message_box(title: &str, message: &str) {
    let title = CString::new(title).unwrap();
    let message = CString::new(message).unwrap();

    unsafe {
        MessageBoxA(
            std::ptr::null_mut(),
            message.as_ptr() as *const i8,
            title.as_ptr() as *const i8,
            0,
        );
    }
}

use {
    log::LevelFilter,
    log4rs::{
        Config,
        append::file::FileAppender,
        config::{Appender, Root},
        encode::pattern::PatternEncoder,
    },
    std::{panic, sync::Once},
    winapi::um::winuser::MessageBoxA,
};

static LOGGER: Once = Once::new();

pub(crate) fn init_logger() {
    LOGGER.call_once(|| {
        let log_file_path = "dbdata.log";

        let logfile = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(
                "[{d(%Y-%m-%dT%H:%M:%S%.3f)}] [{l}]: {m}{n}",
            )))
            .append(false)
            .build(log_file_path)
            .unwrap();

        let config = Config::builder()
            .appender(Appender::builder().build("logfile", Box::new(logfile)))
            .build(Root::builder().appender("logfile").build(LevelFilter::Info))
            .unwrap();

        let _handle = log4rs::init_config(config).unwrap();

        log::info!("Logger initialized");
    });
}

pub(crate) fn setup_panic_handler() {
    panic::set_hook(Box::new(|panic_info| {
        let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else {
            "Unknown panic message".to_string()
        };

        let location = if let Some(location) = panic_info.location() {
            format!("{}:{}", location.file(), location.line())
        } else {
            "unknown location".to_string()
        };

        unsafe {
            MessageBoxA(
                std::ptr::null_mut(),
                format!("Panic occurred at {}: {}\0", location, message).as_ptr() as *const i8,
                "Panic\0".as_ptr() as *const i8,
                0,
            );
        }

        log::error!("Panic occurred at {}: {}", location, message);
    }));
}

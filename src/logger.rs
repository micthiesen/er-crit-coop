use std::fs::File;

use simplelog::{ConfigBuilder, LevelFilter, WriteLogger};

/// Initialize file logging. The DLL runs inside the game's Proton prefix, so this
/// writes to `er_crit_coop.log` in the process working directory (normally the
/// `ELDEN RING/Game/` folder). The startup line records the actual cwd so it can
/// be located if Proton's cwd differs.
pub fn init() {
    let config = ConfigBuilder::new()
        .set_time_format_rfc3339()
        .build();

    if let Ok(file) = File::create("er_crit_coop.log") {
        let _ = WriteLogger::init(LevelFilter::Info, config, file);
    }

    // Don't let a panic in the worker thread abort the game without a trace.
    std::panic::set_hook(Box::new(|info| {
        log::error!("PANIC: {info}");
    }));

    log::info!("er-crit-coop loaded");
    if let Ok(cwd) = std::env::current_dir() {
        log::info!("cwd = {}", cwd.display());
    }
}

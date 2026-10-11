//! Native tablet launcher. Drawing behaviour stays in CADCraft's shared engine and UI.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

#[cfg(target_os = "android")]
mod platform;
#[cfg(any(target_os = "android", feature = "desktop-preview"))]
pub mod shell;

/// Android's native-activity loader calls this safe Rust entry on its own thread.
/// SAFETY: this cdylib defines exactly one `android_main` with android-activity's required ABI.
#[cfg(target_os = "android")]
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub fn android_main(app: winit::platform::android::activity::AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Info).with_tag("CADCraft"));
    let Some(dir) = app.internal_data_path() else {
        log::error!("Android did not provide an application data directory");
        return;
    };
    let mut options = eframe::NativeOptions {
        android_app: Some(app),
        persistence_path: Some(dir.join("preferences.ron")),
        persist_window: false,
        ..Default::default()
    };
    if let eframe::egui_wgpu::WgpuSetup::CreateNew(create) = &mut options.wgpu_options.wgpu_setup {
        // A Vulkan probe can leave Android's window bound when GLES is selected afterwards.
        // Use the GLES baseline directly, including on tablets without usable Vulkan drivers.
        create.instance_descriptor.backends = eframe::wgpu::Backends::GL;
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        eframe::run_native("CADCraft", options, Box::new(move |cc| shell::Shell::new(cc, dir).map(|app| Box::new(app) as Box<dyn eframe::App>)))
    }));
    match result {
        Ok(Ok(())) => {}
        Ok(Err(error)) => log::error!("CADCraft could not start: {error}"),
        Err(_) => log::error!("CADCraft's Android runner stopped unexpectedly; recovery files have been kept"),
    }
}

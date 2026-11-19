pub mod app;
pub mod worker;

pub use app::WeasApp;

pub fn run_gui() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([880.0, 640.0])
            .with_min_inner_size([700.0, 500.0])
            .with_title("WEAS VPN Client"),
        ..Default::default()
    };

    eframe::run_native(
        "WEAS VPN Client",
        options,
        Box::new(|cc| Ok(Box::new(WeasApp::new(cc)))),
    )
}

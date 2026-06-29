mod app;
mod cdn;
mod emoji;
mod fonts;
mod markdown;
mod net;
mod plugins;

use app::VeloceApp;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Veloce",
        options,
        Box::new(|cc| {
            fonts::setup_fonts(&cc.egui_ctx);
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(VeloceApp::new()))
        }),
    )
}

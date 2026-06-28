mod app;
mod markdown;
mod net;

use app::VeloceApp;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Veloce",
        options,
        Box::new(|_cc| Ok(Box::new(VeloceApp::new()))),
    )
}

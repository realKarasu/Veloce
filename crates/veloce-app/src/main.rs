mod app;
mod cdn;
mod embedded_emoji;
mod emoji;
mod fonts;
mod grouping;
mod imgcache;
mod markdown;
mod net;
mod plugins;
mod timestamp;

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
            cc.egui_ctx
                .add_bytes_loader(std::sync::Arc::new(crate::imgcache::DiskCacheLoader::new()));
            for (code, bytes) in crate::embedded_emoji::EMBEDDED {
                cc.egui_ctx
                    .include_bytes(format!("bytes://emoji/{code}.png"), *bytes);
            }
            Ok(Box::new(VeloceApp::new()))
        }),
    )
}

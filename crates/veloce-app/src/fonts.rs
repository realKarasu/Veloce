use eframe::egui::{FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

/// Construit la config de polices : 4 polices Noto embarquées + fallback.
pub fn build_font_definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        "noto_sans".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/NotoSans.ttf"
        ))),
    );
    fonts.font_data.insert(
        "noto_sans_mono".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/NotoSansMono.ttf"
        ))),
    );
    fonts.font_data.insert(
        "noto_sans_cjk".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/NotoSansCJK.otf"
        ))),
    );
    fonts.font_data.insert(
        "noto_sans_symbols2".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/NotoSansSymbols2.ttf"
        ))),
    );
    fonts.font_data.insert(
        "noto_sans_math".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/NotoSansMath.ttf"
        ))),
    );
    fonts.font_data.insert(
        "noto_emoji".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/NotoEmoji.ttf"
        ))),
    );

    // Fallback : texte -> CJK -> symboles -> lettres décoratives (math) -> emoji N&B.
    fonts.families.insert(
        FontFamily::Proportional,
        vec![
            "noto_sans".to_owned(),
            "noto_sans_cjk".to_owned(),
            "noto_sans_symbols2".to_owned(),
            "noto_sans_math".to_owned(),
            "noto_emoji".to_owned(),
        ],
    );
    fonts.families.insert(
        FontFamily::Monospace,
        vec![
            "noto_sans_mono".to_owned(),
            "noto_sans_cjk".to_owned(),
            "noto_sans_symbols2".to_owned(),
            "noto_sans_math".to_owned(),
            "noto_emoji".to_owned(),
        ],
    );

    fonts
}

/// Applique les polices au contexte egui (à appeler une fois au démarrage).
pub fn setup_fonts(ctx: &eframe::egui::Context) {
    ctx.set_fonts(build_font_definitions());
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::FontFamily;

    #[test]
    fn definitions_contiennent_les_4_polices() {
        let f = build_font_definitions();
        for key in [
            "noto_sans",
            "noto_sans_mono",
            "noto_sans_cjk",
            "noto_sans_symbols2",
            "noto_sans_math",
            "noto_emoji",
        ] {
            assert!(f.font_data.contains_key(key), "police manquante : {key}");
        }
    }

    #[test]
    fn familles_avec_fallback_dans_l_ordre() {
        let f = build_font_definitions();
        let prop = &f.families[&FontFamily::Proportional];
        assert_eq!(prop[0], "noto_sans");
        assert!(prop.contains(&"noto_sans_cjk".to_owned()));
        assert!(prop.contains(&"noto_emoji".to_owned()));
        let mono = &f.families[&FontFamily::Monospace];
        assert_eq!(mono[0], "noto_sans_mono");
        assert!(mono.contains(&"noto_sans_cjk".to_owned()));
    }
}

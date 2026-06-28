# Veloce — Plan d'implémentation : couverture de polices (SP1)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Charger dans egui un jeu de polices Noto à large couverture (latin étendu, cyrillique, grec, symboles, CJK, emojis Unicode N&B) avec fallback, pour éliminer les carrés ▯.

**Architecture:** Un module `fonts.rs` dans `veloce-app` construit une `egui::FontDefinitions` à partir de 4 polices Noto **déjà embarquées** dans `crates/veloce-app/assets/fonts/` (`include_bytes!`), avec des chaînes de fallback, et l'applique au `Context` au démarrage depuis la closure de création eframe.

**Tech Stack:** Rust 2021, eframe/egui 0.30.

## Global Constraints

- Les 4 polices sont **déjà présentes et versionnées** (commit `0d48b40`) :
  `crates/veloce-app/assets/fonts/{NotoSans.ttf, NotoSansMono.ttf, NotoSansCJK.otf, NotoEmoji.ttf}` (+ `OFL.txt`, `SOURCES.md`). Ne PAS les retélécharger.
- Chaîne de fallback : Proportional → `[noto_sans, noto_sans_cjk, noto_emoji]` ; Monospace → `[noto_sans_mono, noto_sans_cjk, noto_emoji]`.
- Clés `font_data` exactes : `noto_sans`, `noto_sans_mono`, `noto_sans_cjk`, `noto_emoji`.
- Emojis en **monochrome** (egui ne rend pas la couleur) ; emojis Discord custom `<:nom:id>` = hors SP1 (texte brut, traités en SP2). `veloce-discord` non touché.
- Édition 2021. Messages de commit en français, style `type: description`.

---

### Task 1 : Module `fonts.rs` + câblage `main.rs`

**Files:**
- Create: `crates/veloce-app/src/fonts.rs`
- Modify: `crates/veloce-app/src/main.rs`

**Interfaces:**
- Consumes: `eframe::egui` ; les 4 fichiers de police embarqués.
- Produces: `pub fn build_font_definitions() -> eframe::egui::FontDefinitions` ; `pub fn setup_fonts(ctx: &eframe::egui::Context)`.

- [ ] **Step 1 : Écrire les tests (échouent)** — créer `crates/veloce-app/src/fonts.rs` avec uniquement le module de tests d'abord :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::FontFamily;

    #[test]
    fn definitions_contiennent_les_4_polices() {
        let f = build_font_definitions();
        for key in ["noto_sans", "noto_sans_mono", "noto_sans_cjk", "noto_emoji"] {
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
```

- [ ] **Step 2 : Vérifier l'échec**

Run: `cargo test -p veloce-app fonts`
Expected: FAIL (`build_font_definitions` introuvable).

- [ ] **Step 3 : Implémenter `fonts.rs`** (au-dessus du `mod tests`)

```rust
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
        "noto_emoji".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/NotoEmoji.ttf"
        ))),
    );

    fonts.families.insert(
        FontFamily::Proportional,
        vec![
            "noto_sans".to_owned(),
            "noto_sans_cjk".to_owned(),
            "noto_emoji".to_owned(),
        ],
    );
    fonts.families.insert(
        FontFamily::Monospace,
        vec![
            "noto_sans_mono".to_owned(),
            "noto_sans_cjk".to_owned(),
            "noto_emoji".to_owned(),
        ],
    );

    fonts
}

/// Applique les polices au contexte egui (à appeler une fois au démarrage).
pub fn setup_fonts(ctx: &eframe::egui::Context) {
    ctx.set_fonts(build_font_definitions());
}
```

> **Note API egui 0.30 :** la valeur de `FontDefinitions.font_data` est `Arc<FontData>` dans les versions récentes d'egui ; d'où `Arc::new(FontData::from_static(...))`. Si le compilateur indique que `font_data` attend `FontData` (et non `Arc<FontData>`), retirer le `Arc::new(...)`. De même, `ctx.set_fonts(...)` doit exister en 0.30 ; faire compiler proprement sans changer le comportement (4 polices + fallback).

- [ ] **Step 4 : Câbler `main.rs`** — déclarer le module et appeler `setup_fonts` dans la closure de création :

```rust
mod app;
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
            Ok(Box::new(VeloceApp::new()))
        }),
    )
}
```

- [ ] **Step 5 : Vérifier le succès + gates**

Run:
```bash
cargo test -p veloce-app fonts
cargo test --all
cargo build
cargo clippy --all-targets -- -D warnings
cargo fmt --all
```
Expected: les 2 tests `fonts` PASS, suite complète PASS, build OK, clippy 0 warning, fmt propre.

- [ ] **Step 6 : Vérification manuelle (utilisateur)**

Run: `cargo run --bin veloce`
Expected : les messages avec accents/cyrillique/symboles, du texte CJK, et des emojis Unicode (en N&B) s'affichent **sans carré ▯**.

- [ ] **Step 7 : Commit**

```bash
git add -A
git commit -m "feat(app): charge les polices Noto dans egui (caractères spéciaux, CJK, emoji N&B)"
```

---

## Self-Review (effectuée)

**1. Couverture de la spec :**
- `build_font_definitions()` + `setup_fonts()` → Task 1. ✅
- 4 polices embarquées via include_bytes (clés exactes) → Task 1 Step 3. ✅
- Chaînes de fallback Proportional/Monospace → Task 1 Step 3 + testées Step 1. ✅
- Câblage main.rs (closure eframe) → Task 1 Step 4. ✅
- Tests de `build_font_definitions` (4 clés + ordre fallback) → Step 1. ✅
- Rendu réel = critère manuel → Step 6. ✅
- `veloce-discord` non touché ; emojis couleur/custom hors SP1 → respecté. ✅

**2. Placeholders :** aucun. La note « API egui 0.30 » est une instruction d'ajustement mécanique (Arc<FontData> vs FontData), pas un placeholder.

**3. Cohérence des types :** clés `font_data` identiques entre Step 1 (tests) et Step 3 (impl) ; `build_font_definitions`/`setup_fonts` signatures cohérentes ; chemins `include_bytes!("../assets/fonts/...")` corrects depuis `src/fonts.rs`.

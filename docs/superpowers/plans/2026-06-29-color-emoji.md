# Veloce — Plan d'implémentation : emojis couleur (partout)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Afficher les emojis en images couleur (Unicode via twemoji, custom Discord via CDN) dans les messages ET les noms de salons/serveurs.

**Architecture:** Un tokeniseur pur découpe les chaînes en segments texte/emoji et mappe chaque emoji vers une URL d'image ; un composant egui rend ces segments inline (texte stylé + `egui::Image`), `egui_extras` chargeant et mettant en cache les textures. Appliqué aux messages et, via des lignes cliquables custom, à la sidebar.

**Tech Stack:** Rust 2021, eframe/egui 0.30, egui_extras (loaders image+http), unicode-segmentation, unic-emoji-char.

## Global Constraints

- Tout dans `veloce-app` ; `veloce-discord` non touché.
- URLs (vérifiées) : Unicode → `https://cdn.jsdelivr.net/gh/jdecked/twemoji@15.1.0/assets/72x72/{code}.png` ; custom → `https://cdn.discordapp.com/emojis/{id}.png?size=44`.
- `twemoji_code` : codepoints en **hex minuscule** joints par `-`, **FE0F retiré sauf si ZWJ (U+200D) présent** dans le graphème.
- Détection emoji Unicode : un graphème est emoji s'il contient **U+FE0F** OU si son 1er caractère a **Emoji_Presentation** (`unic_emoji_char::is_emoji_presentation`). Exclut `3`/`#`/`*` nus.
- Custom Discord : motifs `<:nom:id>` et `<a:nom:id>` (id = chiffres) → image (animés en statique `.png`).
- Emojis rendus à la **hauteur de ligne** (~20 px). GIF animés = statiques.
- Deps (planchers / matcher egui 0.30) : `egui_extras = "0.30"` (features chargeurs image+http, décodage PNG), `unicode-segmentation = "1"`, `unic-emoji-char = "0.9"`.
- Édition 2021. Commits en français, style `type: description`.

---

## Structure des fichiers

```
crates/veloce-app/
├─ Cargo.toml        # Task 1 : egui_extras, unicode-segmentation, unic-emoji-char
└─ src/
   ├─ main.rs        # Task 1 : install_image_loaders
   ├─ emoji.rs       # Task 2 : split_emojis + URLs (pur)
   └─ app.rs         # Task 3 : render_rich messages ; Task 4 : lignes cliquables sidebar
```

---

### Task 1 : Dépendances + chargeurs d'images

**Files:**
- Modify: `crates/veloce-app/Cargo.toml`, `crates/veloce-app/src/main.rs`

**Interfaces:**
- Produces: chargeurs d'images egui installés ; deps disponibles (`egui_extras`, `unicode_segmentation`, `unic_emoji_char`).

**Note :** pas de test unitaire (setup) ; gate = build + clippy + fmt.

- [ ] **Step 1 : Ajouter les dépendances** — dans `crates/veloce-app/Cargo.toml`, section `[dependencies]` :

```toml
egui_extras = { version = "0.30", features = ["all_loaders"] }
image = { version = "0.25", default-features = false, features = ["png"] }
unicode-segmentation = "1"
unic-emoji-char = "0.9"
```

*(`all_loaders` active les chargeurs image + http d'egui_extras ; `image` avec la feature `png` garantit le décodage des PNG twemoji/Discord. Si `all_loaders` tire déjà png, l'ajout reste inoffensif. Épingler `egui_extras` sur la version qui matche `egui`/`eframe` 0.30.)*

- [ ] **Step 2 : Installer les chargeurs** — dans `crates/veloce-app/src/main.rs`, la closure de création eframe, après `fonts::setup_fonts` :

```rust
        Box::new(|cc| {
            fonts::setup_fonts(&cc.egui_ctx);
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(VeloceApp::new()))
        }),
```

Et déclarer le module emoji (créé en Task 2) en avance n'est pas nécessaire ici ; ajouter `mod emoji;` se fait en Task 2.

- [ ] **Step 3 : Vérifier**

Run: `cargo build && cargo clippy --all-targets -- -D warnings && cargo fmt --all`
Expected: compile (egui_extras résout en 0.30), clippy clean, fmt clean.

- [ ] **Step 4 : Commit**

```bash
git add -A
git commit -m "chore(app): egui_extras (chargeurs image/http) + crates emoji + install_image_loaders"
```

---

### Task 2 : Tokeniseur d'emojis + URLs (pur)

**Files:**
- Create: `crates/veloce-app/src/emoji.rs`
- Modify: `crates/veloce-app/src/main.rs` (`mod emoji;`)

**Interfaces:**
- Consumes: `unicode_segmentation::UnicodeSegmentation`, `unic_emoji_char`.
- Produces:
  - `enum EmojiSeg { Text(String), Emoji { url: String } }` (derive Debug, Clone, PartialEq).
  - `pub fn custom_emoji_url(id: &str) -> String`
  - `pub fn twemoji_code(grapheme: &str) -> String`
  - `pub fn twemoji_url(grapheme: &str) -> String`
  - `pub fn split_emojis(input: &str) -> Vec<EmojiSeg>`

- [ ] **Step 1 : Écrire les tests (échouent)** — `emoji.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_url() {
        assert_eq!(
            custom_emoji_url("123"),
            "https://cdn.discordapp.com/emojis/123.png?size=44"
        );
    }

    #[test]
    fn twemoji_code_simple_et_fe0f_et_zwj() {
        assert_eq!(twemoji_code("\u{1F600}"), "1f600"); // 😀
        assert_eq!(twemoji_code("\u{2764}\u{FE0F}"), "2764"); // ❤️ : FE0F retiré (pas de ZWJ)
        // 👨‍👩‍👧 : ZWJ présent → tous les codepoints gardés
        assert_eq!(
            twemoji_code("\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}"),
            "1f468-200d-1f469-200d-1f467"
        );
        // keycap 5️⃣ : 0035 FE0F 20E3, pas de ZWJ → FE0F retiré
        assert_eq!(twemoji_code("5\u{FE0F}\u{20E3}"), "35-20e3");
    }

    #[test]
    fn split_custom_simple_et_anime() {
        assert_eq!(
            split_emojis("<:smile:123>"),
            vec![EmojiSeg::Emoji { url: custom_emoji_url("123") }]
        );
        assert_eq!(
            split_emojis("<a:wave:456>"),
            vec![EmojiSeg::Emoji { url: custom_emoji_url("456") }]
        );
    }

    #[test]
    fn split_unicode_mixte() {
        let segs = split_emojis("a\u{1F600}b");
        assert_eq!(
            segs,
            vec![
                EmojiSeg::Text("a".into()),
                EmojiSeg::Emoji { url: twemoji_url("\u{1F600}") },
                EmojiSeg::Text("b".into()),
            ]
        );
    }

    #[test]
    fn chiffre_nu_n_est_pas_emoji() {
        assert_eq!(split_emojis("5"), vec![EmojiSeg::Text("5".into())]);
    }

    #[test]
    fn keycap_est_emoji() {
        let segs = split_emojis("5\u{FE0F}\u{20E3}");
        assert_eq!(segs, vec![EmojiSeg::Emoji { url: twemoji_url("5\u{FE0F}\u{20E3}") }]);
    }

    #[test]
    fn texte_consecutif_fusionne() {
        assert_eq!(split_emojis("hello"), vec![EmojiSeg::Text("hello".into())]);
    }
}
```

- [ ] **Step 2 : Vérifier l'échec**

Run: `cargo test -p veloce-app emoji`
Expected: FAIL (module/fonctions inexistants — déclarer `mod emoji;` d'abord, cf. Step 4, sinon erreur de module).

*(Pour que le test compile : ajouter `mod emoji;` à `main.rs` avant de lancer — voir Step 4 ; le RED est alors « fonctions introuvables ».)*

- [ ] **Step 3 : Implémenter `emoji.rs`** (au-dessus du `mod tests`)

```rust
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone, PartialEq)]
pub enum EmojiSeg {
    Text(String),
    Emoji { url: String },
}

const TWEMOJI_BASE: &str = "https://cdn.jsdelivr.net/gh/jdecked/twemoji@15.1.0/assets/72x72";

pub fn custom_emoji_url(id: &str) -> String {
    format!("https://cdn.discordapp.com/emojis/{id}.png?size=44")
}

/// Code twemoji : codepoints hex minuscule joints par '-', FE0F retiré sauf si ZWJ présent.
pub fn twemoji_code(grapheme: &str) -> String {
    let has_zwj = grapheme.contains('\u{200D}');
    grapheme
        .chars()
        .filter(|&c| has_zwj || c != '\u{FE0F}')
        .map(|c| format!("{:x}", c as u32))
        .collect::<Vec<_>>()
        .join("-")
}

pub fn twemoji_url(grapheme: &str) -> String {
    format!("{TWEMOJI_BASE}/{}.png", twemoji_code(grapheme))
}

/// Un graphème est un emoji Unicode s'il contient VS16 (U+FE0F) ou si son 1er
/// caractère a la propriété Emoji_Presentation.
fn is_emoji_grapheme(g: &str) -> bool {
    g.contains('\u{FE0F}')
        || g.chars()
            .next()
            .map(unic_emoji_char::is_emoji_presentation)
            .unwrap_or(false)
}

/// Essaie de parser un emoji custom `<:nom:id>` / `<a:nom:id>` au début de `s`.
/// Renvoie (octets consommés, id) si match.
fn parse_custom(s: &str) -> Option<(usize, String)> {
    let b = s.as_bytes();
    if b.first() != Some(&b'<') {
        return None;
    }
    let mut i = 1;
    if b.get(i) == Some(&b'a') {
        i += 1; // animé
    }
    if b.get(i) != Some(&b':') {
        return None;
    }
    i += 1;
    let name_start = i;
    while i < b.len() && b[i] != b':' && b[i] != b'>' {
        i += 1;
    }
    if i == name_start || b.get(i) != Some(&b':') {
        return None;
    }
    i += 1;
    let id_start = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if i == id_start || b.get(i) != Some(&b'>') {
        return None;
    }
    Some((i + 1, s[id_start..i].to_string()))
}

/// Découpe une chaîne en segments texte/emoji (custom + Unicode).
pub fn split_emojis(input: &str) -> Vec<EmojiSeg> {
    let mut segs = Vec::new();
    let mut buf = String::new();
    let mut rest = input;
    while !rest.is_empty() {
        if let Some((consumed, id)) = parse_custom(rest) {
            if !buf.is_empty() {
                segs.push(EmojiSeg::Text(std::mem::take(&mut buf)));
            }
            segs.push(EmojiSeg::Emoji { url: custom_emoji_url(&id) });
            rest = &rest[consumed..];
            continue;
        }
        let g = rest.graphemes(true).next().unwrap();
        if is_emoji_grapheme(g) {
            if !buf.is_empty() {
                segs.push(EmojiSeg::Text(std::mem::take(&mut buf)));
            }
            segs.push(EmojiSeg::Emoji { url: twemoji_url(g) });
        } else {
            buf.push_str(g);
        }
        rest = &rest[g.len()..];
    }
    if !buf.is_empty() {
        segs.push(EmojiSeg::Text(buf));
    }
    segs
}
```

- [ ] **Step 4 : Déclarer le module** — dans `main.rs`, ajouter `mod emoji;`.

- [ ] **Step 5 : Vérifier le succès**

Run: `cargo test -p veloce-app emoji`
Expected: PASS (7 tests). `cargo build -p veloce-app` compile (warnings dead_code tolérés tant que `split_emojis` n'est pas consommé — il l'est en Tasks 3/4 ; ne pas lancer `clippy -D warnings` ici).

- [ ] **Step 6 : Commit**

```bash
git add -A
git commit -m "feat(app): tokeniseur d'emojis pur (twemoji + custom Discord) + tests"
```

---

### Task 3 : Rendu inline couleur dans les messages

**Files:**
- Modify: `crates/veloce-app/src/app.rs`

**Interfaces:**
- Consumes: `crate::emoji::{split_emojis, EmojiSeg}`, `crate::markdown::{parse_markdown, Span}`, `egui`, `egui_extras` (loaders déjà installés).
- Produces: `fn render_message(ui: &mut egui::Ui, content: &str, plugins: &mut PluginManager)` ; helper `fn span_rich(text: &str, span: &Span) -> egui::RichText`.

**Note :** rendu egui → vérif par build + manuel ; le tokeniseur est déjà testé.

- [ ] **Step 1 : Implémenter le rendu** — dans `app.rs`, ajouter :

```rust
use crate::emoji::{split_emojis, EmojiSeg};

const EMOJI_SIZE: f32 = 20.0;

/// RichText stylé selon un span markdown.
fn span_rich(text: &str, span: &Span) -> RichText {
    let mut rt = RichText::new(text).size(14.0);
    if span.code {
        rt = rt.monospace().background_color(Color32::from_gray(40));
    }
    if span.bold {
        rt = rt.strong().color(Color32::WHITE);
    }
    if span.italic {
        rt = rt.italics();
    }
    if span.strike {
        rt = rt.strikethrough();
    }
    rt
}

/// Rend un message : markdown (via plugins) + emojis couleur inline.
fn render_message(ui: &mut egui::Ui, content: &str, plugins: &mut PluginManager) {
    let mut c = content.to_string();
    plugins.apply_render(&mut c);
    let spans = parse_markdown(&c);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for span in &spans {
            for seg in split_emojis(&span.text) {
                match seg {
                    EmojiSeg::Text(t) => {
                        ui.label(span_rich(&t, span));
                    }
                    EmojiSeg::Emoji { url } => {
                        ui.add(
                            egui::Image::new(url)
                                .fit_to_exact_size(egui::vec2(EMOJI_SIZE, EMOJI_SIZE)),
                        );
                    }
                }
            }
        }
    });
}
```

- [ ] **Step 2 : Brancher dans le rendu des messages** — dans `draw_chat`, le `CentralPanel` qui liste les messages, remplacer le `ui.label(spans_to_job(...))` (et le `parse_markdown`/`apply_render` inline associé) par un appel à `render_message`. Le bloc devient :

```rust
                for m in &state.messages {
                    ui.horizontal_wrapped(|ui| {
                        let name = m
                            .author
                            .global_name
                            .clone()
                            .unwrap_or_else(|| m.author.username.clone());
                        ui.label(
                            RichText::new(format!("{name}: "))
                                .strong()
                                .color(Color32::LIGHT_BLUE),
                        );
                        render_message(ui, &m.content, plugins);
                    });
                }
```

*(Note : `render_message` prend `plugins` ; `draw_chat` reçoit déjà `plugins: &mut PluginManager`. Si `spans_to_job` devient inutilisé après ce changement, le retirer ; s'il reste utilisé ailleurs, le garder. Ajuster l'API egui `Image`/`fit_to_exact_size`/`RichText` si elle diffère en 0.30, en préservant le comportement.)*

- [ ] **Step 3 : Vérifier**

Run: `cargo build -p veloce-app && cargo fmt --all`
Expected: compile (clippy -D warnings possible si plus de dead_code ; sinon Task 4 finalise la sidebar — lancer au moins `cargo build`).

- [ ] **Step 4 : Commit**

```bash
git add -A
git commit -m "feat(app): emojis couleur inline dans les messages"
```

---

### Task 4 : Emojis couleur dans la sidebar (salons + serveurs) + clippy-clean

**Files:**
- Modify: `crates/veloce-app/src/app.rs`

**Interfaces:**
- Consumes: `render` helpers + `split_emojis`.
- Produces: `fn rich_label_row(ui, icon: Option<&str>, name: &str, selected: bool, enabled: bool) -> egui::Response` (ligne cliquable avec nom à emojis couleur).

**Note :** dernière tâche → tout le workspace **clippy-clean**. Code egui custom (ligne cliquable) → ajuster pour compiler proprement en 0.30 en préservant le comportement de sélection.

- [ ] **Step 1 : Helper de ligne cliquable** — dans `app.rs` :

```rust
/// Une ligne cliquable de la sidebar : icône (glyphe) optionnelle + nom avec
/// emojis couleur. Fond de sélection ; tout le rang est cliquable.
fn rich_label_row(
    ui: &mut egui::Ui,
    icon: Option<&str>,
    name: &str,
    selected: bool,
    enabled: bool,
) -> egui::Response {
    let inner = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        if let Some(ic) = icon {
            ui.label(ic);
        }
        for seg in split_emojis(name) {
            match seg {
                EmojiSeg::Text(t) => {
                    ui.label(RichText::new(t).size(14.0));
                }
                EmojiSeg::Emoji { url } => {
                    ui.add(egui::Image::new(url).fit_to_exact_size(egui::vec2(18.0, 18.0)));
                }
            }
        }
    });
    let resp = inner.response.interact(egui::Sense::click());
    if selected || resp.hovered() {
        let bg = if selected {
            ui.visuals().selection.bg_fill
        } else {
            ui.visuals().widgets.hovered.bg_fill
        };
        ui.painter()
            .rect_filled(resp.rect, 4.0, bg.gamma_multiply(0.4));
    }
    if !enabled {
        // visuel désactivé léger : rien de cliquable
    }
    resp
}
```

*(Le fond peint après coup peut passer derrière le texte selon l'ordre ; si le rendu est insatisfaisant, peindre le fond AVANT le contenu en réservant le rect — ajuster pour un visuel correct, comportement de clic préservé. L'objectif : ligne entière cliquable + sélection visible + emojis du nom en couleur.)*

- [ ] **Step 2 : Utiliser dans le panneau serveurs** — remplacer la boucle des guildes par :

```rust
                for g in state.guilds.clone() {
                    let selected = state.selected_guild.as_ref() == Some(&g.id);
                    if rich_label_row(ui, None, &g.name, selected, true).clicked() {
                        state.selected_guild = Some(g.id.clone());
                        state.channels.clear();
                        state.channel_tree.clear();
                        net.subscribe_guild(g.id.clone());
                        net.send(Command::SelectGuild(g.id));
                    }
                }
```

- [ ] **Step 3 : Utiliser dans le panneau salons** — dans le rendu de l'arbre, remplacer le `TreeRow::Channel` actuel (SelectableLabel) par `rich_label_row` avec l'icône de type :

```rust
                        TreeRow::Channel(c) => {
                            let selectable = matches!(c.kind, 0 | 5 | 15);
                            let selected = state.selected_channel.as_ref() == Some(&c.id);
                            let name = c.name.clone().unwrap_or_else(|| c.id.clone());
                            let resp =
                                rich_label_row(ui, Some(channel_icon(c.kind)), &name, selected, selectable);
                            if selectable && resp.clicked() {
                                state.selected_channel = Some(c.id.clone());
                                state.messages.clear();
                                net.send(Command::FetchHistory(c.id.clone()));
                            }
                        }
```

(Les `TreeRow::Category` restent rendus comme aujourd'hui — en-tête texte ; un nom de catégorie à emoji peut aussi passer par `split_emojis` si souhaité, optionnel.)

- [ ] **Step 4 : Gates complets**

Run:
```bash
cargo test --all
cargo build
cargo clippy --all-targets -- -D warnings
cargo fmt --all
```
Expected: tests PASS (emoji + existants), build OK, **clippy 0 warning** sur tout le workspace, fmt propre.

- [ ] **Step 5 : Vérification manuelle (utilisateur)**

Run: `cargo run --bin veloce`
Expected : emojis Unicode **en couleur** dans les messages ; emojis custom `<:nom:id>` **en couleur** (plus de texte brut) ; emojis en couleur dans les noms de salons et serveurs ; salons toujours cliquables, icône de type conservée.

- [ ] **Step 6 : Commit**

```bash
git add -A
git commit -m "feat(app): emojis couleur dans la sidebar (salons + serveurs), lignes cliquables"
```

---

## Self-Review (effectuée)

**1. Couverture de la spec :**
- Chargeurs d'images egui + deps → Task 1. ✅
- Tokeniseur pur (custom + Unicode, FE0F/ZWJ, détection Emoji_Presentation/VS16) → Task 2 + tests. ✅
- URLs twemoji (pinné) + custom CDN → Task 2. ✅
- Rendu inline messages (markdown + emoji images) → Task 3. ✅
- Sidebar salons + serveurs en lignes cliquables avec emojis couleur, icône de type conservée → Task 4. ✅
- Tests tokeniseur (custom/animé/unicode/VS16/keycap/chiffre-nu/ZWJ/fusion) → Task 2. ✅
- GIF statiques, 1 fetch/emoji, risques → assumés (spec §10). ✅

**2. Placeholders :** aucun « TBD ». Les notes (ajustement API egui 0.30, ordre de peinture du fond de sélection) sont des précisions d'intégration egui, pas des placeholders de code.

**3. Cohérence des types :** `EmojiSeg`/`split_emojis`/`twemoji_url`/`custom_emoji_url` identiques Tasks 2/3/4 ; `render_message`/`rich_label_row` signatures cohérentes ; `channel_icon` (Task salons fidèles) réutilisé ; `Span` de `markdown` réutilisé par `span_rich`.
```

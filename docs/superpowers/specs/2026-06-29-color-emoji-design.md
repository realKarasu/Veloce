# Veloce — Spec de conception : emojis couleur (partout)

**Date :** 2026-06-29
**Statut :** Validé (design approuvé)
**Périmètre :** rendre les emojis en **images couleur** partout (messages, noms de salons, noms de serveurs) — Unicode (twemoji) et custom Discord — via egui_extras. SP2 du chantier « emojis & caractères spéciaux » (SP1 = polices, déjà livré).

---

## 1. Problème

egui rend les glyphes en **monochrome** (atlas N&B) → les emojis Unicode
sortent en noir et blanc (police Noto Emoji de SP1) et les emojis custom
`<:nom:id>` restent du **texte brut**. Pour de vrais emojis couleur, il faut les
rendre en **petites images** intercalées avec le texte.

## 2. Solution

Tokeniser les chaînes en segments texte/emoji, mapper chaque emoji vers une URL
d'image, et rendre inline via egui (`egui_extras` charge et met en cache les
textures). Appliqué **partout** où un emoji apparaît : messages, noms de salons,
noms de serveurs.

### Inclus
- Chargeur d'images HTTP egui (`egui_extras::install_image_loaders`).
- Tokeniseur pur (custom + Unicode) + mapping d'URL pur.
- Composant de rendu inline réutilisable (texte stylé + images emoji).
- Application : messages, sidebar salons, sidebar serveurs.

### Hors périmètre
- Animation des GIF (emojis animés rendus en **statique**).
- Réactions emoji, sélecteur d'emoji, autocomplétion `:nom:`.

## 3. Dépendances (veloce-app)
- `egui_extras` avec chargeurs **image + http** (`install_image_loaders`).
- `unicode-segmentation` (graphèmes), `unic-emoji-char` (propriétés emoji).

(Versions = planchers, épingler la dernière `0.x`/compatible ; egui_extras doit
matcher la version d'egui/eframe, 0.30.)

## 4. Tokeniseur — pur (`veloce-app/src/emoji.rs`)

`split_emojis(input: &str) -> Vec<EmojiSeg>` avec
`enum EmojiSeg { Text(String), Emoji { url: String } }`.

1. **Custom Discord** d'abord : motifs `<:nom:id>` et `<a:nom:id>` →
   `Emoji { url: https://cdn.discordapp.com/emojis/{id}.png?size=44 }` (animés
   rendus en statique → `.png`).
2. **Unicode** sur le texte restant : découpe en **graphèmes**
   (`unicode-segmentation`). Un graphème est un emoji si son 1er caractère a la
   propriété **Emoji_Presentation** *ou* si le graphème contient **U+FE0F**
   (VS16). (Exclut `3`/`#`/`*` nus ; inclut séquences ZWJ, drapeaux, teintes.)
3. **URL twemoji** : pour un graphème emoji, retirer `U+FE0F` **sauf** si le
   graphème contient un **ZWJ** (U+200D) (règle officielle twemoji) ; encoder
   chaque codepoint restant en hex minuscule, joints par `-` ; URL =
   `https://cdn.jsdelivr.net/gh/jdecked/twemoji@latest/assets/72x72/{code}.png`.
4. Les morceaux de texte consécutifs sont fusionnés en un seul `Text`.

Fonctions pures dédiées et testées : `twemoji_code(grapheme: &str) -> String`,
`custom_emoji_url(id: &str) -> String`, `twemoji_url(grapheme: &str) -> String`.

## 5. Rendu riche inline — egui (`veloce-app`)

`enum RichSeg { Text { text: String, style: SpanStyle }, Emoji { url: String } }`
(`SpanStyle` reprend bold/italic/strike/code de `markdown::Span`).

`render_rich(ui: &mut egui::Ui, segs: &[RichSeg])` : un `horizontal_wrapped`
où chaque `Text` est un label stylé (réutilise la mise en forme de
`spans_to_job`) et chaque `Emoji` une `egui::Image::new(url)` dimensionnée à la
hauteur de ligne (~20 px), alignée au texte.

- **Messages** : `apply_render` (plugins) → `parse_markdown` → `Vec<Span>` ;
  pour chaque span, `split_emojis(span.text)` → `RichSeg` portant le style du
  span ; rendu via `render_rich`.
- **Noms (salons/serveurs)** : `split_emojis(nom)` → `RichSeg` style neutre.

## 6. Lignes cliquables de la sidebar

`SelectableLabel` ne prend pas d'images → on rend chaque ligne salon/serveur
comme une **ligne cliquable custom** : un `horizontal` (icône de type éventuelle
+ `render_rich(nom)`), un fond de sélection/survol, et `Sense::click()` sur le
rectangle de la ligne pour conserver le comportement de sélection actuel. Les
icônes de **type** de salon (`#`, 🔊, 📢, …) restent des glyphes ; seuls les
emojis du **nom** deviennent des images.

## 7. Câblage

- `main.rs` : `egui_extras::install_image_loaders(&cc.egui_ctx)` après
  `fonts::setup_fonts`.
- `app.rs` : messages via `render_rich` ; sidebar salons et serveurs via lignes
  cliquables `render_rich`.

## 8. Tests

Tokeniseur/URL (purs) :
- custom `<:a:123>` → URL CDN ; custom animé `<a:b:456>` → URL CDN (.png).
- unicode simple `😀` → 1 emoji ; `a😀b` → text/emoji/text.
- `5️⃣` (chiffre + VS16 + keycap) → emoji ; **`5` nu → texte** (pas emoji).
- séquence ZWJ `👨‍👩‍👧` → 1 emoji, FE0F gardé si ZWJ présent.
- `twemoji_code` : strip FE0F hors-ZWJ, conservé si ZWJ ; hex minuscule joint `-`.
- fusion des `Text` consécutifs.

Le rendu egui (images inline, lignes cliquables) = **vérification manuelle**.

## 9. Critères de succès

1. `split_emojis`/`twemoji_code`/URLs couverts par tests (verts).
2. (Manuel) Les emojis Unicode des messages s'affichent **en couleur**.
3. (Manuel) Les emojis custom `<:nom:id>` s'affichent **en couleur** (plus de
   texte brut).
4. (Manuel) Les emojis dans les noms de salons et serveurs s'affichent en
   couleur ; les salons restent **cliquables** et l'icône de type est conservée.
5. Aucune régression : markdown, sélection salon, envoi ; build/clippy/fmt
   propres.

## 10. Risques (assumés — choix « partout »)

| Risque | Mitigation |
|---|---|
| Mise en page inline imparfaite (alignement, retour ligne) | Taille image = hauteur de ligne ; itérer si besoin |
| GIF animés | Rendus statiques (1ère frame) ; documenté |
| 1 requête réseau par emoji unique | Cache texture egui en session ; au repos, aucun coût |
| Lignes cliquables custom (sidebar) | Isolées dans un helper ; comportement de sélection préservé |
| Faux positifs détection emoji | Règle Emoji_Presentation/VS16 ; détection isolée et testée, ajustable |
| `egui_extras` désynchronisé d'egui | Épingler la version qui matche egui/eframe 0.30 |

# Veloce — Spec de conception : couverture de polices (SP1)

**Date :** 2026-06-28
**Statut :** Validé (design approuvé)
**Périmètre :** charger des polices larges dans egui (caractères spéciaux, CJK, emojis Unicode N&B). Sous-projet 1 de « emojis & caractères spéciaux » ; SP2 (emojis couleur en images) suivra séparément.

---

## 1. Problème & cause racine

`main.rs` ne configure **aucune** police → egui utilise sa police par défaut
(latin de base). Tout ce qui en sort (emojis, accents étendus, cyrillique, grec,
symboles, CJK) s'affiche en **carrés ▯**. Cause racine confirmée : aucune
`set_fonts` n'est appelée.

## 2. Solution

Charger, au démarrage, un jeu de polices à large couverture dans egui, avec une
**chaîne de fallback** : egui pioche chaque glyphe dans la première police qui le
possède, donc un message mêlant latin + 日本語 + 😀 s'affiche entièrement.

### Inclus
- Module `fonts.rs` : `build_font_definitions()` (config) + `setup_fonts(ctx)`.
- Embarquement (via `include_bytes!`) de 4 polices OFL sous
  `crates/veloce-app/assets/fonts/`.
- Câblage dans `main.rs` (closure de création eframe → `setup_fonts`).

### Hors périmètre (→ SP2)
- Emojis Unicode en **couleur** (images / twemoji).
- Emojis Discord custom `<:nom:id>` / `<a:nom:id>` (rendus en images en SP2 ;
  restent du texte brut en SP1).

## 3. Architecture

```
crates/veloce-app/
├─ assets/fonts/         # 4 .ttf/.otf OFL (NotoSans, NotoSansMono, NotoSansCJK, NotoEmoji)
└─ src/
   ├─ fonts.rs           # build_font_definitions() + setup_fonts(ctx)
   └─ main.rs            # appelle setup_fonts dans la closure eframe
```

### Frontières
- **`fonts.rs`** — *Quoi :* construit la `FontDefinitions` (polices + familles +
  fallback) et l'applique au `egui::Context`. *Dépend de :* `eframe::egui`, les
  octets de polices embarqués. *Sans I/O réseau.*
- **`main.rs`** — appelle `fonts::setup_fonts(&cc.egui_ctx)` une fois, au
  démarrage, avant de construire `VeloceApp`.

## 4. Polices (OFL)

| Police | Rôle |
|---|---|
| Noto Sans (Regular) | latin étendu, cyrillique, grec, symboles |
| Noto Sans Mono (Regular) | blocs de code (`Monospace`) |
| Noto Sans CJK (Regular) | chinois / japonais / coréen |
| Noto Emoji (Regular, monochrome) | emojis Unicode en N&B |

Toutes sous licence **OFL** (compatible GPL-3.0). Fichiers versionnés dans le
repo (`assets/fonts/`).

## 5. Chaîne de fallback

```
Proportional → [NotoSans, NotoSansCJK, NotoEmoji]
Monospace    → [NotoSansMono, NotoSansCJK, NotoEmoji]
```

Les clés `font_data` portent des noms stables (`"noto_sans"`,
`"noto_sans_mono"`, `"noto_sans_cjk"`, `"noto_emoji"`).

## 6. Tests

`build_font_definitions()` étant pur (pas de `Context`), tester :
- `font_data` contient les 4 clés attendues.
- `families[&Proportional]` commence par `noto_sans` et inclut `noto_sans_cjk`
  puis `noto_emoji` (dans cet ordre).
- `families[&Monospace]` commence par `noto_sans_mono` et inclut les fallbacks.

Le rendu réel (plus de carrés) = **critère de vérification manuelle** (relancer
l'app sur des messages avec accents/CJK/emoji).

## 7. Critères de succès

1. `build_font_definitions()` renvoie les 4 polices + les familles avec fallback.
2. **(Manuel)** Un message avec accents/cyrillique/symboles s'affiche sans carré.
3. **(Manuel)** Un message en CJK s'affiche.
4. **(Manuel)** Un emoji Unicode s'affiche (en N&B).
5. Aucune régression de l'UI existante ; build/clippy/fmt propres.

## 8. Compromis & risques

| Sujet | Décision / mitigation |
|---|---|
| Taille binaire (Noto Sans CJK ~16 Mo) | Assumé (choix « TOUT ») ; embarquement pour la simplicité/offline. Téléchargement-au-1er-lancement = piste future si la taille gêne. |
| Police variable vs statique | Utiliser des **statiques** (Regular) — `ab_glyph`/egui les gère sans ambiguïté d'instance. |
| Emojis couleur attendus | Hors SP1 (limite egui : atlas monochrome) → SP2 (images). En SP1, emojis N&B. |
| Licence | OFL uniquement (compatible) ; fichiers + mention de licence dans le repo. |

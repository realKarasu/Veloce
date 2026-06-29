# Veloce — Spec de conception : rendu de messages fidèle à Discord

**Date :** 2026-06-29
**Statut :** Validé (design approuvé)
**Périmètre :** rendre les messages visuellement identiques à Discord — layout
(avatar + en-tête + regroupement), images/pièces jointes, contenu inline riche
(mentions salon/user/rôle, liens, timestamps, spoilers, blocs de code), embeds
riches (= aperçus de liens), réponses, et interactivité (zoom image, popup
profil, liens cliquables). Implémenté en **3 phases** derrière un seul modèle de
données.

---

## 1. Problème

Aujourd'hui un message est rendu en `nom: contenu` sur une ligne
(`draw_chat`, `render_message`), avec markdown basique (`markdown.rs`) et emojis
couleur (`emoji.rs`). Le modèle `Message` ne capture que
`id, channel_id, content, author, timestamp` : **les pièces jointes, embeds et
mentions arrivent dans le JSON Discord mais sont jetés au parsing**. Un client
fidèle affiche les avatars, regroupe les messages consécutifs, montre les images
et embeds, et transforme `<#id>`/`<@id>`/`<@&id>`/`<t:…>`/`||spoiler||` en rendu
lisible — pas en code brut.

## 2. Modèle de données (`veloce-discord/src/models.rs`)

Ajouts **rétro-compatibles** (serde ignore déjà les champs inconnus ; tous les
nouveaux champs en `#[serde(default)]`). Bénéficie à REST **et** gateway sans
changement de flux.

- `User` : ajouter `avatar: Option<String>`.
- `Role` : ajouter `name: String` (`#[serde(default)]`) et `color: u32`
  (`#[serde(default)]`, 0 = pas de couleur).
- `Message` : ajouter
  - `attachments: Vec<Attachment>`
  - `embeds: Vec<Embed>`
  - `mentions: Vec<User>` (objets user complets fournis par Discord)
  - `mention_roles: Vec<Snowflake>`
  - `referenced_message: Option<Box<Message>>` (réponse)
  - `edited_timestamp: Option<String>`
- Nouveau `Attachment { id, filename, content_type: Option<String>,
  url, proxy_url, size: u64, width: Option<u32>, height: Option<u32> }`.
- Nouveau `Embed { kind: Option<String> (#[serde(rename="type")]),
  title, description, url, color: Option<u32>,
  author: Option<EmbedAuthor>, fields: Vec<EmbedField>,
  image: Option<EmbedMedia>, thumbnail: Option<EmbedMedia>,
  footer: Option<EmbedFooter> }` + sous-structs
  `EmbedAuthor { name, icon_url }`, `EmbedField { name, value, inline: bool }`,
  `EmbedMedia { url, proxy_url, width, height }`,
  `EmbedFooter { text, icon_url }`. Tout optionnel/`default`.

Helper image : `Attachment::is_image()` = `content_type` commence par `image/`
**ou** extension du filename ∈ {png, jpg, jpeg, gif, webp, bmp}.

## 3. URLs CDN (`veloce-app`, à côté de `emoji.rs`)

Module `cdn.rs` (pur, testable) :

- `avatar_url(user_id, avatar_hash, size) -> String` :
  `https://cdn.discordapp.com/avatars/{id}/{hash}.{ext}?size={size}` où
  `ext = "gif"` si le hash commence par `a_`, sinon `"png"`.
- `default_avatar_url(user_id, discriminator) -> String` : index =
  `discriminator == "0"|None` → `(user_id >> 22) % 6`, sinon
  `discriminator % 5` ; URL `https://cdn.discordapp.com/embed/avatars/{index}.png`.
- `avatar_for(user) -> String` : `avatar_url` si hash présent, sinon défaut.

(Réutilise le loader URL `egui::Image::new(url)` déjà en place pour emojis.)

## 4. Phase 1 — Layout Discord + images (`veloce-app/src/app.rs`)

Refonte de la boucle de rendu des messages :

- **Regroupement** : `group_messages(&[Message]) -> Vec<MessageRow>` (pur,
  testable) où une ligne est soit un **en-tête** (avatar + `nom · heure` + corps)
  soit une **continuation** (corps seul, indenté), regroupée si même auteur et
  écart < 7 min avec le message précédent.
- **Ligne en-tête** : `horizontal` — gouttière avatar 40px
  (`Image::new(avatar_for(author)).fit_to_exact_size(40,40)` arrondi) + colonne
  (en-tête : nom gras + timestamp gris `small`, puis corps).
- **Continuation** : corps seul aligné sur la colonne (gouttière vide).
- **Images** : pour chaque `attachment.is_image()`, afficher l'image sous le
  corps, taille bornée (max ~400px large, ratio conservé via width/height).
  Clic → ouvre la **visionneuse** (cf. §7). Pièces jointes non-image → ligne
  cliquable « 📎 filename (taille) » ouvrant l'URL.
- **Timestamp** : `format_timestamp(iso) -> String` (pur) ; affichage court
  type `aujourd'hui à HH:MM` / date. Parsing ISO 8601 sans nouvelle dépendance
  lourde (extraction manuelle des champs ; pas de chrono si évitable).

## 5. Phase 2 — Tokenizer inline riche (`veloce-app/src/inline.rs`, nouveau)

Remplace l'enchaînement `parse_markdown` → `split_emojis`. Un seul analyseur :

`tokenize(content, ctx: &MentionCtx) -> Vec<Inline>` où
`Inline` = `Text{ text, style }` | `Emoji{ url }` |
`Mention{ kind: Channel|User|Role, label, color, target_id }` |
`Link{ url, text }` | `Spoiler{ children: Vec<Inline> }` | `CodeInline{ text }`.

- `MentionCtx` = vues empruntées : noms de salons (`&[Channel]` du serveur
  courant), `&message.mentions` (users), rôles (`&[Role]`) → résolution :
  - `<#id>` → `#nom-salon` (couleur accent), `target_id` pour navigation.
  - `<@id>` / `<@!id>` → `@nom` (depuis `mentions`), pastille violet clair.
  - `<@&id>` → `@nom-rôle` (couleur du rôle si non nulle).
  - id inconnu → fallback `#salon-inconnu` / `@inconnu` (jamais le code brut).
- `<t:epoch:style>` → date lisible (réutilise `format_timestamp`).
- `||texte||` → `Spoiler`.
- Markdown : reprend la logique de `markdown.rs` (gras/italique/barré/code
  inline) + `__souligné__` ; emojis custom/Unicode réutilisent `emoji.rs`
  (`split_emojis` appelé sur les segments `Text`).
- Liens `http(s)://…` autodétectés → `Link`.

**Blocs** (hors tokenizer inline) gérés par un pré-découpage du contenu en
blocs : ```` ```lang\n…``` ```` → bloc code monospace encadré ; lignes `> ` →
citation (barre verticale). Le reste = paragraphes passés au tokenizer inline.

Rendu : `render_inline(ui, &[Inline], interactions)` — mentions en pastilles
(`Sense::click`), liens via `ctx.open_url`, spoilers floutés/masqués révélés au
clic (état dans `egui` memory, clé = id message + index).

## 6. Phase 3 — Embeds & réponses (`veloce-app/src/embed.rs`, nouveau)

- `render_embed(ui, &Embed)` : carte avec **barre couleur** à gauche (4px,
  `color` ou gris), fond légèrement contrasté, padding ; dans l'ordre : auteur
  (icône + nom), titre (gras, lien si `url`), description (via tokenizer inline),
  champs (`inline` → côte à côte, sinon empilés), image (large) / thumbnail
  (petite à droite), footer (petit gris). Couvre aussi les **aperçus de liens**
  (Discord les fournit comme embeds de type `link`/`article`/`video`/`image`).
- **Réponse** : si `referenced_message`, afficher au-dessus du message un aperçu
  compact (« ↰ avatar nom : extrait ») cliquable plus tard ; non-cliquable suffit
  pour cette itération mais le rendu est présent.

## 7. Interactivité (état dans `VeloceApp` / `ChatState`)

- **Visionneuse image** : champ `viewer: Option<String>` (url). Affichée en
  overlay plein écran (`egui::Area` + voile sombre), clic hors image / Échap →
  ferme. Clic sur une image inline ou un embed image l'ouvre.
- **Popup profil** : champ `profile: Option<User>`. `egui::Window` non
  redimensionnable : grand avatar, `global_name`, `@username`. Ouvert au clic sur
  un avatar ou une mention user.
- **Liens** : `ctx.open_url(OpenUrl::new_tab(url))` (aucune dépendance).
- **Navigation salon** : clic sur une mention `#salon` (si dans le serveur
  courant) → `selected_channel = id` + `FetchHistory` (même chemin que le panneau
  Salons).

## 8. Flux de données

Aucun changement de commandes/events : les nouveaux champs de `Message`
remontent par les chemins existants (`FetchHistory` → `MessagesLoaded`,
`MessageCreated`, gateway). `GuildChannels` transporte déjà `roles` (enrichis de
`name`/`color`) et `channels` (pour résoudre les mentions de salon). Les `User`
de `Ready`/messages portent désormais `avatar`.

## 9. Tests

- **Modèles** : fixture JSON d'un message avec `attachments`, `embeds`,
  `mentions`, `referenced_message` → désérialisation OK ; rétro-compat (message
  minimal sans ces champs reste valide).
- **`cdn`** : `avatar_url` (gif si `a_`), `default_avatar_url` (index pour
  discriminator `0` vs legacy), `is_image` (par content_type et par extension).
- **`inline::tokenize`** (cœur) : `<#id>`/`<@id>`/`<@&id>` résolus et fallback ;
  `||spoiler||` ; `<t:…>` ; lien autodétecté ; markdown + emoji combinés ; bloc
  de code multi-ligne isolé ; mention inconnue → label de repli, jamais brut.
- **`group_messages`** : regroupement même auteur < 7 min, coupure si auteur
  différent ou écart > 7 min, premier message = en-tête.
- **`format_timestamp`** : ISO valide → chaîne attendue ; entrée vide/invalide →
  repli sans panique.

## 10. Critères de succès

1. Les messages s'affichent avec **avatar + en-tête nom·heure**, consécutifs
   regroupés, comme Discord. (manuel)
2. Les **images** postées s'affichent inline et s'ouvrent en grand au clic.
   (manuel)
3. Mentions `#salon` / `@user` / `@rôle` rendues en **pastilles colorées**
   lisibles ; clic `#salon` navigue. (manuel)
4. Les **embeds**/aperçus de liens s'affichent en cartes ; spoilers masqués
   révélés au clic ; liens ouvrent le navigateur. (manuel)
5. `inline`, `cdn`, `group_messages`, `format_timestamp` couverts par tests
   unitaires verts.
6. Aucune régression : connexion → serveurs → salons → messages → envoi ;
   plugins ; emojis couleur existants ; `cargo build`/`clippy`/`fmt` propres.

## 11. Hors périmètre

Réactions, stickers, threads/forums, édition/suppression de message depuis l'UI,
lecture vidéo/audio inline (les vidéos d'embed restent un lien/thumbnail),
upload de fichiers, recherche dans le profil. Saut entre serveurs au clic sur une
mention de salon d'un **autre** serveur (seul le serveur courant est résolu).

## 12. Risques

| Risque | Mitigation |
|---|---|
| Beaucoup d'images → charge réseau/mémoire (loader egui) | Taille bornée, chargement paresseux du loader egui ; visionneuse à la demande |
| Parsing ISO 8601 sans `chrono` | Extraction manuelle robuste + test repli ; ajouter `chrono`/`time` seulement si nécessaire |
| Tokenizer = réécriture du rendu | Module **pur** et testé ; `markdown.rs`/`emoji.rs` réutilisés, pas jetés ; bascule derrière `render_inline` |
| Résolution de rôle (couleur/nom) si `Role` incomplet | `name`/`color` en `default` ; fallback `@inconnu` sans couleur |
| Champs `avatar`/embeds absents sur vieux payloads | `#[serde(default)]` partout ; fallback avatar par défaut |

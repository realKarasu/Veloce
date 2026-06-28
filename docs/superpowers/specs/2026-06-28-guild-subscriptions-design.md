# Veloce — Spec de conception : abonnements de guilde (op 14)

**Date :** 2026-06-28
**Statut :** Validé (design approuvé)
**Périmètre :** abonnement gateway op 14 pour le temps réel sur gros serveurs.

---

## 1. Vision & caveat

Sur les comptes **utilisateur**, Discord ne diffuse les `MESSAGE_CREATE` des
**gros serveurs** que si le client envoie une frame **op 14** (« lazy guild
request ») pour s'abonner à la guilde. Sans elle, les nouveaux messages
n'arrivent pas en temps réel sur ces serveurs (sur les petits, ça marche déjà).

**Caveat assumé :** c'est de l'**API user non documentée**. Le format de la
frame peut évoluer côté Discord, et l'efficacité ne se vérifie que sur un **vrai
gros serveur** — donc **vérification manuelle**, pas de test automatisé
possible. La construction de la frame est **isolée dans une seule fonction**
pour être maintenue facilement (même approche que `identity` pour les
super-properties).

## 2. Architecture — ajouter une entrée vers le gateway

Aujourd'hui, le flux est `app → NetHandle.send(Command) → REST`, et la tâche
`run_gateway` n'a qu'une **sortie** d'`Event`. Les commandes gateway (op 14)
doivent atteindre le **gateway**, pas le REST. On ajoute un canal dédié
**app → tâche gateway** qui contourne la boucle REST :

```
app → NetHandle.subscribe_guild(gid) → [canal GatewayCommand] → run_gateway → frame op 14
```

### Frontières des unités
- **`veloce-discord`**
  - `enum GatewayCommand { SubscribeGuild(Snowflake) }` (vocabulaire net→gateway).
  - `run_gateway` gagne un paramètre `gw_cmd_rx: UnboundedReceiver<GatewayCommand>`.
  - `connect_once` gagne ce receiver + un `&mut HashSet<Snowflake>` d'abonnements désirés.
  - `pub fn build_guild_subscribe(guild_id: &str) -> serde_json::Value` (pur, testable, isolé).
- **`veloce-app`**
  - `spawn_net` crée le canal gateway et le passe à `run_gateway` ; `NetHandle`
    stocke le `gw_cmd_tx` et expose `subscribe_guild(&self, Snowflake)`.
  - `draw_chat` appelle `net.subscribe_guild(gid)` au clic sur un serveur.

L'enum `Command` REST existant et `handle_command` sont **inchangés** (les
commandes gateway ne passent pas par eux).

## 3. Comportement dans `connect_once`

- **Nouvelle branche `select!`** : à réception d'un
  `GatewayCommand::SubscribeGuild(gid)` → `subscribed.insert(gid)` ; si déjà
  connecté (`hb_started == true`), envoyer la frame op 14 immédiatement.
- **Re-souscription sur (re)connexion** : juste après avoir traité un `READY`,
  ré-émettre op 14 pour **tous** les guilds de `subscribed`. Discord oublie les
  abonnements à chaque nouvelle session ; cette ré-émission rend l'abo robuste
  aux coupures.
- `subscribed` vit dans `run_gateway` (persiste entre les sessions) et est passé
  en `&mut` à `connect_once`.

## 4. La frame op 14

```json
{
  "op": 14,
  "d": { "guild_id": "<gid>", "typing": true, "activities": true, "threads": false, "channels": {} }
}
```

Format « plat » (le plus documenté pour s'abonner à une guilde). Construit par
`build_guild_subscribe`, **unique point de maintenance** si le format change.

## 5. Câblage UI

Dans `draw_chat`, au clic sur un serveur (là où `Command::SelectGuild` est déjà
envoyé), ajouter `net.subscribe_guild(g.id.clone())`. Aucun changement visible
pour l'utilisateur.

## 6. Périmètre

### Inclus
- Canal app→gateway + `GatewayCommand::SubscribeGuild`.
- `build_guild_subscribe` (pur, isolé).
- Abonnement au clic sur un serveur + re-souscription sur READY.

### Hors périmètre
- Listes de membres / ranges par salon (`channels` reste `{}`).
- Indicateurs de frappe affichés, présences.
- Désabonnement au déselect (on accumule dans `subscribed` — sans danger).

### Critères de succès
1. `build_guild_subscribe(gid)` produit `{op:14, d:{guild_id:gid, typing, activities, threads, channels}}`.
2. Sélectionner un serveur envoie une frame op 14 sur la WebSocket (vérifiable
   par log/inspection).
3. Après une reconnexion, les guilds précédemment sélectionnés sont
   ré-abonnés (op 14 ré-émis au READY).
4. **(Manuel)** Sur un gros serveur, les nouveaux messages arrivent désormais en
   temps réel.
5. Aucune régression : petits serveurs, envoi/lecture, plugins inchangés.

## 7. Tests

- **`build_guild_subscribe`** (pur) : `op == 14`, `d.guild_id == gid`, flags
  `typing`/`activities`/`threads` présents, `channels` présent.
- Plomberie canal + branche `select!` + re-souscription : vérifiées par
  compilation + clippy ; l'effet réel = critère manuel #4.

## 8. Risques

| Risque | Mitigation |
|---|---|
| Format op 14 obsolète/incorrect | Isolé dans `build_guild_subscribe` ; vérif manuelle ; ajustable en un point |
| Frame envoyée avant le handshake | N'émettre que si `hb_started` ; sinon stockée dans `subscribed` et émise au READY |
| Abonnements perdus à la reconnexion | Re-souscription systématique au READY |
| Discord flag l'op 14 « anormal » | Garder des flags réalistes (typing/activities), comme le client officiel |

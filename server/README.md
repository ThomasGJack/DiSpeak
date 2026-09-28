# DiSpeak Server - Serveur temps réel

## 🎯 Rôle

Le serveur DiSpeak gère les communications en temps réel :
- 💬 Chat textuel via WebSocket
- 🎤 Signaling WebRTC pour la voix
- 👥 Tracking des utilisateurs par canal
- 📝 Persistance des messages

## 🚀 Démarrage rapide

### Mode recommandé (binaire distribué + clé propriétaire)

Le workflow recommandé pour distribuer le serveur temps réel à un utilisateur final est maintenant :

1. Créer le serveur dans l'application / backend
2. Copier le `server_id`
3. Ouvrir `Profil` dans le frontend et copier la **clé propriétaire serveur**
4. Renseigner `server-config.toml` (voir `server/server-config.toml.example`)
5. Lancer le binaire `dispeak-server`

Au démarrage, le serveur :
- valide `server_id + owner_key` auprès du backend
- récupère un snapshot de synchronisation (serveur/membres/structure/roles/permissions)
- ne démarre WS/API interne **que si la validation réussit** (fail-fast)

### 1. Installation des dépendances

```bash
cd server
cargo build
```

### 2. Configuration (recommandé: TOML, legacy: `.env`)

#### Option A (recommandée) — `server-config.toml`

```bash
cp server-config.toml.example server-config.toml
```

Puis lancer :

```bash
cargo run -- run --config ./server-config.toml
```

Valider la config :

```bash
cargo run -- validate-config --config ./server-config.toml
```

UI frontend dédiée (nouveau workflow):
- `#/servers/binary/new` : crée/sélectionne un serveur backend et exporte un `server-config.toml` prérempli (sans clé)
- `#/profile` : récupère la clé propriétaire à coller dans `owner_key`

Afficher un template :

```bash
cargo run -- print-config-example
```

#### Option B (legacy) — `.env`

Copier le fichier d'exemple :
```bash
cp .env.example .env
```

Par défaut :
- Port : `8001`
- Database : `server.db`
- Host : `127.0.0.1`

### 3. Lancer le serveur

```bash
cargo run
```

OU utiliser le script :
```bash
start.bat
```

### 4. Vérifier le démarrage

Tu devrais voir :
```
🚀 DiSpeak Server démarré sur 127.0.0.1:8001
```

## 📡 Comment se connecter

### Depuis le frontend

Le frontend se connecte automatiquement quand tu rejoins un serveur :
```typescript
ws://localhost:8001?token=<JWT>
```

### Test manuel avec wscat

```bash
# Installer wscat
npm install -g wscat

# Se connecter (JWT backend requis)
wscat -c "ws://localhost:8001?token=<JWT>"

# Tu recevras un message de connexion
< {"action":"connected","data":{"client_id":"..."}}

# Rejoindre un serveur (membership requis côté serveur)
> {"action":"join_server","data":{"server_id":"srv-123","user_id":"<user-from-jwt>","username":"ignored-or-validated"}}

# Rejoindre un canal texte
> {"action":"join_channel","data":{"channel_id":"chan-123","user_id":"<user-from-jwt>","username":"alice"}}

# Recevoir l'historique
< {"action":"message_history","data":{"channel_id":"chan-123","messages":[]}}

# Envoyer un message
> {"action":"send_message","data":{"channel_id":"chan-123","user_id":"<user-from-jwt>","username":"alice","content":"Hello!"}}

# Le message est broadcast à tous les users du canal
< {"action":"new_message","data":{...}}

# Exemple action de gestion avec réponse corrélée (request_id)
> {"action":"set_channel_permissions","data":{"channel_id":"chan-123","role_id":"role-1","allow_bitfield":2,"deny_bitfield":0,"user_id":"<user-from-jwt>","request_id":"req-1"}}
< {"action":"set_channel_permissions_ack","data":{"channel_id":"chan-123","role_id":"role-1","allow_bitfield":2,"deny_bitfield":0,"server_id":"srv-123","type":"channel_role","request_id":"req-1"}}
```

## 🔍 Fonctionnalités

### Chat textuel

✅ Historique des 50 derniers messages par canal
✅ Broadcast uniquement aux users du canal
✅ Persistance en base de données
✅ Support multi-canaux

### Canaux vocaux

✅ Tracking des utilisateurs connectés
✅ Signaling WebRTC (offer, answer, ICE candidates)
✅ Routing des messages entre peers
✅ Notifications d'entrée/sortie
✅ Snapshot de participants vocaux (`voice_room_snapshot`) au join/reconnect

### Gestion des connexions

✅ Nettoyage automatique à la déconnexion
✅ Message de connexion `connected`
✅ Gestion des erreurs

## 📚 Documentation complète

Consulte **`CONFIG.md`** pour :
- Liste complète des actions WebSocket
- Format des messages
- Architecture interne
- Dépannage

Guides utilisateur (workflow binaire / clé propriétaire) :
- `../docs/09-heberger-un-serveur-binaire.md`
- `../docs/10-configurer-server-config-toml.md`
- `../docs/11-cle-proprietaire-et-liaison-backend.md`

## 🛠️ Développement

### Structure du code

```
server/
├── src/
│   ├── main.rs        # Point d'entrée, gestion WebSocket
│   ├── voip.rs        # Structures WebRTC (futur)
│   ├── db.rs          # Fonctions base de données
│   └── models.rs      # Modèles de données
├── migrations/
│   └── 001_initial.sql # Schéma de base
├── Cargo.toml         # Dépendances
└── start.bat          # Script de lancement
```

### Ajouter une nouvelle action WebSocket

1. Dans `handle_message()` de `main.rs`
2. Ajouter un match arm pour ton action
3. Traiter les données
4. Broadcaster ou répondre

Exemple :
```rust
match msg.action.as_str() {
    "ton_action" => {
        // Traiter l'action
        let response = WebSocketMessage {
            action: "reponse".to_string(),
            data: json!({ ... }),
        };
        send_to_client(client_id, &response, &state.clients).await?;
    }
    // ...
}
```

## ⚡ Performance

- HashMap en mémoire pour tracking rapide
- RwLock pour accès concurrent
- Broadcast ciblé (pas à tous)
- Limite de 50 messages en historique

## 🔐 Sécurité

- [x] Authentification JWT obligatoire sur la connexion WebSocket (`?token=<JWT>`)
- [x] Mode binaire distribué: validation propriétaire au démarrage (bootstrap backend fail-fast)
- [x] Validation JWT RS256 via JWKS standard (JWK RSA `n`/`e`) pour compatibilité interop
- [x] Validation d’identité côté serveur (le client ne fait plus autorité sur `user_id`)
- [x] API HTTP interne protégée (`/health` public, `/api/*` avec `Authorization: Bearer <SERVER_INTERNAL_API_TOKEN>`)
- [x] Support config TOML (`server-config.toml`) avec fallback `.env` legacy
- [ ] Validation des permissions par canal (compléments / durcissement)
- [ ] Rate limiting par user
- [ ] Sanitization des messages

## ⚠️ Note protocole

Le protocole historique documenté dans ce fichier contient encore des exemples anciens.
La source de vérité des noms d’actions/événements est `shared/src/events.rs` (enums `ClientEvent` / `ServerEvent`).

Conventions actuelles importantes :
- Le backend expose un JWKS standard sur `/api/auth/jwks` (JWK RSA `n` / `e`, `kid` optionnel).
- Hors dev, le backend peut basculer par défaut en `RS256` si `JWT_SIGNING_MODE` n’est pas défini.
- Les endpoints legacy `register-auto` / `heartbeat-token` sont dépréciés et peuvent être refusés hors dev.
- Les actions de gestion (canaux/catégories/rôles/permissions) peuvent inclure `data.request_id`.
- Le serveur renvoie des `*_ack` corrélés avec le même `request_id`.
- Les erreurs renvoyées sur ces actions peuvent aussi inclure `request_id`.
- `join_server` retourne `join_server_ack`, puis `presence_snapshot`, puis `server_structure`.
- Les mises à jour de présence temps réel utilisent `presence_delta` (avec `seq`) ; faire un resync via snapshot après reconnexion.
- Les réordonnancements batch sont supportés via `reorder_channels` / `reorder_categories`.

Exemple batch reorder (catégorie) :
```json
{
  "action": "reorder_categories",
  "data": {
    "server_id": "srv-123",
    "ordered_category_ids": ["cat-2", "cat-1", "cat-3"],
    "user_id": "<user-from-jwt>",
    "request_id": "req-reorder-cat-1"
  }
}
```

Réponse :
```json
{
  "action": "category_reorder_ack",
  "data": {
    "server_id": "srv-123",
    "ordered_category_ids": ["cat-2", "cat-1", "cat-3"],
    "request_id": "req-reorder-cat-1"
  }
}
```

## 📝 Notes

- Le serveur ne stocke PAS les infos utilisateur (gérées par le backend)
- Les canaux sont créés dynamiquement (pas de création explicite)
- Les messages sont persistés dans `server.db`
- Le WebRTC signaling est transparent (routing seulement)

Bon développement ! 🚀


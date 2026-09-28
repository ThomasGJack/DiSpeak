# Configuration du Serveur DiSpeak

## 🚀 Vue d'ensemble

Le serveur DiSpeak gère :
- ✅ Connexions WebSocket temps réel
- ✅ Chat textuel avec historique
- ✅ Canaux vocaux (WebRTC signaling)
- ✅ Tracking des utilisateurs par canal
- ✅ Base de données SQLite dédiée

## 📋 Prérequis

- Rust (dernière version stable)
- SQLite3
- Port 8001 disponible

## 🔧 Configuration

### 0. Nouveau mode recommandé: binaire + `server-config.toml`

Le serveur supporte désormais un fichier `server-config.toml` (prioritaire) avec fallback `.env` legacy.

Workflow recommandé :
- créer le serveur côté backend/app
- récupérer `server_id`
- récupérer la clé propriétaire dans `Profil`
- renseigner `server-config.toml`
- lancer le binaire serveur (bootstrap backend fail-fast)

Fichiers utiles :
- `server/server-config.toml.example`
- `../docs/09-heberger-un-serveur-binaire.md`
- `../docs/10-configurer-server-config-toml.md`
- `../docs/11-cle-proprietaire-et-liaison-backend.md`

### 1. Variables d'environnement

Créer un fichier `.env` dans le dossier `server/` :

```env
# Base de données
DATABASE_URL=sqlite:server.db

# Port du serveur
SERVER_PORT=8001

# Adresse d'écoute
SERVER_HOST=127.0.0.1

# API HTTP interne (backend -> serveur)
SERVER_INTERNAL_API_HOST=127.0.0.1
# SERVER_INTERNAL_API_PORT=8002
SERVER_INTERNAL_API_TOKEN=change-me-internal-api-token

# Mode hebergement binaire (recommande)
# OWNER_KEY=dso_xxxxx_xxxxxxxxx
# SERVER_ENFORCE_OWNER_VALIDATION=true
# SERVER_BOOTSTRAP_TIMEOUT_MS=5000
# SERVER_ALLOW_INSECURE_HTTP_BACKEND=false
# SERVER_ALLOW_LEGACY_HS256=false
```

### 1.b `server-config.toml` (recommandé)

Exemple minimal :

```toml
[server]
id = "uuid-du-serveur"
database_url = "sqlite:server.db"
ws_host = "127.0.0.1"
ws_port = 8001
internal_api_host = "127.0.0.1"
internal_api_port = 8002
internal_api_token = "change-me-internal-api-token"

[backend]
url = "http://127.0.0.1:8000"
owner_key = "dso_xxxxx_xxxxxxxxx"

[security]
enforce_owner_validation = true
allow_insecure_http_backend = true
allow_legacy_hs256 = true
```

### 2. Structure de la base de données

La base de données `server.db` contient :

**Table `channels`** :
- `id` : Identifiant unique du canal
- `server_id` : ID du serveur propriétaire
- `name` : Nom du canal (ex: "général", "vocal-1")
- `type` : Type de canal ("text" ou "voice")
- `created_at` : Date de création

**Table `messages`** :
- `id` : Identifiant unique du message
- `channel_id` : Canal où le message a été envoyé
- `user_id` : Auteur du message
- `content` : Contenu du message
- `created_at` : Date d'envoi

**Table `participants`** :
- `id` : Identifiant unique
- `server_id` : Serveur concerné
- `user_id` : Utilisateur participant
- `role` : Rôle (admin, moderator, user)
- `joined_at` : Date d'entrée

### 3. Migrations

Les migrations se lancent automatiquement au démarrage :

```rust
sqlx::migrate!("./migrations").run(&pool).await?;
```

Fichiers de migration dans `server/migrations/` :
- `001_initial.sql` - Tables de base

## 🌐 Protocole WebSocket

⚠️ Les exemples ci-dessous contiennent des messages historiques. La source de vérité des noms d’actions/événements est `shared/src/events.rs`.
✅ Le protocole actuel impose un JWT au handshake WebSocket (`?token=<JWT>`).
✅ En mode binaire distribué, le serveur récupère les infos de validation JWT (RS256/JWKS standard ou fallback legacy) via le bootstrap backend avant l’ouverture du socket.
✅ Le JWKS backend est désormais standard (JWK RSA avec `n` / `e`, et `kid` si configuré).

### URL de connexion

```
ws://localhost:8001?token=<JWT>
```

### Format des messages

Tous les messages utilisent le format JSON :

```json
{
  "action": "nom_action",
  "data": { ... }
}
```

Pour les opérations de gestion (canaux / catégories / rôles / permissions), le client peut inclure un identifiant de corrélation :

```json
{
  "action": "set_channel_permissions",
  "data": {
    "channel_id": "chan-123",
    "role_id": "role-1",
    "allow_bitfield": 2,
    "deny_bitfield": 0,
    "user_id": "user-123",
    "request_id": "req-123"
  }
}
```

Le serveur répond alors avec un `*_ack` contenant le même `request_id` (ou `error` avec `request_id` si l’action échoue).

### Conventions request/response (actuelles)

Actions de gestion avec ack corrélé (non exhaustif, mais aligné avec l’implémentation actuelle) :
- `create_channel` → `channel_create_ack`
- `update_channel` → `channel_update_ack`
- `delete_channel` → `channel_delete_ack`
- `move_channel` → `channel_move_ack`
- `reorder_channels` → `channel_reorder_ack`
- `create_category` → `category_create_ack`
- `update_category` → `category_update_ack`
- `delete_category` → `category_delete_ack`
- `move_category` → `category_move_ack`
- `reorder_categories` → `category_reorder_ack`
- `create_role` → `role_create_ack`
- `update_role` → `role_update_ack`
- `delete_role` → `role_delete_ack`
- `assign_role` → `role_assign_ack`
- `remove_role` → `role_remove_ack`
- `set_channel_permissions` → `set_channel_permissions_ack`
- `set_category_permissions` → `set_category_permissions_ack`
- `set_member_channel_permissions` → `set_member_channel_permissions_ack`
- `set_member_category_permissions` → `set_member_category_permissions_ack`
- `get_effective_permissions` → `effective_permissions`

## 📡 Actions WebSocket disponibles

### Client → Serveur

#### Chat textuel

**`join_channel`** - Rejoindre un canal texte (alias legacy `join_text_channel` encore toléré)
```json
{
  "action": "join_channel",
  "data": {
    "channel_id": "chan-123",
    "user_id": "user-123",
    "username": "alice"
  }
}
```

**`leave_server`** - Quitter le serveur (l’état de canal est nettoyé côté serveur)
```json
{
  "action": "leave_server",
  "data": {
    "server_id": "srv-123",
    "user_id": "user-123",
    "username": "alice"
  }
}
```

**`send_message`** - Envoyer un message
```json
{
  "action": "send_message",
  "data": {
    "channel_id": "chan-123",
    "user_id": "user-123",
    "username": "alice",
    "content": "Bonjour tout le monde !"
  }
}
```

#### Canaux vocaux

**`join_voice_channel`** - Rejoindre un canal vocal
```json
{
  "action": "join_voice_channel",
  "data": {
    "channel_id": "voice-123"
  }
}
```

**`leave_voice_channel`** - Quitter un canal vocal
```json
{
  "action": "leave_voice_channel",
  "data": {
    "channel_id": "voice-123"
  }
}
```

#### WebRTC Signaling

**`webrtc_offer`** - Envoyer une offre WebRTC
```json
{
  "action": "webrtc_offer",
  "data": {
    "peer_id": "user-456",
    "offer": { ... }
  }
}
```

**`webrtc_answer`** - Répondre à une offre
```json
{
  "action": "webrtc_answer",
  "data": {
    "peer_id": "user-789",
    "answer": { ... }
  }
}
```

**`ice_candidate`** - Envoyer un candidat ICE
```json
{
  "action": "ice_candidate",
  "data": {
    "peer_id": "user-789",
    "candidate": { ... }
  }
}
```

### Serveur → Client

#### Système

**`connected`** - Message de connexion
```json
{
  "action": "connected",
  "data": {
    "client_id": "uuid-client"
  }
}
```

**`join_server_ack`** - Ack de jointure serveur (émis avant `presence_snapshot` puis `server_structure`)
```json
{
  "action": "join_server_ack",
  "data": {
    "server_id": "srv-123",
    "structure_version": 1739999999999,
    "presence_seq": 12,
    "voice_seq": 0
  }
}
```

**`presence_snapshot`** - Snapshot autoritatif des membres (join/reconnect)
```json
{
  "action": "presence_snapshot",
  "data": {
    "server_id": "srv-123",
    "seq": 12,
    "members": [{ "user_id": "user-1", "username": "alice", "is_online": true }]
  }
}
```

**`presence_delta`** - Delta de présence séquencé
```json
{
  "action": "presence_delta",
  "data": {
    "server_id": "srv-123",
    "seq": 13,
    "changes": [{ "user_id": "user-2", "username": "bob", "is_online": false }]
  }
}
```

#### Chat

**`message_history`** - Historique des messages
```json
{
  "action": "message_history",
  "data": {
    "channel_id": "chan-123",
    "messages": [
      {
        "id": "msg-1",
        "channel_id": "chan-123",
        "user_id": "user-1",
        "content": "Hello",
        "created_at": "2025-10-26T20:00:00Z"
      }
    ]
  }
}
```

**`new_message`** - Nouveau message
```json
{
  "action": "new_message",
  "data": {
    "id": "msg-2",
    "channel_id": "chan-123",
    "user_id": "user-2",
    "content": "Salut !",
    "created_at": "2025-10-26T20:01:00Z"
  }
}
```

#### Vocal

**`voice_joined`** - Confirmation d'entrée en vocal
```json
{
  "action": "voice_joined",
  "data": {
    "channel_id": "voice-123",
    "user_id": "user-1",
    "username": "alice",
    "peer_id": "peer-123"
  }
}
```

**`voice_participant_joined`** - Nouvel utilisateur en vocal
```json
{
  "action": "voice_participant_joined",
  "data": {
    "channel_id": "voice-123",
    "user_id": "user-3",
    "username": "bob",
    "peer_id": "peer-456"
  }
}
```

**`voice_participant_left`** - Utilisateur quitte le vocal
```json
{
  "action": "voice_participant_left",
  "data": {
    "channel_id": "voice-123",
    "user_id": "user-2"
  }
}
```

**`voice_room_snapshot`** - Snapshot des participants d'un salon vocal
```json
{
  "action": "voice_room_snapshot",
  "data": {
    "channel_id": "voice-123",
    "participants": [
      { "user_id": "user-1", "username": "alice", "peer_id": "peer-123", "is_muted": false, "is_deafened": false }
    ]
  }
}
```

#### Réponses de gestion (acks)

Exemple :
```json
{
  "action": "role_update_ack",
  "data": {
    "role_id": "role-123",
    "server_id": "srv-123",
    "name": "Modérateur",
    "permissions_bitfield": 34,
    "request_id": "req-42"
  }
}
```

Batch reorder (exemple catégories) :
```json
{
  "action": "category_reorder_ack",
  "data": {
    "server_id": "srv-123",
    "ordered_category_ids": ["cat-2", "cat-1", "cat-3"],
    "request_id": "req-cat-order-1"
  }
}
```

Événements de diffusion associés :
- `channels_reordered`
- `categories_reordered`

#### JWT / JWKS (mode binaire distribué)

- Endpoint backend: `/api/auth/jwks`
- Format de clé: JWK RSA standard (`kty`, `alg`, `use`, `n`, `e`, `kid?`)
- Le serveur binaire sélectionne en priorité la clé par `kid` (si fourni au bootstrap), sinon la première clé RSA compatible `RS256`.
- `allow_legacy_hs256` reste un fallback de transition (dev uniquement recommandé).

#### Erreurs corrélées (gestion)

```json
{
  "action": "error",
  "data": {
    "message": "You don't have permission to manage roles",
    "request_id": "req-42"
  }
}
```

## 🌐 API HTTP interne (backend → serveur)

- `GET /health` : public (sans auth)
- `/api/*` : nécessite `Authorization: Bearer <SERVER_INTERNAL_API_TOKEN>`
- Bind par défaut conseillé : loopback (`SERVER_INTERNAL_API_HOST=127.0.0.1`)
- Les endpoints legacy backend `register-auto` / `heartbeat-token` sont dépréciés (et peuvent être désactivés hors dev).

## 🏗️ Architecture interne

### Gestion des connexions

```rust
// Map des clients connectés
Clients: HashMap<user_id, WebSocketSink>

// Tracking des canaux texte
TextChannels: HashMap<channel_id, HashSet<user_id>>

// Tracking des canaux vocaux
VoiceChannels: HashMap<channel_id, HashSet<user_id>>
```

### Broadcast intelligent

Les messages sont diffusés **uniquement** aux utilisateurs du canal concerné :

```rust
// Récupérer les users du canal
let users = text_channels.get(channel_id);

// Broadcaster uniquement à eux
broadcast_to_channel(&users, &message, &clients).await;
```

### Nettoyage automatique

À la déconnexion d'un client :
1. Retrait de tous les canaux texte
2. Retrait de tous les canaux vocaux
3. Notification aux autres utilisateurs
4. Suppression de la map des clients

## 🧪 Tester le serveur

### 1. Avec wscat (outil CLI)

```bash
# Installer wscat
npm install -g wscat

# Se connecter (JWT requis)
wscat -c "ws://localhost:8001?token=<JWT>"

# Rejoindre le serveur puis un canal
> {"action":"join_server","data":{"server_id":"srv-123","user_id":"<user-from-jwt>","username":"alice"}}
> {"action":"join_channel","data":{"channel_id":"test","user_id":"<user-from-jwt>","username":"alice"}}

# Recevoir la réponse
< {"action":"message_history","data":{"channel_id":"test","messages":[]}}
```

## ⚠️ Legacy / dépréciations runtime

- Endpoints backend legacy:
  - `POST /api/servers/{server_id}/register-auto`
  - `POST /api/servers/{server_id}/heartbeat-token`
- Flags backend de compat (dev uniquement recommandé):
  - `BACKEND_ALLOW_REGISTER_AUTO_SYSTEM_SERVER`
  - `BACKEND_ALLOW_LEGACY_HEARTBEAT_TOKEN`
- Flag serveur pour autoriser le mode legacy sans bootstrap:
  - `SERVER_ALLOW_LEGACY_RUNTIME_FLOW`

### 2. Avec le frontend DiSpeak

1. Lance le backend (API) sur le port 8000
2. Lance le serveur WebSocket sur le port 8001
3. Lance le frontend
4. Connecte-toi et rejoins un serveur
5. Sélectionne un canal → connexion WebSocket automatique

### 3. Avec Postman/Insomnia

1. Créer une connexion WebSocket vers `ws://localhost:8001?token=<JWT>`
2. Envoyer des messages JSON
3. Observer les réponses

## 🐛 Dépannage

### Le serveur ne démarre pas

**Erreur : "Address already in use"**
- Le port 8001 est déjà utilisé
- Solution : Changer `SERVER_PORT` dans `.env`

**Erreur : "Failed to connect to database"**
- Le fichier `server.db` n'existe pas ou n'est pas accessible
- Solution : Le serveur crée automatiquement la BDD au démarrage

### Les messages ne sont pas reçus

**Vérifier :**
1. Le client est bien connecté (message `connected` reçu)
2. Le client a rejoint un canal (`join_channel` / alias legacy `join_text_channel`)
3. Le `channel_id` est correct dans les messages

### Les canaux vocaux ne fonctionnent pas

**Note :** Le serveur fait uniquement le **signaling WebRTC**, pas le transport audio lui-même.

Pour tester :
1. Rejoindre le canal avec `join_voice_channel`
2. Recevoir la liste des `peers`
3. Établir les connexions WebRTC côté client
4. Le serveur route les messages `webrtc_*` entre peers

## 📊 Monitoring

### Logs

Le serveur affiche dans la console :
```
🚀 DiSpeak Server démarré sur 127.0.0.1:8001
Nouvelle connexion WebSocket: uuid-123 (127.0.0.1:xxxxx)
Client uuid-123 rejoint le canal texte chan-1
Connexion fermée: uuid-123
```

### Statistiques

Actuellement, les stats sont en mémoire :
- Nombre de clients connectés
- Utilisateurs par canal texte
- Utilisateurs par canal vocal

## 🔮 Évolutions futures

- [x] Authentification JWT sur WebSocket
- [x] Health check endpoint (`/health`)
- [x] Permissions par canal/catégorie/rôle (base + overrides, UI en cours de finalisation)
- [ ] Chiffrement end-to-end
- [ ] Compression des messages
- [ ] Rate limiting
- [ ] Métriques Prometheus

## 📝 Notes techniques

### Pourquoi WebSocket et pas HTTP ?

- Temps réel bidirectionnel
- Faible latence pour le chat
- Signaling WebRTC nécessite push serveur

### Pourquoi une BDD séparée ?

- Séparation des responsabilités
- Le backend gère l'auth/users/servers
- Le serveur gère uniquement le temps réel
- Scalabilité (possibilité de plusieurs serveurs)

### Performance

- HashMap en mémoire pour accès rapide
- RwLock pour concurrence
- Broadcast ciblé (pas à tous les clients)
- Historique limité à 50 messages

## 🆘 Support

En cas de problème :
1. Vérifier les logs du serveur
2. Vérifier que le backend est démarré
3. Consulter `README.md` pour l'architecture globale
4. Tester avec wscat pour isoler le problème


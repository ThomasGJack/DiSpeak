# Documentation Architecture DiSpeak

## Vue d'ensemble de l'application

DiSpeak est une application de communication en temps réel (type Discord) composée de 3 composants principaux qui fonctionnent ensemble :

1. **Backend API** (Rust/Actix-web) - Port 8000
2. **Serveur WebSocket** (Rust/Tokio) - Port 8001  
3. **Frontend** (React/TypeScript/Tauri) - Port 5173
4. **Crate partagée** (`shared/` Rust) - Contrats communs

## Architecture des composants

### 1. Backend API (Port 8000)

**Rôle principal :**
- Gestion centralisée des données (utilisateurs, serveurs, licences)
- Authentification et autorisation (JWT)
- API REST pour les opérations CRUD
- Surveillance de la santé des serveurs WebSocket

**Fichiers clés :**
- `backend/src/main.rs` - Point d'entrée, configuration HTTP, health checker
- `backend/src/routes/` - Endpoints API (auth, servers, channels, roles, users, admin, licenses)
- `backend/src/models.rs` - Structures de données et constantes de permissions
- `backend/src/auth.rs` - Logique d'authentification JWT
- `backend/src/permissions.rs` - Vérification des permissions par rôle

**Base de données :**
- SQLite : `backend/backend.db`
- Migrations : `backend/migrations/`
- Tables principales : users, servers, server_members, channels, categories, roles, permissions

**Endpoints principaux :**
```
POST   /api/auth/register - Inscription
POST   /api/auth/login - Connexion
GET    /api/servers - Liste des serveurs
POST   /api/servers - Créer un serveur
GET    /api/servers/:id/structure - Structure complète (canaux, catégories, membres)
POST   /api/channels - Créer un canal
POST   /api/roles - Créer un rôle
```

**Fonctionnalités spécifiques :**
- Tâche périodique (10s) vérifiant la santé des serveurs WebSocket
- Marque automatiquement les serveurs offline en cas d'échec de health check
- Gestion des propriétaires de serveurs (owner_id)

---

### 2. Serveur WebSocket (Port 8001)

**Rôle principal :**
- Communication temps réel (messages, présence, événements)
- Gestion des canaux vocaux (WebRTC/SFU)
- Broadcasting des événements aux clients connectés
- Synchronisation avec le Backend via script

**Fichiers clés :**
- `server/src/main.rs` - Serveur WebSocket, gestion des messages
- `server/src/voip.rs` - Selective Forwarding Unit pour audio/vidéo
- `server/src/permissions.rs` - Vérification des permissions côté serveur
- `server/src/bin/sync_backend_data.rs` - Script de synchronisation

**Base de données :**
- SQLite : `server/server.db`
- Copie synchronisée des données du Backend
- Tables : servers, server_members, channels, categories, messages, roles

**Contrats WebSocket typés :**
```rust
use dispeak_shared::events::{
    ClientEvent, ClientEventEnvelope, JoinServerPayload,
    MessageHistoryPayload, ServerEvent, ServerEventEnvelope,
    ServerStructurePayload,
};

// Client → Serveur
ClientEventEnvelope {
    action: ClientEvent::JoinServer,
    data: serde_json::to_value(JoinServerPayload {
        server_id,
        user_id,
        username,
    })?,
};

// Serveur → Client
ServerEventEnvelope {
    action: ServerEvent::ServerStructure,
    data: serde_json::to_value(ServerStructurePayload { server })?,
};

ServerEventEnvelope {
    action: ServerEvent::MessageHistory,
    data: serde_json::to_value(MessageHistoryPayload::new(channel_id, history))?,
};
```

Les anciens littéraux (`action: "join_server"`) restent compatibles côté client, mais toutes les nouvelles implémentations doivent passer par les enums `ClientEvent` et `ServerEvent` afin d’éviter les fautes de frappe et de centraliser la documentation.

**Broadcasting :**
- `broadcast_to_channel()` - Envoie à tous les clients d'un canal
- `broadcast_to_server()` - Envoie à tous les clients d'un serveur
- `send_to_client()` - Envoie à un client spécifique

**Système de permissions :**
- Bitfield permissions (VIEW_CHANNELS, SEND_MESSAGES, MANAGE_CHANNELS, etc.)
- Vérification des rôles (owner, admin, moderator, user)
- Permission overrides par canal/catégorie

---

### 3. Frontend (Port 5173)

**Rôle principal :**
- Interface utilisateur (React + Radix UI + Tailwind CSS)
- Connexion WebSocket temps réel
- Gestion de l'état local (hooks React)
- Application Desktop via Tauri

**Structure des fichiers :**
```
frontend/src/
├── pages/          - Pages principales (Login, Dashboard, ServerView, ServerSettings)
├── components/     - Composants réutilisables (UI, Discord layout)
│   ├── ui/         - Composants Radix UI (Button, Avatar, ScrollArea, etc.)
│   └── discord/    - (supprimés, code intégré dans ServerView)
├── hooks/          - Hooks personnalisés (useAuth, useWebSocket, useChannels, useRoles)
├── lib/            - Utilitaires (api.ts, websocket.ts, utils.ts)
└── styles/         - Styles CSS globaux
```

**Hooks principaux :**

1. **useAuth** (`hooks/useAuth.tsx`)
   - Gestion de l'authentification
   - Stockage du token JWT
   - Informations utilisateur

2. **useWebSocket** (`hooks/useWebSocket.ts`)
   - Connexion WebSocket au serveur
   - Gestion des messages temps réel
   - Reconnexion automatique
   - État : connected, messages, channels, serverMembers, etc.

3. **useChannels** (`hooks/useChannels.ts`)
   - CRUD canaux et catégories
   - Communication avec Backend API et WebSocket

4. **useRoles** (`hooks/useRoles.ts`)
   - Gestion des rôles et permissions
   - Permission overrides

**Client API :**
- `lib/api.ts` - Appels HTTP au Backend (`fetch` natif)
- `lib/websocket.ts` - Client WebSocket avec méthodes dédiées

**Pages principales :**

1. **Login** - Authentification Google OAuth / Email
2. **Dashboard** - Vue d'accueil après connexion
3. **Servers** - Liste des serveurs rejoints
4. **ServerView** - Interface Discord (4 colonnes : serveurs, canaux, chat, membres)
5. **ServerSettings** - Gestion du serveur (canaux, rôles, permissions)

**Application Tauri :**
- Empaquetage desktop Windows/Mac/Linux
- Configuration : `frontend/src-tauri/tauri.conf.json`
- Fenêtre native avec barre de titre personnalisée

### 4. Crate partagée (`shared/`)

**Rôle principal :**
- Centraliser les types de données métiers (`Server`, `Channel`, `ServerMember`, `ChatMessage`, etc.)
- Exposer les enums d’actions temps réel (`ClientEvent`, `ServerEvent`) via `shared::events`
- Fournir des structures de payload typées pour les échanges WebSocket

**Fichiers clés :**
- `shared/Cargo.toml` – Dépendances communes (`serde`, `chrono`, `uuid`)
- `shared/src/types.rs` – Modèles partagés pour le backend, le serveur temps réel et le frontend
- `shared/src/events.rs` – Enveloppes et enums sérialisables pour toutes les actions et événements temps réel
- `shared/src/lib.rs` – Point d’entrée exposant `events` et `types`

---

## Flux de données

### Création d'un serveur

1. Frontend → Backend API : `POST /api/servers` { name, description }
2. Backend crée le serveur dans `backend.db` avec owner_id
3. Backend retourne server_id au Frontend
4. **Synchronisation** : Exécuter `cargo run --bin sync_backend_data` dans `/server`
5. Serveur WebSocket a maintenant les données dans `server.db`
6. Frontend peut rejoindre via WebSocket : `{ action: "join_server" }`

### Envoi d'un message

1. Frontend → WebSocket : `{ action: "send_message", data: { content } }`
2. WebSocket vérifie permissions (SEND_MESSAGES)
3. WebSocket enregistre dans `server.db` table `messages`
4. WebSocket broadcast à tous les clients du canal
5. Frontend reçoit : `{ action: "message", data: { user, content, timestamp } }`
6. Frontend affiche le message dans l'interface

### Rejoindre un serveur

1. Frontend récupère serveur depuis Backend : `GET /api/servers/:id`
2. Frontend obtient structure : `GET /api/servers/:id/structure`
3. Frontend se connecte au WebSocket
4. Frontend envoie : `{ action: "join_server", data: { server_id, ... } }` (l’identité est validée côté serveur via JWT de la connexion WebSocket)
5. WebSocket vérifie si utilisateur est membre (sinon, il refuse l’accès)
6. WebSocket envoie structure complète + liste membres en ligne
7. Frontend affiche interface Discord avec canaux, membres, etc.

---

## Synchronisation Backend ↔ Server

**Problème :**
Le Backend et le Serveur WebSocket ont chacun leur propre base de données SQLite.

**Nouvelle solution :**
- Le serveur WebSocket expose une API HTTP (port `SERVER_PORT + 1`) bâtie avec Axum.
- Endpoints principaux :
  - `GET /health` — supervision et heartbeat
  - `GET /api/servers/:id/structure` — structure complète (catégories, canaux, membres)
  - `GET /api/servers/:id/channels` — canaux textuels/vocaux
  - `GET /api/servers/:id/categories` — catégories
  - `GET /api/servers/:id/members` — membres et statuts
  - `GET /api/servers/:id/channels/:channel_id/messages` — historique paginé
- Le backend consomme directement ces endpoints (ex: `GET /api/servers/:id/structure`) via `reqwest` au lieu de lire `server.db`.
- Les routes `/api/*` du serveur temps réel sont protégées par `Authorization: Bearer <SERVER_INTERNAL_API_TOKEN>` (`/health` reste public).
- Les synchronisations se font à la demande, sans accès croisé aux bases SQLite.
- Les messages temps réel transitent exclusivement via les enums `dispeak_shared::events::{ClientEvent, ServerEvent}` pour garantir la compatibilité cross-services.
 - La connexion WebSocket requiert un JWT (`?token=<JWT>`) validé au handshake; l’identité n’est plus acceptée depuis les payloads client.

**Scripts existants :** `server/src/bin/sync_backend_data.rs` reste disponible pour des migrations ponctuelles, mais n'est plus requis en production.

---

## Guide d'utilisation

### Démarrage complet

**Option 1 : Script automatique (Recommandé)**
```bash
scripts/windows/start.bat
# Choisir option [4] - Demarrer TOUT
```

**Option 2 : Manuel**
```bash
# Terminal 1 - Backend
cd backend
cargo run --bin dispeak-backend

# Terminal 2 - Serveur WebSocket
cd server
cargo run --bin dispeak-server

# Terminal 3 - Frontend/Tauri
cd frontend
npm run tauri:dev
```

### Premiers pas

1. **Lancer l'application** avec `scripts/windows/start.bat` → Option [4]

2. **Créer un compte** :
   - Ouvrir http://localhost:5173
   - S'inscrire avec email/mot de passe ou Google

3. **Créer un serveur** :
   - Dashboard → "Créer un serveur"
   - Nommer le serveur
   - Le serveur est créé dans `backend.db`

4. **Synchroniser** :
   - `scripts/windows/start.bat` → Option [6] - Synchroniser données
   - OU : `cd server && cargo run --bin sync_backend_data`

5. **Rejoindre le serveur** :
   - Cliquer sur le serveur dans la liste
   - L'interface Discord s'affiche avec canaux par défaut

6. **Créer des canaux** :
   - ServerSettings → Onglet "Canaux"
   - Créer catégories et canaux textuels/vocaux

7. **Gérer les rôles** :
   - ServerSettings → Onglet "Rôles"
   - Créer rôles avec permissions personnalisées
   - Assigner rôles aux membres

### Commandes utiles

**Vérifier l'état des services :**
```bash
scripts/windows/start.bat → Option [9]
```

**Créer un serveur de test :**
```bash
scripts/windows/start.bat → Option [7]
# Crée un serveur avec ID "test-server-123" et canaux par défaut
```

**Arrêter tous les services :**
```bash
scripts/windows/start.bat → Option [8]
```

**Créer un administrateur :**
```bash
cd backend
cargo run --bin create_admin
```

### Ports utilisés

- **8000** : Backend API (HTTP REST)
- **8001** : Serveur WebSocket
- **5173** : Frontend (développement)
- **8002** : Health check endpoint (serveur WebSocket)

### Variables d'environnement

**Backend** (`backend/.env`) :
```env
DATABASE_URL=sqlite:backend.db
JWT_SECRET=your-secret-key
PORT=8000
GOOGLE_CLIENT_ID=...
```

**Serveur** (`server/.env`) :
```env
DATABASE_URL=sqlite:server.db
WEBSOCKET_PORT=8001
HEALTH_PORT=8002
```

**Frontend** (`frontend/.env`) :
```env
VITE_API_URL=http://localhost:8000
VITE_WEBSOCKET_URL=ws://localhost:8001
VITE_GOOGLE_CLIENT_ID=...
```

---

## Architecture de la base de données

### Tables principales (Backend)

**users**
- id, email, password_hash, name, google_id, avatar_url, numero
- Stockage des comptes utilisateurs

**servers**
- id, name, owner_id, description, created_at, actual_host, actual_port, is_online
- Informations des serveurs + état de santé

**server_members**
- id, server_id, user_id, username, role, joined_at, last_seen, is_online, status
- Appartenance utilisateurs ↔ serveurs

**channels**
- id, server_id, name, type (text/voice), position, category_id
- Canaux de communication

**categories**
- id, server_id, name, position
- Groupement de canaux

**roles**
- id, server_id, name, permissions (bitfield), color, position
- Rôles avec permissions

**channel_role_overrides / member_channel_overrides**
- Permissions spécifiques par canal

### Tables spécifiques (Server)

**messages**
- id, channel_id, user_id, username, content, timestamp
- Historique des messages (uniquement dans server.db)

**voice_channels**
- Gestion des connexions vocales (WebRTC)

---

## Système de permissions

### Permissions disponibles (bitfield)

```rust
VIEW_CHANNELS        = 1 << 0   // Voir les canaux
SEND_MESSAGES        = 1 << 1   // Envoyer des messages
MANAGE_MESSAGES      = 1 << 2   // Gérer les messages
MANAGE_CHANNELS      = 1 << 3   // Gérer les canaux
MANAGE_ROLES         = 1 << 4   // Gérer les rôles
CONNECT_VOICE        = 1 << 5   // Se connecter au vocal
SPEAK_VOICE          = 1 << 6   // Parler en vocal
ADMINISTRATOR        = 1 << 7   // Accès administrateur complet
```

### Hiérarchie des rôles

1. **owner** - Créateur du serveur, tous les droits
2. **admin** - Administrateur, quasi tous les droits
3. **moderator** - Modérateur, droits limités
4. **user** - Utilisateur standard, droits de base

### Vérification des permissions

**Backend** : `backend/src/permissions.rs`
- `check_permission()` - Vérifie si l'utilisateur a une permission spécifique
- `can_manage_channels()`, `can_manage_roles()`, etc.

**Serveur WebSocket** : `server/src/permissions.rs`
- Vérification identique côté temps réel
- Empêche les actions non autorisées

---

## Débogage et maintenance

### Logs

**Backend** :
```bash
RUST_LOG=debug cargo run --bin dispeak-backend
```

**Serveur WebSocket** :
```bash
RUST_LOG=debug cargo run --bin dispeak-server
```

**Frontend** :
- Console navigateur (F12)
- Logs WebSocket dans `lib/websocket.ts`

### Problèmes courants

**Écran gris sur ServerView** :
- Vérifier que le serveur existe dans `server.db`
- Vérifier la connexion WebSocket (F12 → Network → WS)
- Synchroniser les données : `cargo run --bin sync_backend_data`

**Serveur n'apparaît pas** :
- Vérifier que l'utilisateur est owner ou membre
- Synchroniser Backend → Server
- Vérifier `server_members` table

**Permissions refusées** :
- Vérifier le rôle de l'utilisateur
- Vérifier les permission overrides
- Logs serveur pour voir les checks de permissions

**Port déjà utilisé** :
- `scripts/windows/start.bat` → Option [8] pour arrêter tous les services
- Ou manuellement : `taskkill /F /IM dispeak-backend.exe`

### Réinitialisation complète

```bash
# Arrêter tous les services
scripts/windows/start.bat → Option [8]

# Supprimer les bases de données
del backend\backend.db
del server\server.db

# Redémarrer
scripts/windows/start.bat → Option [4]
```

---

## Fichiers de configuration importants

**Backend** :
- `backend/Cargo.toml` - Dépendances Rust
- `backend/migrations/` - Schéma de base de données

**Serveur** :
- `server/Cargo.toml` - Dépendances Rust
- `server/migrations/` - Schéma de base de données

**Frontend** :
- `frontend/package.json` - Dépendances npm
- `frontend/vite.config.ts` - Configuration Vite
- `frontend/src-tauri/tauri.conf.json` - Configuration Tauri
- `frontend/tailwind.config.js` - Configuration Tailwind CSS

**Scripts** :
- `scripts/windows/start.bat` - Script principal de gestion
- `scripts/windows/start.ps1` - Version PowerShell
- `backend/install.bat` - Installation Backend
- `frontend/install.bat` - Installation Frontend
- `server/install.bat` - Installation Serveur

---

## Résumé du flux complet

1. **Utilisateur lance l'application** → Frontend Tauri s'ouvre
2. **Authentification** → Backend vérifie credentials, génère JWT
3. **Liste des serveurs** → Backend retourne serveurs où user est membre/owner
4. **Rejoindre serveur** → Frontend se connecte au WebSocket
5. **WebSocket authentifie** → Vérifie JWT, ajoute client à la liste
6. **Structure serveur** → WebSocket envoie canaux, catégories, membres
7. **Rejoindre canal** → WebSocket change current_channel du client
8. **Envoyer message** → WebSocket vérifie permissions, enregistre, broadcast
9. **Recevoir message** → Tous les clients du canal reçoivent le message
10. **Déconnexion** → WebSocket marque membre offline, notifie les autres

---

Cette documentation couvre l'architecture complète de DiSpeak. Pour toute question spécifique, consultez les fichiers README dans chaque dossier (backend/, server/, frontend/).

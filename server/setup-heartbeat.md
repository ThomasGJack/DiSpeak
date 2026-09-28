# Configuration du Heartbeat - Guide étape par étape

## 🎯 Objectif

Configurer le serveur pour qu'il signale sa présence au backend.

## 📋 Étapes

### Étape 1 : Créer un compte et se connecter

1. **Lance le backend** (si pas déjà fait) :
   ```bash
   cd backend
   cargo run
   ```

2. **Lance le frontend** :
   ```bash
   cd frontend
   npm run tauri:dev
   ```

3. **Crée un compte** ou connecte-toi

4. **Récupère ton token JWT** :
   - Ouvre la console développeur (F12)
   - Va dans "Application" → "Local Storage" → `file://`
   - Copie la valeur de `token`

### Étape 2 : Créer un serveur dans le backend

**Option A - Via l'API (curl) :**

```bash
curl -X POST http://localhost:8000/api/servers \
  -H "Authorization: Bearer TON_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"name\":\"Mon Serveur\",\"host\":\"127.0.0.1\",\"port\":8001,\"max_users\":50}"
```

**Option B - Via Postman/Insomnia :**

```
POST http://localhost:8000/api/servers
Headers:
  Authorization: Bearer TON_JWT_TOKEN
  Content-Type: application/json
Body:
{
  "name": "Mon Serveur",
  "host": "127.0.0.1",
  "port": 8001,
  "max_users": 50
}
```

**Réponse :**
```json
{
  "id": "abc-123-def-456",  ← COPIE CET ID !
  "owner_id": "...",
  "name": "Mon Serveur",
  ...
}
```

### Étape 3 : Configurer server/.env

Édite `server/.env` et décommente/configure :

```env
# Base de données
DATABASE_URL=sqlite:server.db

# Port du serveur
SERVER_PORT=8001

# Adresse d'écoute
SERVER_HOST=127.0.0.1

# Heartbeat vers le backend
SERVER_ID=abc-123-def-456        ← L'ID retourné à l'étape 2
BACKEND_URL=http://localhost:8000
AUTH_TOKEN=TON_JWT_TOKEN          ← Le token de l'étape 1
```

### Étape 4 : Lancer le serveur

```bash
cd server
cargo run
```

**Tu devrais voir :**
```
🚀 DiSpeak Server démarré sur 127.0.0.1:8001
✅ Heartbeat envoyé au backend
```

### Étape 5 : Vérifier dans le frontend

1. Va sur la page "Serveurs"
2. Ton serveur affiche : ✅ **En ligne** (badge vert)

## ⚠️ Note importante : Licence requise

Pour créer un serveur, tu dois avoir une **licence active**.

Si tu as l'erreur "Vous devez avoir une licence valide", crée-toi une licence :

```bash
curl -X POST http://localhost:8000/api/licenses \
  -H "Content-Type: application/json" \
  -d "{\"user_id\":\"TON_USER_ID\",\"server_slots\":5}"
```

## 🚀 Alternative : Script automatique

Créer un compte admin par défaut qui possède déjà une licence et un serveur.
(Contacte-moi si tu veux que je crée ce script)

## ✅ Vérification rapide

Ton `.env` doit ressembler à ça :

```env
SERVER_ID=abc-123-def-456       ← Pas commenté !
BACKEND_URL=http://localhost:8000
AUTH_TOKEN=eyJhbGc...           ← Ton vrai token
```

Si `SERVER_ID` est commenté (`# SERVER_ID=...`), le heartbeat ne fonctionnera pas !


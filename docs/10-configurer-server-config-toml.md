# Configurer `server-config.toml`

Le binaire `server` lit en priorite `server-config.toml`.
Si le fichier est absent, il peut encore utiliser le fallback `.env` (legacy, deprecie).
Tu peux generer un template pre-rempli depuis l'UI: `#/servers/binary/new` (sans la cle proprietaire).

## 1) Partir de l'exemple

- Copier `server/server-config.toml.example`
- Renommer en `server-config.toml`

## 2) Champs obligatoires

### `[server]`

- `id` : UUID du serveur cree dans le backend
- `database_url` : base SQLite locale (ex: `sqlite:server.db`)
- `ws_host` / `ws_port` : ecoute WebSocket
- `internal_api_host` / `internal_api_port` : API interne du serveur temps reel

### `[backend]`

- `url` : URL du backend (idealement HTTPS)
- `owner_key` : cle proprietaire du profil utilisateur
- `bootstrap_timeout_ms` : timeout bootstrap backend
- `heartbeat_interval_sec` : intervalle des heartbeats runtime

### `[security]`

- `enforce_owner_validation = true` : recommande (obligatoire pour le workflow propriete)
- `allow_insecure_http_backend = false` : garder `false` en prod
- `allow_legacy_hs256 = false` : garder `false` sauf transition legacy (dev)

## 3) Exemple minimal (dev)

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
owner_key = "" # coller ici la cle depuis Profil

[security]
enforce_owner_validation = true
allow_insecure_http_backend = true
allow_legacy_hs256 = false
```

## 4) Commandes utiles (binaire serveur)

```bash
dispeak-server print-config-example
dispeak-server validate-config --config ./server-config.toml
dispeak-server run --config ./server-config.toml
```

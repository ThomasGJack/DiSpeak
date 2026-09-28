# Rejoindre un serveur

## Prerequis

- Backend OK (`8000`)
- Serveur temps reel OK (`8001`)
- Frontend OK (`5173`)
- Utilisateur connecte (JWT present)

## Depuis l'interface

1. Ouvrir `Serveurs`
2. Dans `Serveurs disponibles`, cliquer `Rejoindre`
3. Le frontend ouvre ensuite la vue serveur

## Ce qui se passe en technique

1. Frontend -> backend : `POST /servers/:id/join`
2. Frontend -> WS : connexion avec `?token=<JWT>`
3. Frontend -> WS : `join_server`
4. Serveur WS -> frontend :
- `join_server_ack`
- `presence_snapshot`
- `server_structure`

## Erreurs frequentes

- "Serveur hors ligne" : le backend ne voit pas de heartbeat valide
- "User is not a member of this server" : votre utilisateur n'est pas dans `server_members`
- Reconnexion infinie WS : token JWT invalide ou `JWT_SECRET` backend/server differents

## Astuce debug

- Verifier `http://127.0.0.1:8002/health`
- Verifier les logs backend et serveur WS
- Voir `docs/07-depannage.md`


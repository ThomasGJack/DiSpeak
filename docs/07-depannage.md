# Depannage

## 1) Barre moche en haut de l'interface (web)

Cause typique :
- offset Tauri (`36px`) applique au navigateur

Etat :
- corrige dans le frontend recent (offset conditionnel au runtime Tauri)

## 2) WebSocket refuse la connexion

Verifier :
- `JWT_SECRET` identique entre `backend/.env` et `server/.env`
- serveur WS lance sur `127.0.0.1:8001`
- token JWT non expire

Symptome :
- console navigateur: reconnections + erreur `Event`

## 3) Backend ne demarre pas (migration modified)

Symptome :
- `migration 1 was previously applied but has been modified`

Options :
- repartir d'une DB de dev propre
- restaurer les migrations conformes a la DB existante
- utiliser une DB de dev separee (`sqlite:dev-run.db`)

## 4) Serveur WS ne demarre pas (SQLite)

Symptome :
- `unable to open database file`

Verifier :
- `DATABASE_URL` (chemin sqlite)
- droits d'ecriture sur le dossier
- lancer depuis le bon dossier projet

## 5) Le bouton Rejoindre ouvre rien

Verifier :
- backend `8000` OK
- serveur cible online dans la liste
- heartbeat recu par le backend
- membre present dans `server_members`

## 6) Les serveurs ne s'affichent pas

Verifier :
- utilisateur connecte
- backend repond sur `GET /servers` et `GET /servers/my`
- front `localStorage` contient un token valide

## 7) Checks rapides

```bash
curl http://127.0.0.1:8000/health
curl http://127.0.0.1:8002/health
```

Et pour le frontend :

- ouvrir `http://127.0.0.1:5173`
- verifier la console navigateur


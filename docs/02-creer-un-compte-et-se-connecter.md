# Creer un compte et se connecter

## Prerequis

- Backend lance (`http://127.0.0.1:8000`)
- Frontend lance (`http://127.0.0.1:5173`)

## Connexion

1. Ouvrir `http://127.0.0.1:5173`
2. Aller sur l'ecran de login
3. Utiliser :
- connexion email / mot de passe
- ou Google OAuth (si `VITE_GOOGLE_CLIENT_ID` est configure)

## Ce qui se passe ensuite

- Le frontend stocke un JWT dans le navigateur (`localStorage`)
- Ce JWT est reutilise pour :
- les appels HTTP au backend
- la connexion WebSocket au serveur temps reel (`?token=<JWT>`)

## Verification rapide

- Si la connexion marche mais les serveurs ne se chargent pas :
- verifier `docs/07-depannage.md` (JWT, backend, WS)


# Creer un serveur

## Important

La creation de serveur passe par le backend et demande une licence valide dans la base backend.

Si vous etes en dev et que rien ne se cree, verifier :
- que votre compte a une licence active
- que le backend est bien lance

## Methode 1 - Via l'interface (quand disponible dans votre build)

1. Connectez-vous
2. Ouvrez le dashboard / liste des serveurs
3. Utilisez l'action de creation de serveur
4. Renseignez :
- nom
- host / port
- capacite
5. Validez

## Methode 2 - Via API backend (manuel)

Endpoint backend :

- `POST /api/servers`

Exemple (avec JWT) :

```bash
curl -X POST http://127.0.0.1:8000/api/servers \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <JWT>" \
  -d '{
    "name": "Mon serveur",
    "host": "127.0.0.1",
    "port": 8001,
    "max_users": 100
  }'
```

## Resultat attendu

- Le serveur est cree dans `servers`
- Votre utilisateur devient `owner_id`
- Une entree `server_members` est creee avec role `owner`

## Ensuite

- Voir `docs/04-proprietaire-d-un-serveur.md`
- Voir `docs/05-rejoindre-un-serveur.md`


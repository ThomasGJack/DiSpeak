# Heberger un serveur binaire (workflow proprietaire)

Ce guide explique comment heberger un serveur DiSpeak avec le binaire compile du dossier `server`.

## Prerequis

- Un compte DiSpeak actif
- Un serveur cree dans l'application (backend)
- Votre cle proprietaire visible dans `Profil`
- Le binaire `dispeak-server` (ou `dispeak-server.exe`)
- Le fichier `server-config.toml` (base depuis `server/server-config.toml.example`)

## Workflow (resume)

1. Creer un serveur dans DiSpeak (backend/app)
2. Copier le `server_id`
3. Ouvrir `Profil` et copier la `cle proprietaire serveur`
4. Remplir `server-config.toml`
5. Lancer le binaire serveur
6. Le binaire valide la cle aupres du backend avant de demarrer

## Ce que verifie le binaire au lancement

- `server_id` existe
- la cle proprietaire est valide
- la cle appartient bien au proprietaire de ce serveur
- le backend renvoie un bootstrap valide (token runtime + infos de sync)

Si une verification echoue:
- le serveur **ne demarre pas** (fail-fast)

## Fichiers utiles

- `server/server-config.toml.example`
- `docs/10-configurer-server-config-toml.md`
- `docs/11-cle-proprietaire-et-liaison-backend.md`
- `docs/07-depannage.md`


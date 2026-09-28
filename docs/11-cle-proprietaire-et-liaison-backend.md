# Cle proprietaire et liaison backend (serveur binaire)

## Qu'est-ce que la cle proprietaire ?

C'est une cle personnelle liee a votre compte utilisateur.
Elle permet au backend de verifier qu'un binaire serveur que vous lancez est autorise a se rattacher a **vos** serveurs.

Format attendu:

- `dso_<kid>_<secret>`

## Ou la recuperer ?

- Ouvrir `Profil`
- Section `Cle proprietaire serveur`
- Cliquer `Copier la cle`

## Rotation (regeneration)

Vous pouvez regenerer la cle depuis `Profil`.

Effets:

- l'ancienne cle devient invalide
- les binaires `dispeak-server` configures avec l'ancienne cle doivent etre mis a jour
- le bootstrap au backend echouera tant que la nouvelle cle n'est pas appliquee

## Bonnes pratiques

- ne jamais partager la cle
- ne pas la commit dans un repo
- ne pas l'envoyer en message/public
- stocker `server-config.toml` dans un emplacement prive

## Lien avec la propriete du serveur

Au bootstrap:

1. le binaire envoie `server_id` + `owner_key`
2. le backend verifie la cle
3. le backend verifie que l'utilisateur est bien proprietaire de `server_id`
4. si OK, le backend renvoie un token runtime et un snapshot de sync

Le serveur binaire peut alors reconnaitre le proprietaire une fois connecte.

## Depannage rapide

- `Cle proprietaire invalide`:
  - verifier la copie
  - verifier si une regeneration a ete faite
- `Serveur non possede`:
  - verifier `server_id`
  - verifier le compte connecte
- `Bootstrap refuse`:
  - verifier backend URL / acces reseau
  - verifier HTTPS / flags dev

- Voir aussi `docs/07-depannage.md`


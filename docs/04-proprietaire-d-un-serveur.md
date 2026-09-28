# Proprietaire d'un serveur (owner)

## Comment on devient proprietaire

Dans DiSpeak, le proprietaire est defini par le backend :

- colonne `servers.owner_id`
- plus une entree `server_members` avec role `owner`

Quand vous creez un serveur via le backend avec votre JWT, vous devenez proprietaire automatiquement.

## Comment verifier que vous etes proprietaire

## Depuis l'interface

- Ouvrir `Serveurs`
- Ouvrir votre serveur
- Aller dans `Parametres du serveur`
- Si vous voyez la console de settings (et non "Acces refuse"), vous avez au minimum les droits admin/owner

## Depuis la base backend (verification technique)

Exemple SQLite :

```sql
SELECT id, owner_id, name FROM servers WHERE id = '<SERVER_ID>';
SELECT server_id, user_id, role FROM server_members WHERE server_id = '<SERVER_ID>' AND user_id = '<USER_ID>';
```

## Si vous n'etes pas proprietaire

- Creez un nouveau serveur avec votre compte
- ou demandez au proprietaire actuel de vous donner les droits admin
- ou, en dev local uniquement, modifiez les tables backend (`servers.owner_id` + `server_members.role`)

## Attention

- Le frontend ne decide pas des droits
- Le backend et le serveur temps reel appliquent les verifications de permissions


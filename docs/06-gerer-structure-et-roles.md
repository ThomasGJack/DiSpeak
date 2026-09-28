# Gerer la structure (canaux/categories) et les roles

## Ouvrir la console serveur

1. Ouvrir un serveur
2. Cliquer `Gerer le serveur`
3. Aller dans `Parametres du serveur`

## Onglets principaux

- `Vue d'ensemble`
- `Canaux / Categories`
- `Roles / Membres`
- `Permissions (beta)`

## Canaux / Categories

### Creer

- Renseigner le nom de categorie puis `Creer la categorie`
- Renseigner le nom/type/categorie/description puis `Creer le canal`

### Reorganiser (DnD)

- Glisser-deposer les categories
- Glisser-deposer les canaux dans la meme categorie (batch reorder)
- Deplacement inter-categories : fallback `move_channel` (fonctionnel)

### Supprimer une categorie

- Les canaux sont rebascules en `Sans categorie` (pas perdus)

## Roles / Membres

- Creer / modifier / supprimer des roles (role par defaut protege)
- Assigner / retirer des roles aux membres

## Permissions (beta)

- Matrix canal/role
- Overrides categorie/role
- Overrides membre/canal et membre/categorie
- Calcul des permissions effectives (sur l'utilisateur connecte)

## Bonnes pratiques

- Faire les gros reorders dans `ServerSettings` (DnD)
- Garder les categories courtes et explicites
- Eviter de supprimer des roles utilises sans verifier les membres assignes


# Admin et moderation de serveur

Guide pratique pour administrer un serveur DiSpeak proprement (roles, permissions, moderation et organisation).

## 1) Verifier que vous etes proprietaire / admin

- Voir `docs/04-proprietaire-d-un-serveur.md` pour confirmer la propriete du serveur
- Ouvrir `Gerer le serveur` depuis la vue serveur
- Verifier que vous pouvez acceder aux onglets:
  - `Canaux / Categories`
  - `Roles / Membres`
  - `Permissions (beta)`

Si un onglet/action est refuse:
- verifier vos roles
- verifier les permissions du role dans `Roles / Membres`
- verifier les overrides de canal/categorie

## 2) Mettre en place une structure propre

Ordre recommande:

1. Creer les categories principales (ex: `Accueil`, `Communaute`, `Jeu`, `Staff`, `Vocal`)
2. Creer les canaux texte par usage
3. Creer les salons vocaux par groupe
4. Reordonner avec glisser-deposer dans `ServerSettings`

Bonnes pratiques:
- garder des noms courts et clairs
- limiter le nombre de categories
- reserver une categorie `Staff` pour l'administration
- eviter les suppressions en masse sans verification

## 3) Roles conseilles (base)

Exemple de hierarchy simple:

- `Proprietaire`
- `Admin`
- `Moderateur`
- `Membre`
- `Invite` (optionnel)

Conseils:
- donner les permissions puissantes seulement a `Admin` et `Moderateur`
- garder `Membre` avec permissions minimales utiles
- utiliser des roles separes pour les groupes/specifiques (event, dev, streamer, etc.)

## 4) Assigner / retirer des roles

Dans `Gerer le serveur` > `Roles / Membres`:

1. Creer les roles
2. Selectionner un membre
3. Assigner ou retirer les roles necessaires
4. Verifier le resultat dans la liste des membres

Astuce:
- faire les changements de roles en une passe, puis tester sur un canal cible

## 5) Permissions (beta): comment les utiliser sans se bloquer

Zones disponibles:
- Matrix canal/role
- Overrides categorie/role
- Overrides membre/canal
- Overrides membre/categorie

Regle simple:
- utiliser la matrix role/canal pour la majorite des regles
- utiliser les overrides membre seulement pour les exceptions temporaires

Avant de valider un changement sensible:
- tester avec un compte secondaire si possible
- utiliser le calcul des permissions effectives (beta) pour l'utilisateur connecte

## 6) Moderation quotidienne (workflow conseille)

### Cas 1 - Canal perturbe

1. Retirer temporairement le droit d'ecrire sur le canal (role/canal)
2. Ajuster les roles des membres concernes
3. Reactiver progressivement

### Cas 2 - Espace staff prive

1. Creer categorie `Staff`
2. Retirer acces a `Membre`
3. Autoriser `Moderateur` et `Admin`
4. Verifier les canaux vocaux et texte de cette categorie

### Cas 3 - Evenement temporaire

1. Creer categorie/canaux temporaires
2. Ajouter role d'evenement si besoin
3. Nettoyer ou archiver apres l'evenement

## 7) Checks rapides quand "ca bug"

- Ouvrir `docs/07-depannage.md`
- Verifier que le WebSocket est connecte
- Verifier que les listes membres/voix se mettent a jour en temps reel
- Si un changement de permission ne se voit pas:
  - rafraichir la structure serveur
  - reouvrir la vue serveur
  - verifier les overrides contradictoires

## 8) Bonnes pratiques de securite

- limiter le nombre de comptes `Admin`
- eviter de partager des comptes
- retirer les roles eleves aux comptes inactifs
- utiliser des roles de moderation distincts des roles communautaires
- documenter les regles de moderation dans un canal staff

## Liens utiles

- `docs/04-proprietaire-d-un-serveur.md`
- `docs/06-gerer-structure-et-roles.md`
- `docs/07-depannage.md`


-- Ajouter la colonne username dans les messages pour la persistance
ALTER TABLE messages ADD COLUMN username TEXT DEFAULT 'Utilisateur';

-- Mettre à jour les messages existants avec un nom par défaut
UPDATE messages SET username = 'Utilisateur' WHERE username IS NULL;


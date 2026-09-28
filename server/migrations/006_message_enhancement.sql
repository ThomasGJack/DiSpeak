-- Migration pour améliorer le système de messages avec pagination et historique complet
-- Ajouter des colonnes et index pour optimiser les performances

-- Ajouter des colonnes à la table messages existante
ALTER TABLE messages ADD COLUMN edited_at TEXT;
ALTER TABLE messages ADD COLUMN reply_to_message_id TEXT;
ALTER TABLE messages ADD COLUMN attachments TEXT; -- JSON array des fichiers attachés
ALTER TABLE messages ADD COLUMN reactions TEXT; -- JSON object des réactions
ALTER TABLE messages ADD COLUMN is_pinned BOOLEAN DEFAULT 0;
ALTER TABLE messages ADD COLUMN is_system BOOLEAN DEFAULT 0; -- Messages système (join, leave, etc.)

-- Index pour optimiser les requêtes de pagination
CREATE INDEX IF NOT EXISTS idx_messages_channel_created ON messages(channel_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_messages_channel_id ON messages(channel_id);
CREATE INDEX IF NOT EXISTS idx_messages_user_id ON messages(user_id);
CREATE INDEX IF NOT EXISTS idx_messages_reply_to ON messages(reply_to_message_id);
CREATE INDEX IF NOT EXISTS idx_messages_pinned ON messages(channel_id, is_pinned DESC);

-- Table pour les réactions aux messages
CREATE TABLE IF NOT EXISTS message_reactions (
    id TEXT PRIMARY KEY,
    message_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    emoji TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE,
    UNIQUE(message_id, user_id, emoji)
);

-- Table pour les fichiers attachés
CREATE TABLE IF NOT EXISTS message_attachments (
    id TEXT PRIMARY KEY,
    message_id TEXT NOT NULL,
    filename TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    mime_type TEXT NOT NULL,
    file_url TEXT NOT NULL,
    thumbnail_url TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE
);

-- Index pour les nouvelles tables
CREATE INDEX IF NOT EXISTS idx_message_reactions_message_id ON message_reactions(message_id);
CREATE INDEX IF NOT EXISTS idx_message_reactions_user_id ON message_reactions(user_id);
CREATE INDEX IF NOT EXISTS idx_message_attachments_message_id ON message_attachments(message_id);

-- Table pour les mentions dans les messages
CREATE TABLE IF NOT EXISTS message_mentions (
    id TEXT PRIMARY KEY,
    message_id TEXT NOT NULL,
    mentioned_user_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE,
    FOREIGN KEY (mentioned_user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE(message_id, mentioned_user_id)
);

CREATE INDEX IF NOT EXISTS idx_message_mentions_message_id ON message_mentions(message_id);
CREATE INDEX IF NOT EXISTS idx_message_mentions_user_id ON message_mentions(mentioned_user_id);




-- Migration complète pour la structure des serveurs côté serveur WebSocket
-- Synchronisation avec le backend pour la gestion temps réel

-- Table des catégories de canaux (synchronisé avec backend)
CREATE TABLE IF NOT EXISTS categories (
    id TEXT PRIMARY KEY,
    server_id TEXT NOT NULL,
    name TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Table des canaux (texte et vocal) - synchronisé avec backend
CREATE TABLE IF NOT EXISTS channels (
    id TEXT PRIMARY KEY,
    server_id TEXT NOT NULL,
    name TEXT NOT NULL,
    type TEXT NOT NULL CHECK (type IN ('text', 'voice')),
    category_id TEXT,
    position INTEGER NOT NULL DEFAULT 0,
    description TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Table des permissions de catégories
CREATE TABLE IF NOT EXISTS category_permissions (
    id TEXT PRIMARY KEY,
    category_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('owner', 'moderator', 'user')),
    can_view BOOLEAN DEFAULT 1 NOT NULL,
    can_send_messages BOOLEAN DEFAULT 1 NOT NULL,
    can_manage_messages BOOLEAN DEFAULT 0 NOT NULL,
    UNIQUE(category_id, role)
);

-- Table des permissions de canaux
CREATE TABLE IF NOT EXISTS channel_permissions (
    id TEXT PRIMARY KEY,
    channel_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('owner', 'moderator', 'user')),
    can_view BOOLEAN DEFAULT 1 NOT NULL,
    can_send_messages BOOLEAN DEFAULT 1 NOT NULL,
    can_manage_messages BOOLEAN DEFAULT 0 NOT NULL,
    can_connect_voice BOOLEAN DEFAULT 1 NOT NULL,
    can_speak BOOLEAN DEFAULT 1 NOT NULL,
    UNIQUE(channel_id, role)
);

-- Table des sessions vocales pour WebRTC
CREATE TABLE IF NOT EXISTS voice_sessions (
    id TEXT PRIMARY KEY,
    channel_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    username TEXT NOT NULL,
    peer_id TEXT NOT NULL,
    joined_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(channel_id, user_id)
);

-- Table des membres du serveur (synchronisé avec backend)
CREATE TABLE IF NOT EXISTS server_members (
    id TEXT PRIMARY KEY,
    server_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    username TEXT NOT NULL,
    avatar TEXT,
    role TEXT DEFAULT 'user' NOT NULL CHECK (role IN ('owner', 'moderator', 'user')),
    joined_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
    is_online BOOLEAN DEFAULT 0 NOT NULL
);

-- Index pour optimiser les requêtes
CREATE INDEX IF NOT EXISTS idx_categories_server_id ON categories(server_id);
CREATE INDEX IF NOT EXISTS idx_categories_position ON categories(position);
CREATE INDEX IF NOT EXISTS idx_channels_server_id ON channels(server_id);
CREATE INDEX IF NOT EXISTS idx_channels_category_id ON channels(category_id);
CREATE INDEX IF NOT EXISTS idx_channels_position ON channels(position);
CREATE INDEX IF NOT EXISTS idx_channels_type ON channels(type);
CREATE INDEX IF NOT EXISTS idx_category_permissions_category_id ON category_permissions(category_id);
CREATE INDEX IF NOT EXISTS idx_channel_permissions_channel_id ON channel_permissions(channel_id);
CREATE INDEX IF NOT EXISTS idx_voice_sessions_channel_id ON voice_sessions(channel_id);
CREATE INDEX IF NOT EXISTS idx_voice_sessions_user_id ON voice_sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_server_members_server_id ON server_members(server_id);
CREATE INDEX IF NOT EXISTS idx_server_members_user_id ON server_members(user_id);
CREATE INDEX IF NOT EXISTS idx_server_members_is_online ON server_members(is_online);


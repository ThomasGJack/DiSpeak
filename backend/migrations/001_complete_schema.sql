-- Migration complète pour DiSpeak Backend
-- Création de toutes les tables nécessaires

-- Table des utilisateurs
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    user_number INTEGER,
    username TEXT NOT NULL,
    email TEXT NOT NULL,
    provider TEXT NOT NULL DEFAULT 'local',
    provider_id TEXT,
    password_hash TEXT NOT NULL,
    avatar TEXT,
    status TEXT DEFAULT 'offline' CHECK (status IN ('online', 'away', 'busy', 'offline')),
    role TEXT DEFAULT 'user' CHECK (role IN ('user', 'moderator', 'admin', 'owner')),
    bio TEXT,
    is_active BOOLEAN DEFAULT 1,
    deleted_at DATETIME,
    deleted_by TEXT,
    deletion_reason TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Table des licences
CREATE TABLE licenses (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    server_slots INTEGER NOT NULL DEFAULT 1,
    active BOOLEAN DEFAULT 1,
    expires_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Table des serveurs
CREATE TABLE servers (
    id TEXT PRIMARY KEY,
    owner_id TEXT NOT NULL,
    name TEXT NOT NULL,
    host TEXT NOT NULL,
    port INTEGER NOT NULL,
    max_users INTEGER NOT NULL DEFAULT 50,
    is_online BOOLEAN DEFAULT 0,
    last_heartbeat DATETIME,
    actual_host TEXT,
    actual_port INTEGER,
    icon TEXT,
    color TEXT DEFAULT '#5865F2',
    description TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Table des membres du serveur
CREATE TABLE server_members (
    id TEXT PRIMARY KEY,
    server_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    username TEXT NOT NULL,
    avatar TEXT,
    role TEXT DEFAULT 'user' NOT NULL CHECK (role IN ('owner', 'moderator', 'user')),
    joined_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
    is_online BOOLEAN DEFAULT 0 NOT NULL,
    FOREIGN KEY (server_id) REFERENCES servers(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE(server_id, user_id)
);

-- Table des catégories de canaux
CREATE TABLE categories (
    id TEXT PRIMARY KEY,
    server_id TEXT NOT NULL,
    name TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (server_id) REFERENCES servers(id) ON DELETE CASCADE
);

-- Table des canaux (texte et vocal)
CREATE TABLE channels (
    id TEXT PRIMARY KEY,
    server_id TEXT NOT NULL,
    name TEXT NOT NULL,
    type TEXT NOT NULL CHECK (type IN ('text', 'voice')),
    category_id TEXT,
    position INTEGER NOT NULL DEFAULT 0,
    description TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (server_id) REFERENCES servers(id) ON DELETE CASCADE,
    FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE SET NULL
);

-- Table des permissions de catégories
CREATE TABLE category_permissions (
    id TEXT PRIMARY KEY,
    category_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('owner', 'moderator', 'user')),
    can_view BOOLEAN DEFAULT 1 NOT NULL,
    can_send_messages BOOLEAN DEFAULT 1 NOT NULL,
    can_manage_messages BOOLEAN DEFAULT 0 NOT NULL,
    FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE CASCADE,
    UNIQUE(category_id, role)
);

-- Table des permissions de canaux
CREATE TABLE channel_permissions (
    id TEXT PRIMARY KEY,
    channel_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('owner', 'moderator', 'user')),
    can_view BOOLEAN DEFAULT 1 NOT NULL,
    can_send_messages BOOLEAN DEFAULT 1 NOT NULL,
    can_manage_messages BOOLEAN DEFAULT 0 NOT NULL,
    can_connect_voice BOOLEAN DEFAULT 1 NOT NULL,
    can_speak BOOLEAN DEFAULT 1 NOT NULL,
    FOREIGN KEY (channel_id) REFERENCES channels(id) ON DELETE CASCADE,
    UNIQUE(channel_id, role)
);

-- Table des sessions vocales pour WebRTC
CREATE TABLE voice_sessions (
    id TEXT PRIMARY KEY,
    channel_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    username TEXT NOT NULL,
    peer_id TEXT NOT NULL,
    joined_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (channel_id) REFERENCES channels(id) ON DELETE CASCADE,
    UNIQUE(channel_id, user_id)
);

-- Table d'audit log
CREATE TABLE user_audit_log (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    action TEXT NOT NULL CHECK (action IN ('login', 'logout', 'register', 'password_change', 'profile_update', 'deactivated', 'reactivated', 'role_changed', 'server_created', 'member_added', 'member_removed', 'server_updated')),
    performed_by TEXT,
    details TEXT,
    ip_address TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

-- Index pour optimiser les requêtes
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_user_number ON users(user_number) WHERE user_number IS NOT NULL;
CREATE INDEX idx_users_deleted_at ON users(deleted_at);
CREATE INDEX idx_users_is_active ON users(is_active);
CREATE INDEX idx_users_role ON users(role);
CREATE UNIQUE INDEX idx_users_email_active ON users(email) WHERE deleted_at IS NULL;
CREATE UNIQUE INDEX idx_users_provider_active ON users(provider, provider_id) WHERE deleted_at IS NULL AND provider_id IS NOT NULL;

CREATE INDEX idx_licenses_user_id ON licenses(user_id);

CREATE INDEX idx_servers_owner_id ON servers(owner_id);
CREATE INDEX idx_servers_is_online ON servers(is_online);
CREATE INDEX idx_servers_last_heartbeat ON servers(last_heartbeat);

CREATE INDEX idx_server_members_server_id ON server_members(server_id);
CREATE INDEX idx_server_members_user_id ON server_members(user_id);
CREATE INDEX idx_server_members_role ON server_members(role);
CREATE INDEX idx_server_members_is_online ON server_members(is_online);

CREATE INDEX idx_categories_server_id ON categories(server_id);
CREATE INDEX idx_categories_position ON categories(position);

CREATE INDEX idx_channels_server_id ON channels(server_id);
CREATE INDEX idx_channels_category_id ON channels(category_id);
CREATE INDEX idx_channels_position ON channels(position);
CREATE INDEX idx_channels_type ON channels(type);

CREATE INDEX idx_category_permissions_category_id ON category_permissions(category_id);
CREATE INDEX idx_channel_permissions_channel_id ON channel_permissions(channel_id);

CREATE INDEX idx_voice_sessions_channel_id ON voice_sessions(channel_id);
CREATE INDEX idx_voice_sessions_user_id ON voice_sessions(user_id);

CREATE INDEX idx_audit_user_id ON user_audit_log(user_id);
CREATE INDEX idx_audit_action ON user_audit_log(action);
CREATE INDEX idx_audit_created_at ON user_audit_log(created_at);




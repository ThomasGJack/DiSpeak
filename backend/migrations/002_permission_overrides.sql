-- Migration pour les overrides de permissions
-- Ajout des tables pour les overrides de permissions par canal et catégorie

-- Table des overrides de permissions par canal et rôle
CREATE TABLE IF NOT EXISTS channel_role_overrides (
    id TEXT PRIMARY KEY,
    channel_id TEXT NOT NULL,
    role_id TEXT NOT NULL,
    allow_bitfield INTEGER NOT NULL DEFAULT 0,
    deny_bitfield INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY (channel_id) REFERENCES channels(id) ON DELETE CASCADE,
    FOREIGN KEY (role_id) REFERENCES server_roles(id) ON DELETE CASCADE,
    UNIQUE(channel_id, role_id)
);

-- Table des overrides de permissions par catégorie et rôle
CREATE TABLE IF NOT EXISTS category_role_overrides (
    id TEXT PRIMARY KEY,
    category_id TEXT NOT NULL,
    role_id TEXT NOT NULL,
    allow_bitfield INTEGER NOT NULL DEFAULT 0,
    deny_bitfield INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE CASCADE,
    FOREIGN KEY (role_id) REFERENCES server_roles(id) ON DELETE CASCADE,
    UNIQUE(category_id, role_id)
);

-- Table des overrides de permissions par canal et membre
CREATE TABLE IF NOT EXISTS member_channel_overrides (
    id TEXT PRIMARY KEY,
    channel_id TEXT NOT NULL,
    member_id TEXT NOT NULL,
    allow_bitfield INTEGER NOT NULL DEFAULT 0,
    deny_bitfield INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY (channel_id) REFERENCES channels(id) ON DELETE CASCADE,
    FOREIGN KEY (member_id) REFERENCES server_members(id) ON DELETE CASCADE,
    UNIQUE(channel_id, member_id)
);

-- Table des overrides de permissions par catégorie et membre
CREATE TABLE IF NOT EXISTS member_category_overrides (
    id TEXT PRIMARY KEY,
    category_id TEXT NOT NULL,
    member_id TEXT NOT NULL,
    allow_bitfield INTEGER NOT NULL DEFAULT 0,
    deny_bitfield INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE CASCADE,
    FOREIGN KEY (member_id) REFERENCES server_members(id) ON DELETE CASCADE,
    UNIQUE(category_id, member_id)
);

-- Index pour optimiser les requêtes
CREATE INDEX IF NOT EXISTS idx_channel_role_overrides_channel_id ON channel_role_overrides(channel_id);
CREATE INDEX IF NOT EXISTS idx_channel_role_overrides_role_id ON channel_role_overrides(role_id);
CREATE INDEX IF NOT EXISTS idx_category_role_overrides_category_id ON category_role_overrides(category_id);
CREATE INDEX IF NOT EXISTS idx_category_role_overrides_role_id ON category_role_overrides(role_id);
CREATE INDEX IF NOT EXISTS idx_member_channel_overrides_channel_id ON member_channel_overrides(channel_id);
CREATE INDEX IF NOT EXISTS idx_member_channel_overrides_member_id ON member_channel_overrides(member_id);
CREATE INDEX IF NOT EXISTS idx_member_category_overrides_category_id ON member_category_overrides(category_id);
CREATE INDEX IF NOT EXISTS idx_member_category_overrides_member_id ON member_category_overrides(member_id);
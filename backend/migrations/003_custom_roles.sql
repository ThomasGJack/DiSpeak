-- Migration pour le système de rôles personnalisés
-- Création des tables pour les rôles de serveur et l'assignation multiple

-- Table des rôles personnalisés par serveur
CREATE TABLE IF NOT EXISTS server_roles (
    id TEXT PRIMARY KEY,
    server_id TEXT NOT NULL,
    name TEXT NOT NULL,
    color TEXT NOT NULL DEFAULT '#5865F2',
    position INTEGER NOT NULL DEFAULT 0,
    permissions_bitfield INTEGER NOT NULL DEFAULT 0,
    is_default BOOLEAN NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY (server_id) REFERENCES servers(id) ON DELETE CASCADE
);

-- Table de liaison pour les rôles multiples par membre
CREATE TABLE IF NOT EXISTS member_roles (
    id TEXT PRIMARY KEY,
    member_id TEXT NOT NULL,
    role_id TEXT NOT NULL,
    assigned_at TEXT NOT NULL,
    FOREIGN KEY (member_id) REFERENCES server_members(id) ON DELETE CASCADE,
    FOREIGN KEY (role_id) REFERENCES server_roles(id) ON DELETE CASCADE,
    UNIQUE(member_id, role_id)
);

-- Index pour optimiser les requêtes
CREATE INDEX IF NOT EXISTS idx_server_roles_server_id ON server_roles(server_id);
CREATE INDEX IF NOT EXISTS idx_server_roles_position ON server_roles(server_id, position);
CREATE INDEX IF NOT EXISTS idx_member_roles_member_id ON member_roles(member_id);
CREATE INDEX IF NOT EXISTS idx_member_roles_role_id ON member_roles(role_id);

-- Créer les rôles par défaut pour chaque serveur existant
INSERT INTO server_roles (id, server_id, name, color, position, permissions_bitfield, is_default, created_at)
SELECT 
    'role-owner-' || s.id as id,
    s.id as server_id,
    'Owner' as name,
    '#FF6B6B' as color,
    1000 as position,
    2147483647 as permissions_bitfield, -- ADMINISTRATOR
    1 as is_default,
    datetime('now') as created_at
FROM servers s;

INSERT INTO server_roles (id, server_id, name, color, position, permissions_bitfield, is_default, created_at)
SELECT 
    'role-moderator-' || s.id as id,
    s.id as server_id,
    'Moderator' as name,
    '#4ECDC4' as color,
    500 as position,
    1023 as permissions_bitfield, -- Toutes permissions sauf ADMINISTRATOR
    1 as is_default,
    datetime('now') as created_at
FROM servers s;

INSERT INTO server_roles (id, server_id, name, color, position, permissions_bitfield, is_default, created_at)
SELECT 
    'role-user-' || s.id as id,
    s.id as server_id,
    'User' as name,
    '#95A5A6' as color,
    0 as position,
    65 as permissions_bitfield, -- VIEW_CHANNELS + CONNECT_VOICE
    1 as is_default,
    datetime('now') as created_at
FROM servers s;

-- Assigner les rôles par défaut aux membres existants
INSERT INTO member_roles (id, member_id, role_id, assigned_at)
SELECT 
    'mr-' || sm.id || '-' || sr.id as id,
    sm.id as member_id,
    sr.id as role_id,
    datetime('now') as assigned_at
FROM server_members sm
JOIN server_roles sr ON sr.server_id = sm.server_id AND sr.is_default = 1
WHERE (sm.role = 'owner' AND sr.name = 'Owner')
   OR (sm.role = 'moderator' AND sr.name = 'Moderator')
   OR (sm.role = 'user' AND sr.name = 'User');

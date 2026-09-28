-- Clés propriétaires d'hébergement de serveurs (1 clé active par utilisateur)
CREATE TABLE IF NOT EXISTS user_owner_keys (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL UNIQUE,
    kid TEXT NOT NULL UNIQUE,
    secret_hash TEXT NOT NULL,
    encrypted_secret TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    rotated_at TEXT,
    last_used_at TEXT,
    last_used_ip TEXT,
    last_used_server_id TEXT,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_user_owner_keys_user_id ON user_owner_keys(user_id);
CREATE INDEX IF NOT EXISTS idx_user_owner_keys_kid ON user_owner_keys(kid);
CREATE INDEX IF NOT EXISTS idx_user_owner_keys_active ON user_owner_keys(is_active);

-- Sessions runtime pour les binaires server (token opaque hashé)
CREATE TABLE IF NOT EXISTS server_runtime_sessions (
    id TEXT PRIMARY KEY,
    server_id TEXT NOT NULL,
    owner_user_id TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_seen_at TEXT,
    last_seen_ip TEXT,
    instance_name TEXT,
    server_version TEXT,
    platform TEXT,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    FOREIGN KEY (server_id) REFERENCES servers(id) ON DELETE CASCADE,
    FOREIGN KEY (owner_user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_server_runtime_sessions_server_id ON server_runtime_sessions(server_id);
CREATE INDEX IF NOT EXISTS idx_server_runtime_sessions_owner_user_id ON server_runtime_sessions(owner_user_id);
CREATE INDEX IF NOT EXISTS idx_server_runtime_sessions_expires_at ON server_runtime_sessions(expires_at);
CREATE INDEX IF NOT EXISTS idx_server_runtime_sessions_active ON server_runtime_sessions(is_active);


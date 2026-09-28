-- Migration pour ajouter la table servers dans la base de données du serveur WebSocket
-- Cette table permet de vérifier la propriété des serveurs

CREATE TABLE IF NOT EXISTS servers (
    id TEXT PRIMARY KEY,
    owner_id TEXT NOT NULL,
    name TEXT NOT NULL,
    host TEXT NOT NULL,
    port INTEGER NOT NULL,
    max_users INTEGER NOT NULL DEFAULT 50,
    is_online BOOLEAN NOT NULL DEFAULT FALSE,
    last_heartbeat DATETIME,
    actual_host TEXT,
    actual_port INTEGER,
    icon TEXT,
    color TEXT DEFAULT '#7289DA',
    description TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Index pour optimiser les requêtes
CREATE INDEX IF NOT EXISTS idx_servers_owner_id ON servers(owner_id);
CREATE INDEX IF NOT EXISTS idx_servers_is_online ON servers(is_online);
CREATE INDEX IF NOT EXISTS idx_servers_last_heartbeat ON servers(last_heartbeat);




-- Migration pour améliorer le système VOIP avec SFU
-- Ajouter des colonnes pour le tracking des médias et la qualité audio

-- Ajouter des colonnes à la table voice_sessions existante
ALTER TABLE voice_sessions ADD COLUMN is_muted BOOLEAN DEFAULT 0;
ALTER TABLE voice_sessions ADD COLUMN is_deafened BOOLEAN DEFAULT 0;
ALTER TABLE voice_sessions ADD COLUMN is_video_enabled BOOLEAN DEFAULT 0;
ALTER TABLE voice_sessions ADD COLUMN audio_quality INTEGER DEFAULT 128; -- kbps
ALTER TABLE voice_sessions ADD COLUMN video_quality INTEGER DEFAULT 720; -- height
ALTER TABLE voice_sessions ADD COLUMN bandwidth_limit INTEGER DEFAULT 1000; -- kbps

-- Table pour tracker les tracks média WebRTC
CREATE TABLE IF NOT EXISTS media_tracks (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    track_id TEXT NOT NULL,
    kind TEXT NOT NULL, -- 'audio' ou 'video'
    enabled BOOLEAN NOT NULL DEFAULT 1,
    ssrc INTEGER,
    payload_type INTEGER,
    codec TEXT,
    bitrate INTEGER,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES voice_sessions(id) ON DELETE CASCADE
);

-- Table pour les connexions WebRTC entre participants
CREATE TABLE IF NOT EXISTS webrtc_connections (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    remote_session_id TEXT NOT NULL,
    connection_state TEXT NOT NULL DEFAULT 'new', -- new, connecting, connected, failed, disconnected
    ice_connection_state TEXT NOT NULL DEFAULT 'new',
    dtls_state TEXT NOT NULL DEFAULT 'new',
    bytes_sent INTEGER DEFAULT 0,
    bytes_received INTEGER DEFAULT 0,
    packets_sent INTEGER DEFAULT 0,
    packets_received INTEGER DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES voice_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (remote_session_id) REFERENCES voice_sessions(id) ON DELETE CASCADE,
    UNIQUE(session_id, remote_session_id)
);

-- Index pour optimiser les requêtes
CREATE INDEX IF NOT EXISTS idx_media_tracks_session_id ON media_tracks(session_id);
CREATE INDEX IF NOT EXISTS idx_media_tracks_track_id ON media_tracks(track_id);
CREATE INDEX IF NOT EXISTS idx_webrtc_connections_session_id ON webrtc_connections(session_id);
CREATE INDEX IF NOT EXISTS idx_webrtc_connections_remote_session_id ON webrtc_connections(remote_session_id);
CREATE INDEX IF NOT EXISTS idx_voice_sessions_channel_user ON voice_sessions(channel_id, user_id);




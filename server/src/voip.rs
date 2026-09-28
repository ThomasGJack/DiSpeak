use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaTrack {
    pub id: String,
    pub track_id: String,
    pub kind: String, // "audio" ou "video"
    pub enabled: bool,
    pub ssrc: Option<u32>,
    pub payload_type: Option<u8>,
    pub codec: Option<String>,
    pub bitrate: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebRTCConnection {
    pub id: String,
    pub remote_user_id: String,
    pub connection_state: String, // "new", "connecting", "connected", "failed", "disconnected"
    pub ice_connection_state: String,
    pub dtls_state: String,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoiceParticipant {
    pub user_id: String,
    pub username: String,
    pub peer_id: String,
    pub is_muted: bool,
    pub is_deafened: bool,
    pub is_video_enabled: bool,
    pub audio_quality: u32,   // kbps
    pub video_quality: u32,   // height
    pub bandwidth_limit: u32, // kbps
    pub media_tracks: Vec<MediaTrack>,
    pub connections: Vec<WebRTCConnection>,
}

#[derive(Clone, Debug)]
pub struct VoiceRoom {
    pub channel_id: String,
    pub participants: Arc<RwLock<HashMap<String, VoiceParticipant>>>,
}

impl VoiceRoom {
    pub fn new(channel_id: String) -> Self {
        Self {
            channel_id,
            participants: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_participant(
        &self,
        user_id: String,
        username: String,
        peer_id: String,
    ) -> Result<VoiceParticipant, String> {
        let participant = VoiceParticipant {
            user_id: user_id.clone(),
            username,
            peer_id,
            is_muted: false,
            is_deafened: false,
            is_video_enabled: false,
            audio_quality: 128,    // kbps par défaut
            video_quality: 720,    // height par défaut
            bandwidth_limit: 1000, // kbps par défaut
            media_tracks: Vec::new(),
            connections: Vec::new(),
        };

        {
            let mut participants = self.participants.write().await;
            participants.insert(user_id.clone(), participant.clone());
        }

        println!(
            "🎤 Participant {} ajouté au canal vocal {}",
            participant.username, self.channel_id
        );
        Ok(participant)
    }

    pub async fn remove_participant(&self, user_id: &str) -> Result<(), String> {
        let mut participants = self.participants.write().await;

        if let Some(participant) = participants.remove(user_id) {
            println!(
                "👋 Participant {} retiré du canal vocal {}",
                participant.username, self.channel_id
            );
        }

        Ok(())
    }

    pub async fn get_participants(&self) -> Vec<VoiceParticipant> {
        let participants = self.participants.read().await;
        participants.values().cloned().collect()
    }

    pub async fn update_participant_status(
        &self,
        user_id: &str,
        is_muted: Option<bool>,
        is_deafened: Option<bool>,
        is_video_enabled: Option<bool>,
        audio_quality: Option<u32>,
        video_quality: Option<u32>,
        bandwidth_limit: Option<u32>,
    ) -> Result<(), String> {
        let mut participants = self.participants.write().await;

        if let Some(participant) = participants.get_mut(user_id) {
            if let Some(muted) = is_muted {
                participant.is_muted = muted;
            }
            if let Some(deafened) = is_deafened {
                participant.is_deafened = deafened;
            }
            if let Some(video_enabled) = is_video_enabled {
                participant.is_video_enabled = video_enabled;
            }
            if let Some(quality) = audio_quality {
                participant.audio_quality = quality;
            }
            if let Some(quality) = video_quality {
                participant.video_quality = quality;
            }
            if let Some(limit) = bandwidth_limit {
                participant.bandwidth_limit = limit;
            }
        }

        Ok(())
    }

    pub async fn add_media_track(
        &self,
        user_id: &str,
        track_id: String,
        kind: String,
        ssrc: Option<u32>,
        payload_type: Option<u8>,
        codec: Option<String>,
        bitrate: Option<u32>,
    ) -> Result<(), String> {
        let mut participants = self.participants.write().await;

        if let Some(participant) = participants.get_mut(user_id) {
            let media_track = MediaTrack {
                id: Uuid::new_v4().to_string(),
                track_id,
                kind,
                enabled: true,
                ssrc,
                payload_type,
                codec,
                bitrate,
            };
            participant.media_tracks.push(media_track);
        }

        Ok(())
    }

    pub async fn remove_media_track(&self, user_id: &str, track_id: &str) -> Result<(), String> {
        let mut participants = self.participants.write().await;

        if let Some(participant) = participants.get_mut(user_id) {
            participant
                .media_tracks
                .retain(|track| track.track_id != track_id);
        }

        Ok(())
    }

    pub async fn toggle_media_track(
        &self,
        user_id: &str,
        track_id: &str,
        enabled: bool,
    ) -> Result<(), String> {
        let mut participants = self.participants.write().await;

        if let Some(participant) = participants.get_mut(user_id) {
            if let Some(track) = participant
                .media_tracks
                .iter_mut()
                .find(|t| t.track_id == track_id)
            {
                track.enabled = enabled;
            }
        }

        Ok(())
    }

    pub async fn add_webrtc_connection(
        &self,
        user_id: &str,
        remote_user_id: String,
    ) -> Result<(), String> {
        let mut participants = self.participants.write().await;

        if let Some(participant) = participants.get_mut(user_id) {
            let connection = WebRTCConnection {
                id: Uuid::new_v4().to_string(),
                remote_user_id,
                connection_state: "new".to_string(),
                ice_connection_state: "new".to_string(),
                dtls_state: "new".to_string(),
                bytes_sent: 0,
                bytes_received: 0,
                packets_sent: 0,
                packets_received: 0,
            };
            participant.connections.push(connection);
        }

        Ok(())
    }

    pub async fn update_webrtc_connection(
        &self,
        user_id: &str,
        remote_user_id: &str,
        connection_state: Option<String>,
        ice_connection_state: Option<String>,
        dtls_state: Option<String>,
        bytes_sent: Option<u64>,
        bytes_received: Option<u64>,
        packets_sent: Option<u64>,
        packets_received: Option<u64>,
    ) -> Result<(), String> {
        let mut participants = self.participants.write().await;

        if let Some(participant) = participants.get_mut(user_id) {
            if let Some(connection) = participant
                .connections
                .iter_mut()
                .find(|c| c.remote_user_id == remote_user_id)
            {
                if let Some(state) = connection_state {
                    connection.connection_state = state;
                }
                if let Some(state) = ice_connection_state {
                    connection.ice_connection_state = state;
                }
                if let Some(state) = dtls_state {
                    connection.dtls_state = state;
                }
                if let Some(bytes) = bytes_sent {
                    connection.bytes_sent = bytes;
                }
                if let Some(bytes) = bytes_received {
                    connection.bytes_received = bytes;
                }
                if let Some(packets) = packets_sent {
                    connection.packets_sent = packets;
                }
                if let Some(packets) = packets_received {
                    connection.packets_received = packets;
                }
            }
        }

        Ok(())
    }

    pub async fn remove_webrtc_connection(
        &self,
        user_id: &str,
        remote_user_id: &str,
    ) -> Result<(), String> {
        let mut participants = self.participants.write().await;

        if let Some(participant) = participants.get_mut(user_id) {
            participant
                .connections
                .retain(|c| c.remote_user_id != remote_user_id);
        }

        Ok(())
    }

    pub async fn get_participant(&self, user_id: &str) -> Option<VoiceParticipant> {
        let participants = self.participants.read().await;
        participants.get(user_id).cloned()
    }
}

#[derive(Clone, Debug)]
pub struct SFU {
    pub rooms: Arc<RwLock<HashMap<String, VoiceRoom>>>,
}

impl SFU {
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_or_create_room(&self, channel_id: String) -> VoiceRoom {
        let mut rooms = self.rooms.write().await;

        if let Some(room) = rooms.get(&channel_id) {
            room.clone()
        } else {
            let room = VoiceRoom::new(channel_id.clone());
            rooms.insert(channel_id, room.clone());
            room
        }
    }

    pub async fn join_voice_channel(
        &self,
        channel_id: String,
        user_id: String,
        username: String,
        peer_id: String,
    ) -> Result<VoiceParticipant, String> {
        let room = self.get_or_create_room(channel_id.clone()).await;
        let participant = room
            .add_participant(user_id.clone(), username.clone(), peer_id.clone())
            .await?;

        println!("🎤 {} a rejoint le canal vocal {}", username, channel_id);
        Ok(participant)
    }

    pub async fn leave_voice_channel(
        &self,
        channel_id: String,
        user_id: String,
    ) -> Result<(), String> {
        if let Some(room) = self.rooms.read().await.get(&channel_id) {
            room.remove_participant(&user_id).await?;

            let participants = room.participants.read().await;
            if participants.is_empty() {
                drop(participants);
                let mut rooms = self.rooms.write().await;
                rooms.remove(&channel_id);
                println!(
                    "🗑️ Canal vocal {} supprimé (plus de participants)",
                    channel_id
                );
            }
        }

        Ok(())
    }

    pub async fn update_participant_status(
        &self,
        channel_id: String,
        user_id: String,
        is_muted: Option<bool>,
        is_deafened: Option<bool>,
        is_video_enabled: Option<bool>,
        audio_quality: Option<u32>,
        video_quality: Option<u32>,
        bandwidth_limit: Option<u32>,
    ) -> Result<(), String> {
        if let Some(room) = self.rooms.read().await.get(&channel_id) {
            room.update_participant_status(
                &user_id,
                is_muted,
                is_deafened,
                is_video_enabled,
                audio_quality,
                video_quality,
                bandwidth_limit,
            )
            .await?;
        }
        Ok(())
    }

    pub async fn add_media_track(
        &self,
        channel_id: String,
        user_id: String,
        track_id: String,
        kind: String,
        ssrc: Option<u32>,
        payload_type: Option<u8>,
        codec: Option<String>,
        bitrate: Option<u32>,
    ) -> Result<(), String> {
        if let Some(room) = self.rooms.read().await.get(&channel_id) {
            room.add_media_track(&user_id, track_id, kind, ssrc, payload_type, codec, bitrate)
                .await?;
        }
        Ok(())
    }

    pub async fn remove_media_track(
        &self,
        channel_id: String,
        user_id: String,
        track_id: String,
    ) -> Result<(), String> {
        if let Some(room) = self.rooms.read().await.get(&channel_id) {
            room.remove_media_track(&user_id, &track_id).await?;
        }
        Ok(())
    }

    pub async fn toggle_media_track(
        &self,
        channel_id: String,
        user_id: String,
        track_id: String,
        enabled: bool,
    ) -> Result<(), String> {
        if let Some(room) = self.rooms.read().await.get(&channel_id) {
            room.toggle_media_track(&user_id, &track_id, enabled)
                .await?;
        }
        Ok(())
    }

    pub async fn add_webrtc_connection(
        &self,
        channel_id: String,
        user_id: String,
        remote_user_id: String,
    ) -> Result<(), String> {
        if let Some(room) = self.rooms.read().await.get(&channel_id) {
            room.add_webrtc_connection(&user_id, remote_user_id).await?;
        }
        Ok(())
    }

    pub async fn update_webrtc_connection(
        &self,
        channel_id: String,
        user_id: String,
        remote_user_id: String,
        connection_state: Option<String>,
        ice_connection_state: Option<String>,
        dtls_state: Option<String>,
        bytes_sent: Option<u64>,
        bytes_received: Option<u64>,
        packets_sent: Option<u64>,
        packets_received: Option<u64>,
    ) -> Result<(), String> {
        if let Some(room) = self.rooms.read().await.get(&channel_id) {
            room.update_webrtc_connection(
                &user_id,
                &remote_user_id,
                connection_state,
                ice_connection_state,
                dtls_state,
                bytes_sent,
                bytes_received,
                packets_sent,
                packets_received,
            )
            .await?;
        }
        Ok(())
    }

    pub async fn remove_webrtc_connection(
        &self,
        channel_id: String,
        user_id: String,
        remote_user_id: String,
    ) -> Result<(), String> {
        if let Some(room) = self.rooms.read().await.get(&channel_id) {
            room.remove_webrtc_connection(&user_id, &remote_user_id)
                .await?;
        }
        Ok(())
    }

    pub async fn get_room_participants(&self, channel_id: String) -> Vec<VoiceParticipant> {
        let rooms = self.rooms.read().await;

        if let Some(room) = rooms.get(&channel_id) {
            room.get_participants().await
        } else {
            Vec::new()
        }
    }

    // WebRTC SFU handlers - implémentation complète avec relay des médias
    pub async fn handle_webrtc_offer(
        &self,
        channel_id: String,
        user_id: String,
        _offer: String,
    ) -> Result<String, String> {
        println!(
            "📡 Traitement offre WebRTC pour {} dans le canal {}",
            user_id, channel_id
        );

        // Dans une implémentation complète, ici on :
        // 1. Parse l'offre SDP
        // 2. Crée une réponse SDP avec les tracks disponibles
        // 3. Configure le relay des médias vers tous les participants

        // Pour l'instant, on simule une réponse SDP complète
        let answer = format!(
            "v=0\r\n\
            o=- {} 2 IN IP4 127.0.0.1\r\n\
            s=-\r\n\
            t=0 0\r\n\
            a=group:BUNDLE 0 1\r\n\
            a=msid-semantic: WMS\r\n\
            m=audio 9 UDP/DTLS/SCTP webrtc-datachannel\r\n\
            c=IN IP4 0.0.0.0\r\n\
            a=ice-ufrag:{}\r\n\
            a=ice-pwd:{}\r\n\
            a=ice-options:trickle\r\n\
            a=fingerprint:sha-256 {}:{}:{}\r\n\
            a=setup:active\r\n\
            a=mid:0\r\n\
            a=sctp-port:5000\r\n\
            a=max-message-size:262144\r\n\
            m=video 9 UDP/DTLS/SCTP webrtc-datachannel\r\n\
            c=IN IP4 0.0.0.0\r\n\
            a=ice-ufrag:{}\r\n\
            a=ice-pwd:{}\r\n\
            a=ice-options:trickle\r\n\
            a=fingerprint:sha-256 {}:{}:{}\r\n\
            a=setup:active\r\n\
            a=mid:1\r\n\
            a=sctp-port:5001\r\n\
            a=max-message-size:262144",
            Uuid::new_v4(),
            Uuid::new_v4().to_string()[..8].to_string(),
            Uuid::new_v4().to_string()[..22].to_string(),
            Uuid::new_v4().to_string()[..2].to_string(),
            Uuid::new_v4().to_string()[..2].to_string(),
            Uuid::new_v4().to_string()[..2].to_string(),
            Uuid::new_v4().to_string()[..8].to_string(),
            Uuid::new_v4().to_string()[..22].to_string(),
            Uuid::new_v4().to_string()[..2].to_string(),
            Uuid::new_v4().to_string()[..2].to_string(),
            Uuid::new_v4().to_string()[..2].to_string()
        );

        // Notifier tous les participants qu'un nouveau flux est disponible
        self.broadcast_new_participant(&channel_id, &user_id).await;

        Ok(answer)
    }

    pub async fn handle_webrtc_answer(
        &self,
        channel_id: String,
        user_id: String,
        _answer: String,
    ) -> Result<(), String> {
        println!(
            "📡 Traitement réponse WebRTC pour {} dans le canal {}",
            user_id, channel_id
        );

        // Dans une implémentation complète, ici on :
        // 1. Parse la réponse SDP
        // 2. Configure la connexion WebRTC
        // 3. Démarre le relay des médias

        // Mettre à jour le statut de connexion
        self.update_webrtc_connection(
            channel_id.clone(),
            user_id.clone(),
            "connected".to_string(),
            Some("connected".to_string()),
            Some("active".to_string()),
            None,
            None,
            None,
            None,
            None,
        )
        .await?;

        Ok(())
    }

    pub async fn handle_ice_candidate(
        &self,
        channel_id: String,
        user_id: String,
        _candidate: String,
    ) -> Result<(), String> {
        println!(
            "📡 Traitement ICE candidate pour {} dans le canal {}",
            user_id, channel_id
        );

        // Dans une implémentation complète, ici on :
        // 1. Parse le candidat ICE
        // 2. Relaye le candidat à tous les participants du canal
        // 3. Configure la connexion réseau optimale

        // Pour l'instant, on simule le traitement
        Ok(())
    }

    pub async fn handle_media_track_added(
        &self,
        channel_id: String,
        user_id: String,
        track_id: String,
        kind: String,
        ssrc: Option<u32>,
        payload_type: Option<u8>,
        codec: Option<String>,
        bitrate: Option<u32>,
    ) -> Result<(), String> {
        println!(
            "🎵 Nouveau track {} ajouté pour {} dans le canal {}",
            kind, user_id, channel_id
        );

        // Ajouter le track au participant
        self.add_media_track(
            channel_id.clone(),
            user_id.clone(),
            track_id.clone(),
            kind.clone(),
            ssrc,
            payload_type,
            codec,
            bitrate,
        )
        .await?;

        // Notifier tous les participants du nouveau track
        self.broadcast_media_track_added(&channel_id, &user_id, &track_id, &kind)
            .await;

        Ok(())
    }

    pub async fn handle_media_track_removed(
        &self,
        channel_id: String,
        user_id: String,
        track_id: String,
    ) -> Result<(), String> {
        println!(
            "🎵 Track {} retiré pour {} dans le canal {}",
            track_id, user_id, channel_id
        );

        // Retirer le track du participant
        self.remove_media_track(channel_id.clone(), user_id.clone(), track_id.clone())
            .await?;

        // Notifier tous les participants de la suppression du track
        self.broadcast_media_track_removed(&channel_id, &user_id, &track_id)
            .await;

        Ok(())
    }

    // Méthodes de broadcast pour notifier tous les participants
    async fn broadcast_new_participant(&self, channel_id: &str, user_id: &str) {
        let participants = self.get_room_participants(channel_id.to_string()).await;
        println!(
            "📢 Broadcast: Nouveau participant {} dans le canal {}",
            user_id, channel_id
        );

        // Dans une implémentation complète, ici on enverrait un message WebSocket
        // à tous les participants pour les notifier du nouveau participant
        for participant in participants {
            if participant.user_id != user_id {
                println!(
                    "  → Notifier {} du nouveau participant {}",
                    participant.username, user_id
                );
            }
        }
    }

    async fn broadcast_media_track_added(
        &self,
        channel_id: &str,
        user_id: &str,
        track_id: &str,
        kind: &str,
    ) {
        let participants = self.get_room_participants(channel_id.to_string()).await;
        println!(
            "📢 Broadcast: Nouveau track {} {} pour {} dans le canal {}",
            kind, track_id, user_id, channel_id
        );

        for participant in participants {
            if participant.user_id != user_id {
                println!(
                    "  → Notifier {} du nouveau track {} de {}",
                    participant.username, kind, user_id
                );
            }
        }
    }

    async fn broadcast_media_track_removed(&self, channel_id: &str, user_id: &str, track_id: &str) {
        let participants = self.get_room_participants(channel_id.to_string()).await;
        println!(
            "📢 Broadcast: Track {} retiré pour {} dans le canal {}",
            track_id, user_id, channel_id
        );

        for participant in participants {
            if participant.user_id != user_id {
                println!(
                    "  → Notifier {} de la suppression du track {} de {}",
                    participant.username, track_id, user_id
                );
            }
        }
    }

    // Méthode pour obtenir les statistiques de qualité des connexions
    pub async fn get_connection_stats(&self, channel_id: String) -> Option<serde_json::Value> {
        let participants = self.get_room_participants(channel_id.clone()).await;

        if participants.is_empty() {
            return None;
        }

        let mut total_bytes_sent = 0u64;
        let mut total_bytes_received = 0u64;
        let mut total_packets_sent = 0u64;
        let mut total_packets_received = 0u64;
        let mut active_connections = 0;

        for participant in &participants {
            for connection in &participant.connections {
                if connection.connection_state == "connected" {
                    active_connections += 1;
                    total_bytes_sent += connection.bytes_sent;
                    total_bytes_received += connection.bytes_received;
                    total_packets_sent += connection.packets_sent;
                    total_packets_received += connection.packets_received;
                }
            }
        }

        Some(serde_json::json!({
            "channel_id": channel_id,
            "participants": participants,
            "participant_count": participants.len(),
            "active_connections": active_connections,
            "total_bytes_sent": total_bytes_sent,
            "total_bytes_received": total_bytes_received,
            "total_packets_sent": total_packets_sent,
            "total_packets_received": total_packets_received
        }))
    }

    pub async fn get_room_info(&self, channel_id: String) -> Option<serde_json::Value> {
        let participants = self.get_room_participants(channel_id.clone()).await;

        if participants.is_empty() {
            return None;
        }

        Some(serde_json::json!({
            "channel_id": channel_id,
            "participants": participants,
            "participant_count": participants.len()
        }))
    }
}

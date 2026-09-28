import { useState, useEffect, useRef, useCallback, useMemo } from 'react'
import { useAuth } from './useAuth'
import { WebSocketClient } from '../lib/websocket'

export interface WebSocketMessage {
  action: string
  data: any
}

export interface ChatMessage {
  id: string
  channel_id: string
  user_id: string
  username: string
  content: string
  created_at: string
  edited_at?: string | null
  reply_to_message_id?: string | null
  attachments?: string | null
  reactions?: string | null
  is_pinned?: boolean
  is_system?: boolean
}

export interface VoiceParticipant {
  user_id: string
  username: string
  peer_id: string
  is_muted: boolean
  is_deafened: boolean
}

export interface Channel {
  id: string
  server_id: string
  name: string
  type: 'text' | 'voice'
  description?: string | null
  category_id?: string | null
  position: number
  created_at: string
}

export interface Category {
  id: string
  server_id: string
  name: string
  position: number
  created_at: string
}

export interface CategoryWithChannels {
  category: Category
  channels: Channel[]
}

export interface ServerRole {
  id: string
  server_id: string
  name: string
  color: string
  position: number
  permissions_bitfield: number
  is_default: boolean
  created_at: string
}

export interface ServerMember {
  id: string
  server_id: string
  user_id: string
  username: string
  avatar?: string | null
  role: string
  joined_at: string
  last_seen: string
  is_online: boolean
}

export interface ServerInfo {
  id: string
  owner_id: string
  name: string
  host: string
  port: number
  max_users: number
  is_online: boolean
  last_heartbeat?: string | null
  actual_host?: string | null
  actual_port?: number | null
  icon?: string | null
  color?: string | null
  description?: string | null
  created_at: string
}

interface ServerStructurePayload {
  server?: ServerInfo
  categories?: CategoryWithChannels[]
  uncategorized_channels?: Channel[]
  members?: ServerMember[]
  roles?: ServerRole[]
}

interface PresenceSnapshotPayload {
  server_id: string
  seq: number
  members: ServerMember[]
}

interface PresenceDeltaPayload {
  server_id: string
  seq: number
  changes: Array<Partial<ServerMember> & { user_id: string; is_online: boolean }>
}

interface VoiceRoomSnapshotPayload {
  channel_id: string
  participants?: VoiceParticipant[]
}

const DEFAULT_WS_URL = (import.meta.env.VITE_WS_URL as string | undefined) ?? 'ws://localhost:8001'

export const useWebSocket = (serverId?: string) => {
  const { user } = useAuth()
  const [wsClient, setWsClient] = useState<WebSocketClient | null>(null)
  const [connected, setConnected] = useState(false)
  const clientRef = useRef<WebSocketClient | null>(null)
  const isConnectingRef = useRef(false)
  const currentChannelIdRef = useRef<string | null>(null)
  const activeVoiceChannelIdRef = useRef<string | null>(null)
  const presenceSeqRef = useRef<number>(0)

  const [server, setServer] = useState<ServerInfo | null>(null)
  const [categories, setCategories] = useState<Category[]>([])
  const [categoryGroups, setCategoryGroups] = useState<CategoryWithChannels[]>([])
  const [uncategorizedChannels, setUncategorizedChannels] = useState<Channel[]>([])
  const [channels, setChannels] = useState<Channel[]>([])
  const [roles, setRoles] = useState<ServerRole[]>([])
  const [serverMembers, setServerMembers] = useState<ServerMember[]>([])

  const [currentChannelId, setCurrentChannelId] = useState<string | null>(null)
  const [messages, setMessages] = useState<ChatMessage[]>([])

  const [voiceParticipants, setVoiceParticipants] = useState<VoiceParticipant[]>([])
  const [activeVoiceChannelId, setActiveVoiceChannelId] = useState<string | null>(null)

  const [error, setError] = useState<string | null>(null)
  const currentChannel = useMemo(() => {
    if (!currentChannelId) return null
    return channels.find(channel => channel.id === currentChannelId) ?? null
  }, [channels, currentChannelId])

  const requestServerStructure = useCallback(() => {
    const client = clientRef.current ?? wsClient
    if (!serverId || !user || !client || !client.isConnected) return

    client.send('get_server_structure', {
      server_id: serverId,
      user_id: user.id,
    })
  }, [serverId, user, wsClient])

  const handleStructurePayload = useCallback((payload: ServerStructurePayload) => {
    let nextChannelsFromPayload: Channel[] | null = null

    if (payload.server) {
      setServer(payload.server)
    }

    if (Array.isArray(payload.categories)) {
      setCategoryGroups(payload.categories)
      setCategories(payload.categories.map(group => group.category))

      const groupedChannels = payload.categories.flatMap(group => group.channels)
      nextChannelsFromPayload = groupedChannels
    }

    if (Array.isArray(payload.uncategorized_channels)) {
      setUncategorizedChannels(payload.uncategorized_channels)
      nextChannelsFromPayload = [
        ...(nextChannelsFromPayload ?? []),
        ...payload.uncategorized_channels,
      ]
    }

    if (nextChannelsFromPayload) {
      setChannels(nextChannelsFromPayload)
    }

    if (Array.isArray(payload.members)) {
      setServerMembers(payload.members)
    }

    if (Array.isArray(payload.roles)) {
      setRoles(payload.roles)
    }
  }, [])

  const connect = useCallback(async () => {
    if (!serverId || !user) return
    if (clientRef.current || isConnectingRef.current) return

    try {
      isConnectingRef.current = true
      const client = new WebSocketClient(DEFAULT_WS_URL)

      const emitStructureRequest = () => {
        if (!serverId || !user) return
        client.send('get_server_structure', {
          server_id: serverId,
          user_id: user.id,
        })
      }

      client.on('connected', (_action, data) => {
        console.log('[WebSocket] Connecté', data)
        setConnected(true)
        setError(null)

        client.send('join_server', {
          server_id: serverId,
          user_id: user.id,
          username: user.username,
        })

        if (currentChannelIdRef.current) {
          client.send('join_channel', {
            channel_id: currentChannelIdRef.current,
            user_id: user.id,
            username: user.username,
          })
        }

        if (activeVoiceChannelIdRef.current) {
          client.send('get_voice_room_info', {
            channel_id: activeVoiceChannelIdRef.current,
            user_id: user.id,
          })
        }
      })

      client.on('server_structure', (_action, data) => {
        console.log('[WebSocket] Structure reçue', data)
        handleStructurePayload(data as ServerStructurePayload)
      })

      client.on('channel_created', emitStructureRequest)
      client.on('channel_moved', emitStructureRequest)
      client.on('channels_reordered', emitStructureRequest)

      client.on('channel_updated', emitStructureRequest)

      client.on('channel_deleted', (_action, data) => {
        setChannels(prev => prev.filter(channel => channel.id !== data.channel_id))
        setUncategorizedChannels(prev => prev.filter(channel => channel.id !== data.channel_id))
        setCategoryGroups(prev =>
          prev.map(group => ({
            ...group,
            channels: group.channels.filter(channel => channel.id !== data.channel_id),
          }))
        )
        setCurrentChannelId(prev => {
          if (prev === data.channel_id) {
            setMessages([])
            return null
          }
          return prev
        })
      })

      client.on('category_created', emitStructureRequest)
      client.on('category_updated', emitStructureRequest)
      client.on('category_moved', emitStructureRequest)
      client.on('categories_reordered', emitStructureRequest)
      client.on('category_deleted', emitStructureRequest)

      client.on('message_history', (_action, data) => {
        if (data.channel_id === currentChannelIdRef.current) {
          setMessages(data.messages || [])
        }
      })

      client.on('new_message', (_action, data) => {
        const newMessage = data as ChatMessage
        if (newMessage.channel_id === currentChannelIdRef.current) {
          setMessages(prev => [...prev, newMessage])
        }
      })

      client.on('server_members', (_action, data) => {
        if (Array.isArray(data.members)) {
          setServerMembers(data.members)
        }
      })

      client.on('presence_snapshot', (_action, data) => {
        const snapshot = data as PresenceSnapshotPayload
        if (!Array.isArray(snapshot.members)) return
        presenceSeqRef.current = Number(snapshot.seq ?? 0)
        setServerMembers(snapshot.members)
      })

      client.on('presence_delta', (_action, data) => {
        const delta = data as PresenceDeltaPayload
        const seq = Number(delta.seq ?? 0)
        if (!Number.isFinite(seq)) return
        if (seq <= presenceSeqRef.current) return
        presenceSeqRef.current = seq

        const changes = Array.isArray(delta.changes) ? delta.changes : []
        if (changes.length === 0) return

        setServerMembers(prev => {
          const next = [...prev]
          for (const change of changes) {
            const index = next.findIndex(member => member.user_id === change.user_id)
            const now = new Date().toISOString()
            if (index >= 0) {
              next[index] = {
                ...next[index],
                ...change,
                last_seen: change.last_seen ?? now,
              }
            } else {
              next.push({
                id: `${delta.server_id}-${change.user_id}`,
                server_id: delta.server_id,
                user_id: change.user_id,
                username: change.username ?? 'Utilisateur',
                avatar: (change as any).avatar ?? null,
                role: (change as any).role ?? 'user',
                joined_at: now,
                last_seen: change.last_seen ?? now,
                is_online: Boolean(change.is_online),
              })
            }
          }

          return next.sort((a, b) => {
            const rank = (role: string) => (role === 'owner' ? 0 : role === 'admin' ? 1 : 2)
            const byRole = rank(a.role) - rank(b.role)
            if (byRole !== 0) return byRole
            if (a.is_online !== b.is_online) return a.is_online ? -1 : 1
            const byName = a.username.localeCompare(b.username, 'fr', { sensitivity: 'base' })
            if (byName !== 0) return byName
            return a.user_id.localeCompare(b.user_id)
          })
        })
      })

      client.on('member_joined', (_action, data) => {
        setServerMembers(prev => {
          const now = new Date().toISOString()
          const existing = prev.find(member => member.user_id === data.user_id)
          if (existing) {
            return prev.map(member =>
              member.user_id === data.user_id
                ? { ...member, is_online: true, last_seen: now }
                : member
            )
          }

          return [
            ...prev,
            {
              id: data.id || `${data.server_id}-${data.user_id}`,
              server_id: data.server_id,
              user_id: data.user_id,
              username: data.username ?? 'Utilisateur',
              avatar: data.avatar ?? null,
              role: data.role ?? 'user',
              joined_at: data.joined_at ?? now,
              last_seen: now,
              is_online: true,
            },
          ]
        })
      })

      client.on('member_left', (_action, data) => {
        setServerMembers(prev =>
          prev.map(member =>
            member.user_id === data.user_id
              ? { ...member, is_online: false, last_seen: new Date().toISOString() }
              : member
          )
        )
      })

      client.on('voice_participant_joined', (_action, data) => {
        const participant = data as VoiceParticipant
        setVoiceParticipants(prev => {
          if (prev.find(item => item.user_id === participant.user_id)) {
            return prev
          }
          return [...prev, participant]
        })
      })

      client.on('voice_participant_left', (_action, data) => {
        const userId = data.user_id as string
        setVoiceParticipants(prev => prev.filter(participant => participant.user_id !== userId))
      })

      client.on('voice_joined', (_action, data) => {
        if (data.channel_id) {
          setActiveVoiceChannelId(data.channel_id)
          client.send('get_voice_room_info', {
            channel_id: data.channel_id,
            user_id: user.id,
          })
        }
      })

      const applyVoiceSnapshot = (data: VoiceRoomSnapshotPayload) => {
        if (!data?.channel_id) return
        if (activeVoiceChannelIdRef.current && data.channel_id !== activeVoiceChannelIdRef.current) return
        if (Array.isArray(data.participants)) {
          setVoiceParticipants(data.participants)
        }
      }

      client.on('voice_room_info', (_action, data) => {
        applyVoiceSnapshot(data as VoiceRoomSnapshotPayload)
      })
      client.on('voice_room_snapshot', (_action, data) => {
        applyVoiceSnapshot(data as VoiceRoomSnapshotPayload)
      })

      client.on('voice_participant_updated', (_action, data) => {
        setVoiceParticipants(prev =>
          prev.map(participant =>
            participant.user_id === data.user_id
              ? {
                  ...participant,
                  is_muted: data.is_muted ?? participant.is_muted,
                  is_deafened: data.is_deafened ?? participant.is_deafened,
                }
              : participant
          )
        )
      })

      client.on('error', (_action, data) => {
        const details = [data.code, data.message].filter(Boolean).join(' - ')
        setError(details || 'Erreur temps réel')
      })

      client.on('connection_diagnostics', (_action, data) => {
        if (data?.kind === 'close' || data?.kind === 'reconnect_scheduled' || data?.kind === 'reconnect_failed') {
          setConnected(false)
        }
        if (data?.kind === 'error') {
          setError(data.message ?? 'Connexion WebSocket impossible')
        }
      })

      clientRef.current = client
      setWsClient(client)
      const authToken = localStorage.getItem('token') ?? undefined
      await client.connect(authToken)

      isConnectingRef.current = false
    } catch (connectionError) {
      console.error('[WebSocket] Erreur de connexion', connectionError)
      setConnected(false)
      setError(connectionError instanceof Error ? connectionError.message : 'Connexion impossible au serveur temps réel')
      isConnectingRef.current = false
    }
  }, [serverId, user, handleStructurePayload])

  const disconnect = useCallback(() => {
    const client = clientRef.current
    if (client) {
      if (serverId && user) {
        client.send('leave_server', {
          server_id: serverId,
          user_id: user.id,
          username: user.username,
        })
      }

      client.disconnect()
      setWsClient(null)
      setConnected(false)
      setVoiceParticipants([])
      setActiveVoiceChannelId(null)
      clientRef.current = null
      isConnectingRef.current = false
    }
  }, [serverId, user])

  useEffect(() => {
    currentChannelIdRef.current = currentChannelId
  }, [currentChannelId])

  useEffect(() => {
    activeVoiceChannelIdRef.current = activeVoiceChannelId
  }, [activeVoiceChannelId])

  useEffect(() => {
    presenceSeqRef.current = 0
  }, [serverId])

  useEffect(() => {
    if (serverId && user) {
      connect()
    }

    return () => {
      disconnect()
    }
  }, [serverId, user, connect, disconnect])

  const joinChannel = useCallback(
    (channelId: string) => {
      if (!wsClient || !user) return

      setCurrentChannelId(channelId)
      setMessages([])

      wsClient.send('join_channel', {
        channel_id: channelId,
        user_id: user.id,
        username: user.username,
      })
    },
    [wsClient, user]
  )

  const sendChannelMessage = useCallback(
    (content: string) => {
      if (!wsClient || !user || !currentChannelId || !content.trim()) return

      wsClient.send('send_message', {
        channel_id: currentChannelId,
        user_id: user.id,
        username: user.username,
        content,
      })
    },
    [wsClient, user, currentChannelId]
  )

  const createChannel = useCallback(
    async (payload: { name: string; type: 'text' | 'voice'; category_id?: string | null; description?: string | null }) => {
      if (!wsClient || !user || !serverId) throw new Error('WebSocket non connecté')

      await wsClient.request(
        'create_channel',
        {
          server_id: serverId,
          name: payload.name,
          channel_type: payload.type,
          category_id: payload.category_id ?? null,
          description: payload.description ?? null,
          user_id: user.id,
        },
        { expectedActions: ['channel_create_ack'] }
      )
    },
    [wsClient, user, serverId]
  )

  const createCategory = useCallback(
    async (name: string) => {
      if (!wsClient || !user || !serverId) throw new Error('WebSocket non connecté')

      await wsClient.request(
        'create_category',
        {
          server_id: serverId,
          name,
          user_id: user.id,
        },
        { expectedActions: ['category_create_ack'] }
      )
    },
    [wsClient, user, serverId]
  )

  const joinVoiceChannel = useCallback(
    (channelId: string) => {
      if (!wsClient || !user) return

      setActiveVoiceChannelId(channelId)
      wsClient.send('join_voice_channel', {
        channel_id: channelId,
        user_id: user.id,
        username: user.username,
      })
      wsClient.send('get_voice_room_info', {
        channel_id: channelId,
        user_id: user.id,
      })
    },
    [wsClient, user]
  )

  const leaveVoiceChannel = useCallback(
    (channelId?: string) => {
      if (!wsClient || !user) return

      const targetChannel = channelId ?? activeVoiceChannelId
      if (!targetChannel) return

      wsClient.send('leave_voice_channel', {
        channel_id: targetChannel,
        user_id: user.id,
      })
      setActiveVoiceChannelId(null)
      setVoiceParticipants([])
    },
    [wsClient, user, activeVoiceChannelId]
  )

  const setVoiceAudioEnabled = useCallback(
    (enabled: boolean) => {
      if (!wsClient || !user || !activeVoiceChannelId) return

      wsClient.send('toggle_audio', {
        channel_id: activeVoiceChannelId,
        user_id: user.id,
        enabled,
      })

      setVoiceParticipants(prev =>
        prev.map(participant =>
          participant.user_id === user.id
            ? { ...participant, is_muted: !enabled }
            : participant
        )
      )
    },
    [wsClient, user, activeVoiceChannelId]
  )

  const sendCommand = useCallback(
    async (message: { action: string; data?: any }): Promise<void> => {
      if (!wsClient) {
        throw new Error('WebSocket non connecté')
      }

      wsClient.send(message.action, message.data ?? {})
    },
    [wsClient]
  )

  const request = useCallback(
    async (message: {
      action: string
      data?: any
      timeoutMs?: number
      expectedActions?: string[]
    }): Promise<any> => {
      if (!wsClient) {
        throw new Error('WebSocket non connecté')
      }

      return wsClient.request(message.action, message.data ?? {}, {
        timeoutMs: message.timeoutMs,
        expectedActions: message.expectedActions,
      })
    },
    [wsClient]
  )

  // Compat helper for older hooks while the WS API is being unified.
  const sendMessage = useCallback(
    async (message: { action: string; data?: any }): Promise<unknown> => {
      if (message.action === 'get_effective_permissions') {
        return request({
          action: message.action,
          data: message.data,
          expectedActions: ['effective_permissions'],
        })
      }

      await sendCommand(message)
      return null
    },
    [request, sendCommand]
  )

  return {
    connected,
    error,
    setError,
    server,
    roles,
    categories,
    categoryGroups,
    uncategorizedChannels,
    channels,
    serverMembers,
    currentChannel,
    currentChannelId,
    messages,
    voiceParticipants,
    activeVoiceChannelId,
    joinChannel,
    sendChannelMessage,
    createChannel,
    createCategory,
    joinVoiceChannel,
    leaveVoiceChannel,
    setVoiceAudioEnabled,
    requestServerStructure,
    sendCommand,
    request,
    sendMessage,
  }
}

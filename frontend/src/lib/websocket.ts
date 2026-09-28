type MessageHandler = (action: string, data: any) => void

export interface WebSocketMessage {
  action: string
  data: any
}

interface PendingRequest {
  resolve: (data: any) => void
  reject: (error: Error) => void
  timeoutId: number
  expectedActions?: string[]
}

interface RequestOptions {
  timeoutMs?: number
  expectedActions?: string[]
}

export class WebSocketClient {
  private ws: WebSocket | null = null
  private url: string
  private token: string | null = null
  private reconnectAttempts = 0
  private maxReconnectAttempts = 5
  private reconnectDelay = 1000
  private handlers: Map<string, Set<MessageHandler>> = new Map()
  private isIntentionallyClosed = false
  private pendingRequests: Map<string, PendingRequest> = new Map()
  private reconnectTimerId: number | null = null

  constructor(url: string) {
    this.url = url
  }

  connect(token?: string): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        if (this.ws && (this.ws.readyState === WebSocket.OPEN || this.ws.readyState === WebSocket.CONNECTING)) {
          resolve()
          return
        }

        if (token) {
          this.token = token
        }

        const wsUrl = this.token ? `${this.url}?token=${this.token}` : this.url
        this.ws = new WebSocket(wsUrl)
        this.isIntentionallyClosed = false
        let settled = false
        let opened = false
        const safeResolve = () => {
          if (settled) return
          settled = true
          resolve()
        }
        const safeReject = (error: unknown) => {
          if (settled) return
          settled = true
          reject(error)
        }

        this.ws.onopen = () => {
          console.log('[WebSocket] Connecté')
          this.reconnectAttempts = 0
          opened = true
          this.emitLocal('connection_diagnostics', {
            kind: 'open',
            url: wsUrl,
          })
          safeResolve()
        }

        this.ws.onmessage = (event) => {
          try {
            const message: WebSocketMessage = JSON.parse(event.data)
            this.handleMessage(message)
          } catch (error) {
            console.error('[WebSocket] Erreur parsing message:', error)
          }
        }

        this.ws.onerror = (error) => {
          console.error('[WebSocket] Erreur:', error)
          this.emitLocal('connection_diagnostics', {
            kind: 'error',
            phase: opened ? 'runtime' : 'connect',
            message: opened
              ? 'Erreur WebSocket runtime'
              : 'Connexion WebSocket refusée ou impossible (serveur/token/URL)',
            url: wsUrl,
          })
          safeReject(error)
        }

        this.ws.onclose = (event) => {
          console.log('[WebSocket] Déconnecté', event.code, event.reason)
          this.emitLocal('connection_diagnostics', {
            kind: 'close',
            code: event.code,
            reason: event.reason || null,
            wasClean: event.wasClean,
            url: wsUrl,
          })
          this.rejectPendingRequests(new Error('WebSocket déconnecté'))
          if (!opened) {
            safeReject(new Error(`Connexion WebSocket refusée (${event.code})`))
          }
          if (!this.isIntentionallyClosed) {
            this.attemptReconnect()
          }
        }
      } catch (error) {
        reject(error)
      }
    })
  }

  disconnect() {
    this.isIntentionallyClosed = true
    if (this.reconnectTimerId !== null) {
      window.clearTimeout(this.reconnectTimerId)
      this.reconnectTimerId = null
    }
    this.rejectPendingRequests(new Error('WebSocket fermé'))
    if (this.ws) {
      this.ws.close()
      this.ws = null
    }
  }

  send(action: string, data: any = {}) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      const message: WebSocketMessage = { action, data }
      this.ws.send(JSON.stringify(message))
    } else {
      console.warn('[WebSocket] Non connecté, impossible d\'envoyer le message')
    }
  }

  request(action: string, data: any = {}, options: RequestOptions = {}): Promise<any> {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      return Promise.reject(new Error('WebSocket non connecté'))
    }

    const requestId = this.generateRequestId()
    const timeoutMs = options.timeoutMs ?? 5000

    return new Promise((resolve, reject) => {
      const timeoutId = window.setTimeout(() => {
        this.pendingRequests.delete(requestId)
        reject(new Error(`Timeout WebSocket pour l'action ${action}`))
      }, timeoutMs)

      this.pendingRequests.set(requestId, {
        resolve,
        reject,
        timeoutId,
        expectedActions: options.expectedActions,
      })

      const payload =
        data && typeof data === 'object' && !Array.isArray(data)
          ? { ...data, request_id: requestId }
          : { value: data, request_id: requestId }

      this.send(action, payload)
    })
  }

  on(action: string, handler: MessageHandler) {
    if (!this.handlers.has(action)) {
      this.handlers.set(action, new Set())
    }
    this.handlers.get(action)!.add(handler)
  }

  off(action: string, handler: MessageHandler) {
    const handlers = this.handlers.get(action)
    if (handlers) {
      handlers.delete(handler)
    }
  }

  private handleMessage(message: WebSocketMessage) {
    this.maybeResolvePendingRequest(message)

    const handlers = this.handlers.get(message.action)
    if (handlers) {
      handlers.forEach(handler => {
        try {
          handler(message.action, message.data)
        } catch (error) {
          console.error(`[WebSocket] Erreur handler ${message.action}:`, error)
        }
      })
    }
  }

  private emitLocal(action: string, data: any) {
    const handlers = this.handlers.get(action)
    if (!handlers) return
    handlers.forEach(handler => {
      try {
        handler(action, data)
      } catch (error) {
        console.error(`[WebSocket] Erreur handler ${action}:`, error)
      }
    })
  }

  private maybeResolvePendingRequest(message: WebSocketMessage) {
    const requestId =
      message?.data && typeof message.data === 'object'
        ? (message.data.request_id as string | undefined)
        : undefined

    if (!requestId) return

    const pending = this.pendingRequests.get(requestId)
    if (!pending) return

    if (
      pending.expectedActions &&
      pending.expectedActions.length > 0 &&
      !pending.expectedActions.includes(message.action) &&
      message.action !== 'error'
    ) {
      return
    }

    window.clearTimeout(pending.timeoutId)
    this.pendingRequests.delete(requestId)

    if (message.action === 'error') {
      const messageText =
        message?.data && typeof message.data === 'object'
          ? (message.data.message as string | undefined)
          : undefined
      pending.reject(new Error(messageText || 'Erreur WebSocket'))
      return
    }

    pending.resolve(message.data)
  }

  private rejectPendingRequests(error: Error) {
    for (const [requestId, pending] of this.pendingRequests.entries()) {
      window.clearTimeout(pending.timeoutId)
      pending.reject(error)
      this.pendingRequests.delete(requestId)
    }
  }

  private generateRequestId(): string {
    if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
      return crypto.randomUUID()
    }

    return `req_${Date.now()}_${Math.random().toString(36).slice(2)}`
  }

  private attemptReconnect() {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.error('[WebSocket] Nombre maximum de tentatives de reconnexion atteint')
      this.emitLocal('connection_diagnostics', {
        kind: 'reconnect_exhausted',
        attempts: this.reconnectAttempts,
      })
      return
    }

    this.reconnectAttempts++
    const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1)
    
    console.log(`[WebSocket] Tentative de reconnexion ${this.reconnectAttempts}/${this.maxReconnectAttempts} dans ${delay}ms`)
    
    this.emitLocal('connection_diagnostics', {
      kind: 'reconnect_scheduled',
      attempt: this.reconnectAttempts,
      maxAttempts: this.maxReconnectAttempts,
      delayMs: delay,
    })

    this.reconnectTimerId = window.setTimeout(() => {
      this.reconnectTimerId = null
      this.connect(this.token || undefined)
        .catch(error => {
          console.error('[WebSocket] Échec de reconnexion:', error)
          this.emitLocal('connection_diagnostics', {
            kind: 'reconnect_failed',
            attempt: this.reconnectAttempts,
            message: error instanceof Error ? error.message : 'Erreur inconnue',
          })
        })
    }, delay)
  }

  get isConnected(): boolean {
    return this.ws !== null && this.ws.readyState === WebSocket.OPEN
  }

  // ===== MÉTHODES DE GESTION DES CANAUX =====

  sendCreateChannel(data: {
    server_id: string;
    name: string;
    channel_type: 'text' | 'voice';
    category_id?: string | null;
    description?: string | null;
    user_id: string;
  }) {
    this.send('create_channel', data);
  }

  sendUpdateChannel(data: {
    channel_id: string;
    name?: string;
    description?: string;
    user_id: string;
  }) {
    this.send('update_channel', data);
  }

  sendDeleteChannel(data: {
    channel_id: string;
    user_id: string;
  }) {
    this.send('delete_channel', data);
  }

  sendMoveChannel(data: {
    channel_id: string;
    position: number;
    category_id?: string | null;
    user_id: string;
  }) {
    this.send('move_channel', data);
  }

  // ===== MÉTHODES DE GESTION DES CATÉGORIES =====

  sendCreateCategory(data: {
    server_id: string;
    name: string;
    user_id: string;
  }) {
    this.send('create_category', data);
  }

  sendUpdateCategory(data: {
    category_id: string;
    name?: string;
    user_id: string;
  }) {
    this.send('update_category', data);
  }

  sendDeleteCategory(data: {
    category_id: string;
    user_id: string;
  }) {
    this.send('delete_category', data);
  }

  sendMoveCategory(data: {
    category_id: string;
    position: number;
    user_id: string;
  }) {
    this.send('move_category', data);
  }

  // ===== MÉTHODES DE GESTION DES RÔLES =====

  sendCreateRole(data: {
    server_id: string;
    name: string;
    color?: string;
    permissions_bitfield?: number;
    position?: number;
    user_id: string;
  }) {
    this.send('create_role', data);
  }

  sendUpdateRole(data: {
    role_id: string;
    name?: string;
    color?: string;
    permissions_bitfield?: number;
    position?: number;
    user_id: string;
  }) {
    this.send('update_role', data);
  }

  sendDeleteRole(data: {
    role_id: string;
    user_id: string;
  }) {
    this.send('delete_role', data);
  }

  sendAssignRole(data: {
    server_id: string;
    member_id: string;
    role_id: string;
    user_id: string;
  }) {
    this.send('assign_role', data);
  }

  sendRemoveRole(data: {
    server_id: string;
    member_id: string;
    role_id: string;
    user_id: string;
  }) {
    this.send('remove_role', data);
  }

  // ===== MÉTHODES DE GESTION DES PERMISSIONS =====

  sendSetChannelPermissions(data: {
    channel_id: string;
    role_id: string;
    allow_bitfield: number;
    deny_bitfield: number;
    user_id: string;
  }) {
    this.send('set_channel_permissions', data);
  }

  sendSetCategoryPermissions(data: {
    category_id: string;
    role_id: string;
    allow_bitfield: number;
    deny_bitfield: number;
    user_id: string;
  }) {
    this.send('set_category_permissions', data);
  }

  sendSetMemberChannelPermissions(data: {
    channel_id: string;
    member_id: string;
    allow_bitfield: number;
    deny_bitfield: number;
    user_id: string;
  }) {
    this.send('set_member_channel_permissions', data);
  }

  sendSetMemberCategoryPermissions(data: {
    category_id: string;
    member_id: string;
    allow_bitfield: number;
    deny_bitfield: number;
    user_id: string;
  }) {
    this.send('set_member_category_permissions', data);
  }

  sendGetEffectivePermissions(data: {
    channel_id?: string;
    category_id?: string;
    user_id: string;
  }) {
    this.send('get_effective_permissions', data);
  }

  // ===== MÉTHODES DE GESTION DES MEMBRES =====

  sendAddMember(data: {
    server_id: string;
    user_id: string;
    inviter_id: string;
  }) {
    this.send('add_member', data);
  }

  sendRemoveMember(data: {
    server_id: string;
    member_id: string;
    user_id: string;
  }) {
    this.send('remove_member', data);
  }

  sendUpdateMember(data: {
    server_id: string;
    member_id: string;
    nickname?: string;
    user_id: string;
  }) {
    this.send('update_member', data);
  }

  // ===== MÉTHODES DE GESTION DU SERVEUR =====

  sendUpdateServerSettings(data: {
    server_id: string;
    name?: string;
    description?: string;
    icon?: string;
    color?: string;
    max_users?: number;
    user_id: string;
  }) {
    this.send('update_server_settings', data);
  }

  sendGetServerStructure(data: {
    server_id: string;
    user_id: string;
  }) {
    this.send('get_server_structure', data);
  }

  // ===== MÉTHODES DE GESTION DES MESSAGES =====

  sendCreateMessage(data: {
    channel_id: string;
    content: string;
    user_id: string;
  }) {
    this.send('create_message', data);
  }

  sendUpdateMessage(data: {
    message_id: string;
    content: string;
    user_id: string;
  }) {
    this.send('update_message', data);
  }

  sendDeleteMessage(data: {
    message_id: string;
    user_id: string;
  }) {
    this.send('delete_message', data);
  }

  // ===== MÉTHODES DE GESTION VOCALE =====

  sendJoinVoiceChannel(data: {
    channel_id: string;
    user_id: string;
  }) {
    this.send('join_voice_channel', data);
  }

  sendLeaveVoiceChannel(data: {
    user_id: string;
  }) {
    this.send('leave_voice_channel', data);
  }

  sendMuteMember(data: {
    server_id: string;
    member_id: string;
    muted: boolean;
    user_id: string;
  }) {
    this.send('mute_member', data);
  }

  sendDeafenMember(data: {
    server_id: string;
    member_id: string;
    deafened: boolean;
    user_id: string;
  }) {
    this.send('deafen_member', data);
  }

  sendMoveMember(data: {
    server_id: string;
    member_id: string;
    channel_id: string;
    user_id: string;
  }) {
    this.send('move_member', data);
  }

  // ===== MÉTHODES UTILITAIRES =====

  sendPing() {
    this.send('ping', { timestamp: Date.now() });
  }

  sendPong(data: { timestamp: number }) {
    this.send('pong', data);
  }

  sendHeartbeat() {
    this.send('heartbeat', { timestamp: Date.now() });
  }
}


import React, { useEffect, useRef, useState } from 'react'
import { colors, inputStyle, primaryButtonStyle } from '../styles/commonStyles'

export interface Message {
  id: string
  channel_id: string
  user_id: string
  username?: string
  content: string
  created_at: string
}

interface ChatAreaProps {
  channelName: string
  messages: Message[]
  onSendMessage: (content: string) => void
  currentUserId: string | null
}

export function ChatArea({ channelName, messages, onSendMessage, currentUserId }: ChatAreaProps) {
  const [inputValue, setInputValue] = useState('')
  const messagesEndRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (messagesEndRef.current) {
      messagesEndRef.current.scrollIntoView({ behavior: 'smooth' })
    }
  }, [messages])

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (inputValue.trim()) {
      onSendMessage(inputValue.trim())
      setInputValue('')
    }
  }

  const formatTime = (dateString: string) => {
    try {
      const date = new Date(dateString)
      return date.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })
    } catch {
      return ''
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      {/* Header */}
      <div style={{
        padding: '16px 24px',
        borderBottom: `1px solid ${colors.cardBorder}`,
        background: 'rgba(26, 26, 46, 0.4)',
        backdropFilter: 'blur(10px)'
      }}>
        <h2 style={{
          margin: 0,
          fontSize: 18,
          fontWeight: 600,
          color: colors.textPrimary,
          display: 'flex',
          alignItems: 'center',
          gap: 8
        }}>
          <span style={{ opacity: 0.6 }}>#</span>
          {channelName}
        </h2>
      </div>

      {/* Messages */}
      <div style={{
        flex: 1,
        overflowY: 'auto',
        padding: 20,
        display: 'flex',
        flexDirection: 'column',
        gap: 16
      }}>
        {messages.length === 0 ? (
          <div style={{
            flex: 1,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            flexDirection: 'column',
            gap: 12
          }}>
            <div style={{ fontSize: 48 }}>💬</div>
            <p style={{ color: colors.textSecondary, textAlign: 'center', margin: 0 }}>
              Aucun message dans ce canal.<br />Soyez le premier à écrire !
            </p>
          </div>
        ) : (
          messages.map((message) => {
            const isOwn = message.user_id === currentUserId
            return (
              <div 
                key={message.id} 
                style={{
                  display: 'flex',
                  flexDirection: 'column',
                  gap: 4,
                  alignSelf: isOwn ? 'flex-end' : 'flex-start',
                  maxWidth: '70%'
                }}
              >
                <div style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8,
                  fontSize: 12
                }}>
                  <span style={{
                    fontWeight: 600,
                    color: isOwn ? colors.primaryDark : colors.primary
                  }}>
                    {message.username || 'Utilisateur inconnu'}
                  </span>
                  <span style={{ color: colors.textTertiary, fontSize: 11 }}>
                    {formatTime(message.created_at)}
                  </span>
                </div>
                <div style={{
                  padding: '10px 14px',
                  borderRadius: 12,
                  background: isOwn 
                    ? `linear-gradient(135deg, ${colors.primary} 0%, ${colors.primaryDark} 100%)`
                    : 'rgba(255, 255, 255, 0.05)',
                  color: colors.textPrimary,
                  wordWrap: 'break-word',
                  lineHeight: 1.5,
                  fontSize: 14
                }}>
                  {message.content}
                </div>
              </div>
            )
          })
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Input */}
      <div style={{
        padding: 16,
        borderTop: `1px solid ${colors.cardBorder}`,
        background: 'rgba(26, 26, 46, 0.4)',
        backdropFilter: 'blur(10px)'
      }}>
        <form style={{ display: 'flex', gap: 8 }} onSubmit={handleSubmit}>
          <input
            type="text"
            style={{ ...inputStyle, flex: 1 }}
            placeholder={`Message dans #${channelName}`}
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            onFocus={(e) => {
              e.target.style.borderColor = colors.primary
              e.target.style.background = 'rgba(15, 15, 15, 0.8)'
            }}
            onBlur={(e) => {
              e.target.style.borderColor = colors.cardBorder
              e.target.style.background = colors.inputBg
            }}
          />
          <button 
            type="submit" 
            style={primaryButtonStyle(false)}
            onMouseEnter={(e) => e.currentTarget.style.transform = 'scale(1.02)'}
            onMouseLeave={(e) => e.currentTarget.style.transform = 'scale(1)'}
          >
            Envoyer
          </button>
        </form>
      </div>
    </div>
  )
}

import * as Avatar from '@radix-ui/react-avatar'
import { colors } from '../styles/commonStyles'

export interface Member {
  id: string
  user_number?: number
  username: string
  avatar?: string | null
  status: 'online' | 'away' | 'busy' | 'offline'
  role: 'owner' | 'moderator' | 'user'
  lastSeen?: string
  inVoice?: boolean
}

interface MemberListProps {
  members: Member[]
}

const roleLabels: Record<Member['role'], string> = {
  owner: 'Propri�taire',
  moderator: 'Mod�rateur',
  user: 'Membre',
}

export function MemberList({ members }: MemberListProps) {
  const statusColors = {
    online: '#10b981',
    away: '#f59e0b',
    busy: '#ef4444',
    offline: '#6b7280',
  }

  const formatLastSeen = (value?: string) => {
    if (!value) return null
    try {
      const date = new Date(value)
      return date.toLocaleString('fr-FR', {
        day: '2-digit',
        month: '2-digit',
        hour: '2-digit',
        minute: '2-digit',
      })
    } catch {
      return null
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 2, padding: 12 }}>
      <div
        style={{
          padding: '8px 0',
          fontSize: 11,
          textTransform: 'uppercase',
          color: colors.textTertiary,
          fontWeight: 700,
          letterSpacing: '0.05em',
          marginBottom: 4,
        }}
      >
        Membres � {members.length}
      </div>

      {members.map((member) => {
        const initials = member.username
          .split(' ')
          .map((n) => n[0])
          .join('')
          .toUpperCase()
          .slice(0, 2)

        const lastSeenLabel = member.status === 'offline' ? formatLastSeen(member.lastSeen) : null

        return (
          <div
            key={member.id}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 10,
              padding: '8px 10px',
              borderRadius: 8,
              cursor: 'pointer',
              transition: 'background 0.2s',
            }}
            onMouseEnter={(e) => (e.currentTarget.style.background = 'rgba(255, 255, 255, 0.05)')}
            onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
          >
            <Avatar.Root
              style={{
                width: 32,
                height: 32,
                borderRadius: '50%',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
                color: '#fff',
                fontWeight: 600,
                fontSize: 13,
                position: 'relative',
              }}
            >
              {member.avatar ? (
                <Avatar.Image
                  src={member.avatar}
                  alt={member.username}
                  style={{
                    width: '100%',
                    height: '100%',
                    borderRadius: '50%',
                    objectFit: 'cover',
                  }}
                />
              ) : (
                <Avatar.Fallback delayMs={0}>{initials}</Avatar.Fallback>
              )}
              <div
                style={{
                  position: 'absolute',
                  bottom: -2,
                  right: -2,
                  width: 10,
                  height: 10,
                  borderRadius: '50%',
                  border: '2px solid rgba(15, 15, 15, 0.6)',
                  background: statusColors[member.status],
                }}
              />
            </Avatar.Root>

            <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 2 }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                <span
                  style={{
                    fontSize: 14,
                    color: colors.textPrimary,
                    fontWeight: 500,
                  }}
                >
                  {member.username}
                </span>
                <span
                  style={{
                    fontSize: 10,
                    color: colors.textTertiary,
                    textTransform: 'uppercase',
                    letterSpacing: '0.08em',
                  }}
                >
                  {roleLabels[member.role]}
                </span>
              </div>
              {lastSeenLabel && (
                <span
                  style={{
                    fontSize: 10,
                    color: colors.textTertiary,
                  }}
                >
                  Vu {lastSeenLabel}
                </span>
              )}
            </div>

            {member.inVoice && <span style={{ fontSize: 16 }}>???</span>}
          </div>
        )
      })}

      {members.length === 0 && (
        <div
          style={{
            padding: 12,
            color: colors.textTertiary,
            fontSize: 13,
            textAlign: 'center',
          }}
        >
          Aucun membre connect�
        </div>
      )}
    </div>
  )
}

import { colors } from '../styles/commonStyles'

export interface Channel {
  id: string
  server_id: string
  name: string
  type: 'text' | 'voice'
  created_at: string
  category_id?: string | null
  description?: string | null
  position?: number
}

export interface ChannelCategory {
  id: string
  name: string
  position: number
}

interface ChannelListProps {
  categories: Array<{ category: ChannelCategory; channels: Channel[] }>
  uncategorizedChannels: Channel[]
  activeChannelId: string | null
  onChannelSelect: (channelId: string) => void
}

export function ChannelList({
  categories,
  uncategorizedChannels,
  activeChannelId,
  onChannelSelect,
}: ChannelListProps) {
  const renderSectionTitle = (label: string) => (
    <div
      style={{
        padding: '8px 14px',
        fontSize: 11,
        textTransform: 'uppercase' as const,
        color: colors.textTertiary,
        fontWeight: 700,
        letterSpacing: '0.05em',
        marginTop: 8,
      }}
    >
      {label}
    </div>
  )

  const channelItemStyle = (active: boolean) => ({
    padding: '10px 14px',
    borderRadius: 8,
    background: active
      ? `linear-gradient(135deg, ${colors.primary} 0%, ${colors.primaryDark} 100%)`
      : 'transparent',
    color: active ? colors.textPrimary : colors.textSecondary,
    cursor: 'pointer',
    display: 'flex',
    alignItems: 'center',
    gap: 10,
    fontSize: 14,
    border: 'none',
    width: '100%',
    textAlign: 'left' as const,
    transition: 'all 0.2s ease',
    fontWeight: active ? 600 : 500,
  })

  const renderChannel = (channel: Channel) => {
    const isActive = channel.id === activeChannelId
    const icon = channel.type === 'voice' ? '🎧' : '#'

    return (
      <button
        key={channel.id}
        style={channelItemStyle(isActive)}
        onClick={() => onChannelSelect(channel.id)}
        onMouseEnter={(e) => {
          if (!isActive) {
            e.currentTarget.style.background = 'rgba(255, 255, 255, 0.05)'
          }
        }}
        onMouseLeave={(e) => {
          if (!isActive) {
            e.currentTarget.style.background = 'transparent'
          }
        }}
      >
        <span style={{ fontSize: 16, opacity: channel.type === 'voice' ? 1 : 0.7 }}>
          {icon}
        </span>
        <span>{channel.name}</span>
      </button>
    )
  }

  const hasCategories = categories.length > 0
  const hasUncategorized = uncategorizedChannels.length > 0

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 4, padding: 12 }}>
      {hasCategories &&
        categories
          .sort((a, b) => a.category.position - b.category.position)
          .map(({ category, channels }) => (
            <div key={category.id} style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
              {renderSectionTitle(category.name)}
              {channels
                .slice()
                .sort((a, b) => (a.position ?? 0) - (b.position ?? 0))
                .map(renderChannel)}
            </div>
          ))}

      {hasUncategorized && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
          {renderSectionTitle('Sans catégorie')}
          {uncategorizedChannels
            .slice()
            .sort((a, b) => (a.position ?? 0) - (b.position ?? 0))
            .map(renderChannel)}
        </div>
      )}

      {!hasCategories && !hasUncategorized && (
        <div
          style={{
            padding: 12,
            color: colors.textTertiary,
            fontSize: 13,
            textAlign: 'center',
          }}
        >
          Aucun canal disponible
        </div>
      )}
    </div>
  )
}

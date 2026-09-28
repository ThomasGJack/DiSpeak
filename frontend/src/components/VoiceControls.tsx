import { colors, primaryButtonStyle, secondaryButtonStyle } from '../styles/commonStyles'

interface VoiceControlsProps {
  channelName: string
  connected: boolean
  onJoin: () => void
  onLeave: () => void
  onToggleMute: () => void
  onToggleDeafen: () => void
  muted: boolean
  deafened: boolean
}

export function VoiceControls({
  channelName,
  connected,
  onJoin,
  onLeave,
  onToggleMute,
  onToggleDeafen,
  muted,
  deafened
}: VoiceControlsProps) {
  const controlButtonStyle = (active: boolean) => ({
    width: 64,
    height: 64,
    borderRadius: '50%',
    border: `2px solid ${active ? colors.primary : colors.cardBorder}`,
    background: active 
      ? `linear-gradient(135deg, ${colors.primary} 0%, ${colors.primaryDark} 100%)`
      : 'rgba(255, 255, 255, 0.05)',
    color: colors.textPrimary,
    fontSize: 28,
    cursor: 'pointer',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    transition: 'all 0.3s ease',
    boxShadow: active ? '0 4px 20px rgba(100, 108, 255, 0.4)' : 'none'
  })

  if (!connected) {
    return (
      <div style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        height: '100%',
        padding: 40,
        gap: 32
      }}>
        <div style={{ fontSize: 80 }}>🔊</div>
        <div style={{ textAlign: 'center' }}>
          <h2 style={{ 
            fontSize: 28, 
            fontWeight: 600, 
            margin: '0 0 8px',
            background: `linear-gradient(135deg, ${colors.primary} 0%, ${colors.primaryDark} 100%)`,
            WebkitBackgroundClip: 'text',
            WebkitTextFillColor: 'transparent'
          }}>
            Canal vocal : {channelName}
          </h2>
          <p style={{ color: colors.textSecondary, margin: 0, fontSize: 14 }}>
            Cliquez sur le bouton ci-dessous pour rejoindre le canal vocal
          </p>
        </div>
        <button 
          style={primaryButtonStyle(false)} 
          onClick={onJoin}
          onMouseEnter={(e) => e.currentTarget.style.transform = 'scale(1.05)'}
          onMouseLeave={(e) => e.currentTarget.style.transform = 'scale(1)'}
        >
          Rejoindre le canal vocal
        </button>
      </div>
    )
  }

  return (
    <div style={{
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      justifyContent: 'center',
      height: '100%',
      padding: 40,
      gap: 32
    }}>
      <div style={{ fontSize: 80 }}>🎤</div>
      
      <div style={{ textAlign: 'center' }}>
        <h2 style={{ 
          fontSize: 24, 
          fontWeight: 600, 
          margin: '0 0 8px',
          background: `linear-gradient(135deg, ${colors.primary} 0%, ${colors.primaryDark} 100%)`,
          WebkitBackgroundClip: 'text',
          WebkitTextFillColor: 'transparent'
        }}>
          Connecté à : {channelName}
        </h2>
        <p style={{ 
          color: colors.textSecondary, 
          margin: 0,
          fontSize: 14
        }}>
          {muted && 'Microphone coupé • '}
          {deafened && 'Son coupé • '}
          {!muted && !deafened && 'Microphone et son actifs'}
        </p>
      </div>

      <div style={{ display: 'flex', gap: 16, flexWrap: 'wrap', justifyContent: 'center' }}>
        <button
          style={controlButtonStyle(!muted)}
          onClick={onToggleMute}
          title={muted ? 'Activer le micro' : 'Couper le micro'}
          onMouseEnter={(e) => e.currentTarget.style.transform = 'scale(1.1)'}
          onMouseLeave={(e) => e.currentTarget.style.transform = 'scale(1)'}
        >
          {muted ? '🔇' : '🎤'}
        </button>

        <button
          style={controlButtonStyle(!deafened)}
          onClick={onToggleDeafen}
          title={deafened ? 'Activer le son' : 'Couper le son'}
          onMouseEnter={(e) => e.currentTarget.style.transform = 'scale(1.1)'}
          onMouseLeave={(e) => e.currentTarget.style.transform = 'scale(1)'}
        >
          {deafened ? '🔇' : '🔊'}
        </button>
      </div>

      <button 
        style={{ 
          ...secondaryButtonStyle, 
          background: 'rgba(239, 68, 68, 0.2)', 
          borderColor: 'rgba(239, 68, 68, 0.4)' 
        }} 
        onClick={onLeave}
        onMouseEnter={(e) => {
          e.currentTarget.style.background = 'rgba(239, 68, 68, 0.3)'
          e.currentTarget.style.transform = 'scale(1.02)'
        }}
        onMouseLeave={(e) => {
          e.currentTarget.style.background = 'rgba(239, 68, 68, 0.2)'
          e.currentTarget.style.transform = 'scale(1)'
        }}
      >
        Quitter le canal vocal
      </button>
    </div>
  )
}

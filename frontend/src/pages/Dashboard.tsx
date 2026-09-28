import { useEffect, useMemo, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import * as Avatar from '@radix-ui/react-avatar'
import { useAuth } from '../hooks/useAuth'
import { 
  mainContainerStyle, 
  backgroundPatternStyle,
  gradientTitleStyle,
  primaryButtonStyle,
  secondaryButtonStyle,
  sectionCardStyle,
  colors
} from '../styles/commonStyles'

export default function Dashboard() {
  const navigate = useNavigate()
  const [activeSection, setActiveSection] = useState<'overview' | 'profile' | 'settings' | 'servers'>('overview')
  const { user, logout } = useAuth()

  useEffect(() => {
    if (!user) {
      navigate('/')
    }
  }, [user, navigate])

  useEffect(() => {
    if (activeSection === 'profile') {
      navigate('/profile')
    }
  }, [activeSection, navigate])

  const userInitials = useMemo(() => {
    if (!user?.username) return 'UT'
    const parts = user.username.trim().split(' ').filter(Boolean)
    if (parts.length === 0) return 'UT'
    if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase()
    return (parts[0][0] + parts[1][0]).toUpperCase()
  }, [user?.username])

  const displayStatus = user?.status ?? 'offline'
  const statusLabel = {
    online: 'En ligne',
    away: 'Absent',
    busy: 'Occupé',
    offline: 'Hors ligne'
  }[displayStatus]

  const handleNavigateHome = () => {
    logout()
    navigate('/')
  }

  const navItems = [
    { id: 'overview' as const, label: 'Vue d\'ensemble', icon: '🏠' },
    { id: 'profile' as const, label: 'Profil', icon: '👤' },
    { id: 'settings' as const, label: 'Paramètres', icon: '⚙️' },
    { id: 'servers' as const, label: 'Serveurs', icon: '🌐' }
  ]

  const renderContent = () => {
    switch (activeSection) {
      case 'profile':
        return null
      case 'settings':
        return (
          <div>
            <div style={sectionCardStyle}>
              <h3 style={{ fontSize: 20, fontWeight: 600, marginBottom: 16, color: colors.textPrimary }}>
                Préférences de notification
              </h3>
              <p style={{ color: colors.textSecondary, lineHeight: 1.6, margin: 0 }}>
                Recevez des alertes pour les invitations, messages privés et mises à jour système.
              </p>
            </div>
            <div style={sectionCardStyle}>
              <h3 style={{ fontSize: 20, fontWeight: 600, marginBottom: 16, color: colors.textPrimary }}>
                Apparence
              </h3>
              <p style={{ color: colors.textSecondary, lineHeight: 1.6, margin: 0 }}>
                Thème actuel : sombre. Personnalisation avancée à venir.
              </p>
            </div>
          </div>
        )
      case 'servers':
        return (
          <div style={sectionCardStyle}>
            <h3 style={{ fontSize: 20, fontWeight: 600, marginBottom: 16, color: colors.textPrimary }}>
              Gestion des serveurs
            </h3>
            <p style={{ color: colors.textSecondary, lineHeight: 1.6, marginBottom: 24 }}>
              Utilisez le menu « Serveurs » pour rejoindre un espace existant ou parcourir vos accès.
            </p>
            <button 
              style={primaryButtonStyle(false)} 
              onClick={() => navigate('/servers')}
              onMouseEnter={(e) => e.currentTarget.style.transform = 'scale(1.02)'}
              onMouseLeave={(e) => e.currentTarget.style.transform = 'scale(1)'}
            >
              Voir mes serveurs
            </button>
          </div>
        )
      default:
        return (
          <div>
            <div style={sectionCardStyle}>
              <h3 style={{ fontSize: 20, fontWeight: 600, marginBottom: 16, color: colors.textPrimary }}>
                Bienvenue sur DiSpeak 👋
              </h3>
              <p style={{ color: colors.textSecondary, lineHeight: 1.6, marginBottom: 16 }}>
                Retrouvez ici un aperçu rapide de votre activité. Naviguez via le menu pour accéder à votre profil, vos réglages ou vos serveurs.
              </p>
              <p style={{ color: colors.textPrimary, lineHeight: 1.6, margin: 0 }}>
                Connecté en tant que <span style={{ color: colors.primary, fontWeight: 600 }}>{user?.username ?? 'Utilisateur'}</span>
              </p>
            </div>
            <div style={sectionCardStyle}>
              <h3 style={{ fontSize: 20, fontWeight: 600, marginBottom: 16, color: colors.textPrimary }}>
                Activité récente
              </h3>
              <p style={{ color: colors.textSecondary, lineHeight: 1.6, margin: 0 }}>
                Aucune activité récente à afficher.
              </p>
            </div>
          </div>
        )
    }
  }

  return (
    <div style={mainContainerStyle}>
      <div style={backgroundPatternStyle} />
      
      {/* Header */}
      <div style={{ 
        padding: '24px 32px', 
        background: 'rgba(26, 26, 46, 0.6)', 
        backdropFilter: 'blur(10px)',
        borderBottom: `1px solid ${colors.cardBorder}`,
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        position: 'relative',
        zIndex: 1
      }}>
        <h1 style={gradientTitleStyle}>Dashboard</h1>
        <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
          <Avatar.Root style={{
            width: 40,
            height: 40,
            borderRadius: '50%',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            background: `linear-gradient(135deg, ${colors.primary} 0%, ${colors.primaryDark} 100%)`,
            color: '#fff',
            fontWeight: 600,
            fontSize: 14
          }}>
            <Avatar.Fallback delayMs={0}>{userInitials}</Avatar.Fallback>
          </Avatar.Root>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
            <span style={{ fontWeight: 600, fontSize: 14 }}>{user?.username ?? 'Utilisateur'}</span>
            <span style={{ fontSize: 12, color: colors.textSecondary }}>{statusLabel}</span>
          </div>
          <button 
            style={{ ...secondaryButtonStyle, background: 'rgba(239, 68, 68, 0.2)', borderColor: 'rgba(239, 68, 68, 0.4)' }} 
            onClick={handleNavigateHome}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = 'rgba(239, 68, 68, 0.3)'
              e.currentTarget.style.transform = 'scale(1.02)'
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = 'rgba(239, 68, 68, 0.2)'
              e.currentTarget.style.transform = 'scale(1)'
            }}
          >
            Déconnexion
          </button>
        </div>
      </div>

      {/* Layout */}
      <div style={{ display: 'flex', minHeight: 'calc(100vh - 132px)', position: 'relative', zIndex: 1 }}>
        {/* Sidebar */}
        <aside style={{ 
          width: 280, 
          background: 'rgba(15, 15, 15, 0.5)',
          backdropFilter: 'blur(10px)',
          borderRight: `1px solid ${colors.cardBorder}`,
          padding: 24
        }}>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            {navItems.map((item) => (
              <button
                key={item.id}
                type="button"
                style={{
                  padding: '14px 18px',
                  borderRadius: 12,
                  border: activeSection === item.id ? `1px solid ${colors.primary}` : '1px solid transparent',
                  background: activeSection === item.id 
                    ? `linear-gradient(135deg, ${colors.primary} 0%, ${colors.primaryDark} 100%)`
                    : 'transparent',
                  color: colors.textPrimary,
                  textAlign: 'left',
                  cursor: 'pointer',
                  fontWeight: activeSection === item.id ? 600 : 500,
                  fontSize: 14,
                  transition: 'all 0.3s ease',
                  display: 'flex',
                  alignItems: 'center',
                  gap: 12
                }}
                onClick={() => setActiveSection(item.id)}
                onMouseEnter={(e) => {
                  if (activeSection !== item.id) {
                    e.currentTarget.style.background = 'rgba(255, 255, 255, 0.05)'
                  }
                }}
                onMouseLeave={(e) => {
                  if (activeSection !== item.id) {
                    e.currentTarget.style.background = 'transparent'
                  }
                }}
              >
                <span style={{ fontSize: 18 }}>{item.icon}</span>
                <span>{item.label}</span>
              </button>
            ))}
          </div>
        </aside>

        {/* Main content */}
        <main style={{ flex: 1, padding: 32, overflowY: 'auto' }}>
          {renderContent()}
        </main>
      </div>
    </div>
  )
}

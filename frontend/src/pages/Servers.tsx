import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuth } from '../hooks/useAuth'
import { api } from '../lib/api'
import { 
  mainContainerStyle, 
  backgroundPatternStyle,
  glassCardStyle,
  gradientTitleStyle,
  primaryButtonStyle,
  secondaryButtonStyle,
  sectionCardStyle,
  colors
} from '../styles/commonStyles'
import { getTopChromeOffset } from '../lib/runtime'

interface Server {
  id: string
  owner_id: string
  name: string
  host: string
  port: number
  max_users: number
  is_online: boolean
  last_heartbeat: string | null
  actual_host: string | null
  actual_port: number | null
  created_at: string
}

export default function Servers() {
  const navigate = useNavigate()
  const { user } = useAuth()
  const [myServers, setMyServers] = useState<Server[]>([])
  const [allServers, setAllServers] = useState<Server[]>([])
  const [loading, setLoading] = useState(true)
  const [actionPendingServerId, setActionPendingServerId] = useState<string | null>(null)
  const [pageError, setPageError] = useState<string | null>(null)
  const [copiedServerId, setCopiedServerId] = useState<string | null>(null)

  useEffect(() => {
    if (!user) {
      navigate('/')
      return
    }

    loadServers()

    // Auto-refresh toutes les 5 secondes pour voir les changements de statut rapidement
    const interval = setInterval(() => {
      loadServers()
    }, 5000)

    return () => clearInterval(interval)
  }, [user, navigate])

  const loadServers = async () => {
    try {
      setLoading(true)
      setPageError(null)
      const [my, all] = await Promise.all([
        api.get<Server[]>('/servers/my'),
        api.get<Server[]>('/servers')
      ])
      setMyServers(my)
      // Filtrer pour afficher uniquement les serveurs en ligne qui ne sont pas à nous
      setAllServers(all.filter(s => s.owner_id !== user?.id && s.is_online))
    } catch (error) {
      console.error('Erreur chargement serveurs:', error)
      setPageError('Impossible de charger la liste des serveurs pour le moment.')
    } finally {
      setLoading(false)
    }
  }

  const handleJoinServer = async (serverId: string) => {
    try {
      setActionPendingServerId(serverId)
      setPageError(null)
      const response = await api.post<{server_id: string, host: string, port: number}>(`/servers/${serverId}/join`, {})
      // Vérifier que le serveur a retourné des infos de connexion
      if (response.host && response.port) {
        navigate(`/server/${serverId}`)
      } else {
        setPageError('Serveur hors ligne ou inaccessible.')
      }
    } catch (error: any) {
      console.error('Erreur rejoindre serveur:', error)
      setPageError(error.message || 'Impossible de rejoindre le serveur')
    } finally {
      setActionPendingServerId(null)
    }
  }

  const handleOpenServer = async (serverId: string) => {
    // Vérifier que le serveur est en ligne avant d'ouvrir
    const server = myServers.find(s => s.id === serverId)
    if (server && !server.is_online) {
      setPageError('Le serveur est actuellement hors ligne. Veuillez le démarrer avant de vous connecter.')
      return
    }
    setPageError(null)
    navigate(`/server/${serverId}`)
  }

  const handleCopyServerId = async (serverId: string) => {
    try {
      await navigator.clipboard.writeText(serverId)
      setCopiedServerId(serverId)
      window.setTimeout(() => setCopiedServerId((current) => (current === serverId ? null : current)), 1500)
    } catch (err) {
      console.error('Erreur copie server_id', err)
      setPageError('Impossible de copier le server_id')
    }
  }

  if (loading) {
    return (
      <div style={mainContainerStyle}>
        <div style={backgroundPatternStyle} />
        <div style={{ 
          display: 'flex', 
          alignItems: 'center', 
          justifyContent: 'center',
          minHeight: `calc(100vh - ${getTopChromeOffset()}px)`,
          position: 'relative',
          zIndex: 1
        }}>
          <div style={{ ...glassCardStyle, padding: 40, textAlign: 'center' }}>
            <div style={{ fontSize: 40, marginBottom: 16 }}>⏳</div>
            <p style={{ color: colors.textSecondary, margin: 0 }}>Chargement des serveurs...</p>
          </div>
        </div>
      </div>
    )
  }

  const ServerCard = ({ server, isOwned }: { server: Server, isOwned: boolean }) => (
    <div style={{
      ...sectionCardStyle,
      marginBottom: 16,
      cursor: 'pointer',
      transition: 'all 0.3s ease',
      position: 'relative'
    }}
    onMouseEnter={(e) => {
      e.currentTarget.style.transform = 'translateY(-4px)'
      e.currentTarget.style.boxShadow = '0 12px 40px rgba(100, 108, 255, 0.2)'
    }}
    onMouseLeave={(e) => {
      e.currentTarget.style.transform = 'translateY(0)'
      e.currentTarget.style.boxShadow = '0 8px 32px rgba(0, 0, 0, 0.2)'
    }}
    >
      {/* Badge online/offline */}
      <div style={{
        position: 'absolute',
        top: 16,
        right: 16,
        padding: '6px 12px',
        borderRadius: 8,
        background: server.is_online 
          ? 'rgba(16, 185, 129, 0.2)' 
          : 'rgba(107, 114, 128, 0.2)',
        border: `1px solid ${server.is_online ? 'rgba(16, 185, 129, 0.4)' : 'rgba(107, 114, 128, 0.4)'}`,
        fontSize: 11,
        fontWeight: 600,
        textTransform: 'uppercase',
        color: server.is_online ? '#6ee7b7' : '#9ca3af',
        display: 'flex',
        alignItems: 'center',
        gap: 6
      }}>
        <div style={{
          width: 6,
          height: 6,
          borderRadius: '50%',
          background: server.is_online ? '#10b981' : '#6b7280'
        }} />
        {server.is_online ? 'En ligne' : 'Hors ligne'}
      </div>

      <div style={{ marginBottom: 16 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 12 }}>
          <div style={{
            width: 48,
            height: 48,
            borderRadius: 12,
            background: server.is_online
              ? `linear-gradient(135deg, ${colors.primary} 0%, ${colors.primaryDark} 100%)`
              : 'rgba(107, 114, 128, 0.3)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            fontSize: 24
          }}>
            🌐
          </div>
          <div style={{ flex: 1 }}>
            <h3 style={{ 
              fontSize: 18, 
              fontWeight: 600, 
              margin: 0,
              color: colors.textPrimary
            }}>
              {server.name}
            </h3>
            <p style={{ 
              fontSize: 13, 
              color: colors.textSecondary, 
              margin: '4px 0 0'
            }}>
              {server.is_online && server.actual_host && server.actual_port
                ? `${server.actual_host}:${server.actual_port}`
                : `${server.host}:${server.port}`
              }
            </p>
          </div>
        </div>
        
        <div style={{ 
          display: 'flex', 
          gap: 16,
          padding: '12px 0',
          borderTop: `1px solid ${colors.cardBorder}`,
          marginTop: 12
        }}>
          <div>
            <span style={{ color: colors.textSecondary, fontSize: 12 }}>Capacité</span>
            <p style={{ margin: '4px 0 0', fontWeight: 600, color: colors.primary }}>
              {server.max_users} utilisateurs
            </p>
          </div>
          {isOwned && (
            <div style={{ minWidth: 0 }}>
              <span style={{ color: colors.textSecondary, fontSize: 12 }}>Server ID</span>
              <p style={{
                margin: '4px 0 0',
                fontWeight: 600,
                color: colors.textPrimary,
                fontFamily: 'monospace',
                fontSize: 12,
                whiteSpace: 'nowrap',
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                maxWidth: 180
              }}>
                {server.id}
              </p>
            </div>
          )}
        </div>
      </div>

      {isOwned && (
        <div style={{
          marginBottom: 12,
          padding: 12,
          borderRadius: 10,
          border: '1px solid rgba(34, 211, 238, 0.18)',
          background: 'rgba(6, 182, 212, 0.06)'
        }}>
          <div style={{ color: colors.textSecondary, fontSize: 12, marginBottom: 8 }}>
            Liaison binaire serveur
          </div>
          <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
            <button
              style={secondaryButtonStyle}
              onClick={(e) => {
                e.stopPropagation()
                handleCopyServerId(server.id)
              }}
            >
              {copiedServerId === server.id ? 'Server ID copié' : 'Copier le server_id'}
            </button>
            <button
              style={secondaryButtonStyle}
              onClick={(e) => {
                e.stopPropagation()
                navigate('/profile')
              }}
            >
              Récupérer ma clé propriétaire
            </button>
            <button
              style={secondaryButtonStyle}
              onClick={(e) => {
                e.stopPropagation()
                navigate(`/servers/binary/new?serverId=${encodeURIComponent(server.id)}`)
              }}
            >
              Exporter server-config.toml
            </button>
          </div>
        </div>
      )}

      <button 
        style={isOwned ? primaryButtonStyle(!server.is_online || actionPendingServerId === server.id) : secondaryButtonStyle}
        onClick={(e) => {
          e.stopPropagation()
          if (server.is_online) {
            isOwned ? handleOpenServer(server.id) : handleJoinServer(server.id)
          }
        }}
        disabled={!server.is_online || actionPendingServerId === server.id}
        onMouseEnter={(e) => server.is_online && (e.currentTarget.style.transform = 'scale(1.02)')}
        onMouseLeave={(e) => (e.currentTarget.style.transform = 'scale(1)')}
      >
        {actionPendingServerId === server.id
          ? 'Connexion...'
          : server.is_online
            ? (isOwned ? 'Ouvrir' : 'Rejoindre')
            : 'Serveur hors ligne'}
      </button>
    </div>
  )

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
        <h1 style={gradientTitleStyle}>Serveurs DiSpeak</h1>
        <div style={{ display: 'flex', gap: 12 }}>
          <button
            style={primaryButtonStyle(false)}
            onClick={() => navigate('/servers/binary/new')}
            onMouseEnter={(e) => {
              e.currentTarget.style.transform = 'scale(1.02)'
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.transform = 'scale(1)'
            }}
          >
            + Créer un serveur binaire
          </button>
          <button 
            style={secondaryButtonStyle} 
            onClick={() => navigate('/dashboard')}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = 'rgba(255, 255, 255, 0.1)'
              e.currentTarget.style.transform = 'scale(1.02)'
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = 'rgba(255, 255, 255, 0.05)'
              e.currentTarget.style.transform = 'scale(1)'
            }}
          >
            ← Retour au Dashboard
          </button>
        </div>
      </div>

      {/* Content */}
      <main style={{ 
        padding: 32, 
        maxWidth: 1400, 
        margin: '0 auto',
        position: 'relative',
        zIndex: 1
      }}>
        {/* Mes serveurs */}
        <section style={{ marginBottom: 48 }}>
          {pageError && (
            <div style={{
              ...sectionCardStyle,
              marginBottom: 16,
              border: '1px solid rgba(239, 68, 68, 0.4)',
              background: 'rgba(127, 29, 29, 0.25)',
              color: '#fecaca',
              padding: 16
            }}>
              {pageError}
            </div>
          )}
          <div style={{ marginBottom: 24 }}>
            <h2 style={{ 
              fontSize: 24, 
              fontWeight: 600, 
              margin: '0 0 8px',
              color: colors.textPrimary
            }}>
              Mes serveurs
            </h2>
            <p style={{ color: colors.textSecondary, margin: 0, fontSize: 14 }}>
              Serveurs que vous possédez et gérez
            </p>
          </div>

          <div style={{
            ...sectionCardStyle,
            marginBottom: 16,
            border: '1px solid rgba(34, 211, 238, 0.22)',
            background: 'rgba(8, 20, 34, 0.55)'
          }}>
            <h3 style={{ margin: '0 0 10px', fontSize: 16, color: colors.textPrimary }}>
              Héberger un binaire serveur (nouveau workflow)
            </h3>
            <ol style={{ margin: 0, paddingLeft: 18, color: colors.textSecondary, lineHeight: 1.6 }}>
              <li>Créez (ou récupérez) votre serveur et copiez son <code style={{ color: colors.primary }}>server_id</code>.</li>
              <li>Ouvrez votre <strong>Profil</strong> pour copier votre <strong>clé propriétaire</strong>.</li>
              <li>Renseignez <code style={{ color: colors.primary }}>server-config.toml</code> du binaire <code style={{ color: colors.primary }}>dispeak-server</code>.</li>
              <li>Lancez le binaire: il validera la clé auprès du backend avant démarrage.</li>
            </ol>
            <div style={{ marginTop: 12 }}>
              <button
                style={secondaryButtonStyle}
                onClick={() => navigate('/servers/binary/new')}
              >
                Ouvrir l’écran “Créer un serveur binaire”
              </button>
            </div>
          </div>
          
          {myServers.length === 0 ? (
            <div style={{ 
              ...sectionCardStyle,
              textAlign: 'center',
              padding: 48
            }}>
              <div style={{ fontSize: 48, marginBottom: 16 }}>📭</div>
              <p style={{ color: colors.textSecondary, marginBottom: 8 }}>
                Vous n'avez pas encore de serveur.
              </p>
              <p style={{ color: colors.textTertiary, fontSize: 14, margin: 0 }}>
                Créez un serveur via l'application serveur dédiée pour commencer.
              </p>
            </div>
          ) : (
            <div style={{ 
              display: 'grid', 
              gridTemplateColumns: 'repeat(auto-fill, minmax(320px, 1fr))', 
              gap: 16 
            }}>
              {myServers.map((server) => (
                <ServerCard key={server.id} server={server} isOwned={true} />
              ))}
            </div>
          )}
        </section>

        {/* Serveurs disponibles */}
        <section>
          <div style={{ marginBottom: 24 }}>
            <h2 style={{ 
              fontSize: 24, 
              fontWeight: 600, 
              margin: '0 0 8px',
              color: colors.textPrimary
            }}>
              Serveurs disponibles
            </h2>
            <p style={{ color: colors.textSecondary, margin: 0, fontSize: 14 }}>
              Rejoignez des serveurs publics
            </p>
          </div>
          
          {allServers.length === 0 ? (
            <div style={{ 
              ...sectionCardStyle,
              textAlign: 'center',
              padding: 48
            }}>
              <div style={{ fontSize: 48, marginBottom: 16 }}>🔍</div>
              <p style={{ color: colors.textSecondary, margin: 0, marginBottom: 8 }}>
                Aucun serveur public en ligne pour le moment.
              </p>
              <p style={{ color: colors.textTertiary, fontSize: 14, margin: 0 }}>
                Seuls les serveurs actuellement démarrés sont affichés ici.
              </p>
            </div>
          ) : (
            <div style={{ 
              display: 'grid', 
              gridTemplateColumns: 'repeat(auto-fill, minmax(320px, 1fr))', 
              gap: 16 
            }}>
              {allServers.map((server) => (
                <ServerCard key={server.id} server={server} isOwned={false} />
              ))}
            </div>
          )}
        </section>
      </main>
    </div>
  )
}

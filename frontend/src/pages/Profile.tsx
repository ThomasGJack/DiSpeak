import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuth } from '../hooks/useAuth'
import { api } from '../lib/api'
import { 
  mainContainerStyle, 
  backgroundPatternStyle,
  sectionCardStyle,
  gradientTitleStyle,
  primaryButtonStyle,
  secondaryButtonStyle,
  inputStyle,
  colors
} from '../styles/commonStyles'

interface OwnerKeyInfo {
  key: string
  created_at: string
  rotated_at?: string | null
  last_used_at?: string | null
  last_used_server_id?: string | null
}

export default function Profile() {
  const navigate = useNavigate()
  const { user, refresh } = useAuth()
  const [editing, setEditing] = useState(false)
  const [username, setUsername] = useState('')
  const [bio, setBio] = useState('')
  const [avatar, setAvatar] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [success, setSuccess] = useState<string | null>(null)

  // Changement de mot de passe
  const [changingPassword, setChangingPassword] = useState(false)
  const [oldPassword, setOldPassword] = useState('')
  const [newPassword, setNewPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [passwordError, setPasswordError] = useState<string | null>(null)
  const [ownerKeyInfo, setOwnerKeyInfo] = useState<OwnerKeyInfo | null>(null)
  const [ownerKeyLoading, setOwnerKeyLoading] = useState(false)
  const [ownerKeyError, setOwnerKeyError] = useState<string | null>(null)
  const [ownerKeyActionLoading, setOwnerKeyActionLoading] = useState(false)

  useEffect(() => {
    if (!user) {
      navigate('/')
      return
    }

    setUsername(user.username)
    setBio(user.bio || '')
    setAvatar(user.avatar || '')
  }, [user, navigate])

  useEffect(() => {
    if (!user) return

    let cancelled = false
    const loadOwnerKey = async () => {
      try {
        setOwnerKeyLoading(true)
        setOwnerKeyError(null)
        const info = await api.get<OwnerKeyInfo>('/users/me/server-owner-key')
        if (!cancelled) {
          setOwnerKeyInfo(info)
        }
      } catch (err: any) {
        if (!cancelled) {
          setOwnerKeyError(err.message || 'Impossible de charger la clé propriétaire')
        }
      } finally {
        if (!cancelled) {
          setOwnerKeyLoading(false)
        }
      }
    }

    loadOwnerKey()
    return () => {
      cancelled = true
    }
  }, [user])

  const formatUserNumber = (num: number) => {
    return `#${num.toString().padStart(4, '0')}`
  }

  const handleSaveProfile = async () => {
    try {
      setLoading(true)
      setError(null)
      setSuccess(null)

      await api.patch('/users/me', {
        username: username !== user?.username ? username : undefined,
        bio: bio !== (user?.bio || '') ? bio : undefined,
        avatar: avatar !== (user?.avatar || '') ? avatar : undefined
      })

      await refresh()
      setSuccess('Profil mis à jour avec succès')
      setEditing(false)
    } catch (err: any) {
      setError(err.message || 'Erreur lors de la mise à jour')
    } finally {
      setLoading(false)
    }
  }

  const handleChangePassword = async (e: React.FormEvent) => {
    e.preventDefault()
    
    if (newPassword !== confirmPassword) {
      setPasswordError('Les mots de passe ne correspondent pas')
      return
    }

    if (newPassword.length < 8) {
      setPasswordError('Le mot de passe doit contenir au moins 8 caractères')
      return
    }

    const hasLetter = /[a-zA-Z]/.test(newPassword)
    const hasDigit = /[0-9]/.test(newPassword)
    
    if (!hasLetter || !hasDigit) {
      setPasswordError('Le mot de passe doit contenir au moins une lettre et un chiffre')
      return
    }

    try {
      setLoading(true)
      setPasswordError(null)

      await api.post('/auth/change-password', {
        old_password: oldPassword,
        new_password: newPassword
      })

      setSuccess('Mot de passe modifié avec succès')
      setOldPassword('')
      setNewPassword('')
      setConfirmPassword('')
      setChangingPassword(false)
    } catch (err: any) {
      setPasswordError(err.message || 'Erreur lors du changement de mot de passe')
    } finally {
      setLoading(false)
    }
  }

  const handleCopyOwnerKey = async () => {
    if (!ownerKeyInfo?.key) return
    try {
      await navigator.clipboard.writeText(ownerKeyInfo.key)
      setSuccess('Clé propriétaire copiée dans le presse-papiers')
      setError(null)
    } catch {
      setError('Impossible de copier la clé automatiquement')
    }
  }

  const handleRegenerateOwnerKey = async () => {
    const confirmed = window.confirm(
      "Régénérer la clé propriétaire invalidera l'ancienne. Les binaires serveur configurés avec l'ancienne clé devront être reconfigurés. Continuer ?"
    )
    if (!confirmed) return

    try {
      setOwnerKeyActionLoading(true)
      setOwnerKeyError(null)
      setError(null)
      const info = await api.post<OwnerKeyInfo>('/users/me/server-owner-key/regenerate', {})
      setOwnerKeyInfo(info)
      setSuccess('Nouvelle clé propriétaire générée. Mettez à jour la configuration de vos serveurs binaires.')
    } catch (err: any) {
      setOwnerKeyError(err.message || 'Impossible de régénérer la clé propriétaire')
    } finally {
      setOwnerKeyActionLoading(false)
    }
  }

  if (!user) return null

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
        <h1 style={gradientTitleStyle}>Mon Profil</h1>
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

      {/* Content */}
      <main style={{ 
        padding: 32, 
        maxWidth: 900,
        margin: '0 auto',
        position: 'relative',
        zIndex: 1
      }}>
        {/* Messages */}
        {error && (
          <div style={{
            padding: 16,
            background: 'rgba(239, 68, 68, 0.1)',
            border: '1px solid rgba(239, 68, 68, 0.3)',
            borderRadius: 12,
            color: '#fca5a5',
            marginBottom: 24
          }}>
            {error}
          </div>
        )}
        
        {success && (
          <div style={{
            padding: 16,
            background: 'rgba(16, 185, 129, 0.1)',
            border: '1px solid rgba(16, 185, 129, 0.3)',
            borderRadius: 12,
            color: '#6ee7b7',
            marginBottom: 24
          }}>
            {success}
          </div>
        )}

        {/* Informations générales */}
        <div style={sectionCardStyle}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 24 }}>
            <h2 style={{ fontSize: 22, fontWeight: 600, margin: 0 }}>Informations générales</h2>
            {!editing && (
              <button
                style={primaryButtonStyle(false)}
                onClick={() => setEditing(true)}
                onMouseEnter={(e) => e.currentTarget.style.transform = 'scale(1.02)'}
                onMouseLeave={(e) => e.currentTarget.style.transform = 'scale(1)'}
              >
                Modifier
              </button>
            )}
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
            {/* User Number (lecture seule) */}
            <div>
              <label style={{ color: colors.textSecondary, fontSize: 13, fontWeight: 600, display: 'block', marginBottom: 8 }}>
                Identifiant unique
              </label>
              <div style={{
                padding: '12px 16px',
                background: 'rgba(100, 108, 255, 0.1)',
                border: `1px solid ${colors.primary}`,
                borderRadius: 12,
                color: colors.primary,
                fontSize: 18,
                fontWeight: 700,
                fontFamily: 'monospace'
              }}>
                {formatUserNumber(user.user_number)}
              </div>
            </div>

            {/* Username */}
            <div>
              <label style={{ color: colors.textSecondary, fontSize: 13, fontWeight: 600, display: 'block', marginBottom: 8 }}>
                Nom d'utilisateur
              </label>
              {editing ? (
                <input
                  type="text"
                  value={username}
                  onChange={(e) => setUsername(e.target.value)}
                  style={inputStyle}
                  onFocus={(e) => {
                    e.target.style.borderColor = colors.primary
                    e.target.style.background = 'rgba(15, 15, 15, 0.8)'
                  }}
                  onBlur={(e) => {
                    e.target.style.borderColor = colors.cardBorder
                    e.target.style.background = colors.inputBg
                  }}
                />
              ) : (
                <div style={{ ...inputStyle, background: 'transparent', cursor: 'default' }}>
                  {user.username}
                </div>
              )}
            </div>

            {/* Email (lecture seule) */}
            <div>
              <label style={{ color: colors.textSecondary, fontSize: 13, fontWeight: 600, display: 'block', marginBottom: 8 }}>
                Email
              </label>
              <div style={{ ...inputStyle, background: 'transparent', cursor: 'not-allowed', opacity: 0.6 }}>
                {user.email}
              </div>
              <span style={{ color: colors.textTertiary, fontSize: 12, marginTop: 4, display: 'block' }}>
                L'email ne peut pas être modifié
              </span>
            </div>

            {/* Provider (lecture seule) */}
            <div>
              <label style={{ color: colors.textSecondary, fontSize: 13, fontWeight: 600, display: 'block', marginBottom: 8 }}>
                Type de compte
              </label>
              <div style={{ ...inputStyle, background: 'transparent', cursor: 'default' }}>
                {user.provider === 'local' ? 'Compte local' : `Compte ${user.provider}`}
              </div>
            </div>

            {/* Bio */}
            <div>
              <label style={{ color: colors.textSecondary, fontSize: 13, fontWeight: 600, display: 'block', marginBottom: 8 }}>
                Bio / Description
              </label>
              {editing ? (
                <textarea
                  value={bio}
                  onChange={(e) => setBio(e.target.value)}
                  style={{ ...inputStyle, minHeight: 100, resize: 'vertical', fontFamily: 'inherit' }}
                  maxLength={500}
                  placeholder="Parlez-nous de vous..."
                  onFocus={(e) => {
                    e.target.style.borderColor = colors.primary
                    e.target.style.background = 'rgba(15, 15, 15, 0.8)'
                  }}
                  onBlur={(e) => {
                    e.target.style.borderColor = colors.cardBorder
                    e.target.style.background = colors.inputBg
                  }}
                />
              ) : (
                <div style={{ ...inputStyle, background: 'transparent', minHeight: 60, cursor: 'default' }}>
                  {user.bio || 'Aucune bio renseignée'}
                </div>
              )}
              <span style={{ color: colors.textTertiary, fontSize: 12, marginTop: 4, display: 'block' }}>
                {bio.length}/500 caractères
              </span>
            </div>

            {editing && (
              <div style={{ display: 'flex', gap: 12, marginTop: 12 }}>
                <button
                  style={primaryButtonStyle(loading)}
                  disabled={loading}
                  onClick={handleSaveProfile}
                  onMouseEnter={(e) => !loading && (e.currentTarget.style.transform = 'scale(1.02)')}
                  onMouseLeave={(e) => (e.currentTarget.style.transform = 'scale(1)')}
                >
                  {loading ? 'Enregistrement...' : 'Enregistrer'}
                </button>
                <button
                  style={secondaryButtonStyle}
                  onClick={() => {
                    setEditing(false)
                    setUsername(user.username)
                    setBio(user.bio || '')
                    setAvatar(user.avatar || '')
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background = 'rgba(255, 255, 255, 0.1)'
                    e.currentTarget.style.transform = 'scale(1.02)'
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = 'rgba(255, 255, 255, 0.05)'
                    e.currentTarget.style.transform = 'scale(1)'
                  }}
                >
                  Annuler
                </button>
              </div>
            )}
          </div>
        </div>

        {/* Clé propriétaire serveur */}
        <div style={sectionCardStyle}>
          <h2 style={{ fontSize: 22, fontWeight: 600, margin: '0 0 16px' }}>Clé propriétaire serveur</h2>
          <p style={{ color: colors.textSecondary, marginTop: 0, marginBottom: 16 }}>
            Cette clé permet de lier un binaire <code style={{ color: colors.primary }}>dispeak-server</code> à votre compte.
            Ne la partagez jamais.
          </p>

          <div style={{
            padding: 14,
            borderRadius: 12,
            border: '1px solid rgba(245, 158, 11, 0.35)',
            background: 'rgba(120, 53, 15, 0.18)',
            color: '#fde68a',
            marginBottom: 16,
            fontSize: 13
          }}>
            ⚠️ Toute personne possédant cette clé peut tenter de rattacher un serveur binaire à votre compte.
          </div>

          {ownerKeyError && (
            <div style={{
              padding: 12,
              background: 'rgba(239, 68, 68, 0.1)',
              border: '1px solid rgba(239, 68, 68, 0.3)',
              borderRadius: 8,
              color: '#fca5a5',
              fontSize: 13,
              marginBottom: 16
            }}>
              {ownerKeyError}
            </div>
          )}

          {ownerKeyLoading ? (
            <div style={{ ...inputStyle, background: 'transparent', cursor: 'default' }}>
              Chargement de la clé propriétaire...
            </div>
          ) : (
            <>
              <div>
                <label style={{ color: colors.textSecondary, fontSize: 13, fontWeight: 600, display: 'block', marginBottom: 8 }}>
                  Clé propriétaire
                </label>
                <div style={{
                  ...inputStyle,
                  fontFamily: 'monospace',
                  fontSize: 13,
                  wordBreak: 'break-all',
                  lineHeight: 1.5,
                  background: 'rgba(6, 10, 22, 0.75)',
                  border: `1px solid ${colors.cardBorder}`
                }}>
                  {ownerKeyInfo?.key ?? 'Aucune clé disponible'}
                </div>
              </div>

              <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap', marginTop: 14 }}>
                <button
                  style={secondaryButtonStyle}
                  onClick={handleCopyOwnerKey}
                  disabled={!ownerKeyInfo?.key}
                >
                  Copier la clé
                </button>
                <button
                  style={primaryButtonStyle(ownerKeyActionLoading)}
                  onClick={handleRegenerateOwnerKey}
                  disabled={ownerKeyActionLoading}
                >
                  {ownerKeyActionLoading ? 'Régénération...' : 'Régénérer la clé'}
                </button>
              </div>

              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12, marginTop: 16 }}>
                <MetaRow label="Créée le" value={ownerKeyInfo?.created_at} />
                <MetaRow label="Dernière rotation" value={ownerKeyInfo?.rotated_at ?? 'Jamais'} />
                <MetaRow label="Dernière utilisation" value={ownerKeyInfo?.last_used_at ?? 'Jamais'} />
                <MetaRow label="Dernier serveur utilisé" value={ownerKeyInfo?.last_used_server_id ?? 'N/A'} mono />
              </div>

              <div style={{
                marginTop: 16,
                padding: 14,
                borderRadius: 10,
                border: `1px solid ${colors.cardBorder}`,
                background: 'rgba(255, 255, 255, 0.03)',
                color: colors.textSecondary,
                fontSize: 13
              }}>
                Workflow: créez un serveur dans l'application, récupérez son <code style={{ color: colors.primary }}>server_id</code>,
                copiez cette clé dans <code style={{ color: colors.primary }}>server-config.toml</code>, puis lancez le binaire serveur.
              </div>
            </>
          )}
        </div>

        {/* Sécurité */}
        <div style={sectionCardStyle}>
          <h2 style={{ fontSize: 22, fontWeight: 600, margin: '0 0 24px' }}>Sécurité</h2>

          {user.provider === 'local' ? (
            <>
              {!changingPassword ? (
                <div>
                  <p style={{ color: colors.textSecondary, marginBottom: 16 }}>
                    Changez régulièrement votre mot de passe pour sécuriser votre compte.
                  </p>
                  <button
                    style={secondaryButtonStyle}
                    onClick={() => setChangingPassword(true)}
                    onMouseEnter={(e) => {
                      e.currentTarget.style.background = 'rgba(255, 255, 255, 0.1)'
                      e.currentTarget.style.transform = 'scale(1.02)'
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.background = 'rgba(255, 255, 255, 0.05)'
                      e.currentTarget.style.transform = 'scale(1)'
                    }}
                  >
                    Changer le mot de passe
                  </button>
                </div>
              ) : (
                <form onSubmit={handleChangePassword} style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
                  {passwordError && (
                    <div style={{
                      padding: 12,
                      background: 'rgba(239, 68, 68, 0.1)',
                      border: '1px solid rgba(239, 68, 68, 0.3)',
                      borderRadius: 8,
                      color: '#fca5a5',
                      fontSize: 13
                    }}>
                      {passwordError}
                    </div>
                  )}

                  <div>
                    <label style={{ color: colors.textSecondary, fontSize: 13, fontWeight: 600, display: 'block', marginBottom: 8 }}>
                      Ancien mot de passe
                    </label>
                    <input
                      type="password"
                      value={oldPassword}
                      onChange={(e) => setOldPassword(e.target.value)}
                      style={inputStyle}
                      required
                      onFocus={(e) => {
                        e.target.style.borderColor = colors.primary
                        e.target.style.background = 'rgba(15, 15, 15, 0.8)'
                      }}
                      onBlur={(e) => {
                        e.target.style.borderColor = colors.cardBorder
                        e.target.style.background = colors.inputBg
                      }}
                    />
                  </div>

                  <div>
                    <label style={{ color: colors.textSecondary, fontSize: 13, fontWeight: 600, display: 'block', marginBottom: 8 }}>
                      Nouveau mot de passe
                    </label>
                    <input
                      type="password"
                      value={newPassword}
                      onChange={(e) => setNewPassword(e.target.value)}
                      style={inputStyle}
                      required
                      onFocus={(e) => {
                        e.target.style.borderColor = colors.primary
                        e.target.style.background = 'rgba(15, 15, 15, 0.8)'
                      }}
                      onBlur={(e) => {
                        e.target.style.borderColor = colors.cardBorder
                        e.target.style.background = colors.inputBg
                      }}
                    />
                  </div>

                  <div>
                    <label style={{ color: colors.textSecondary, fontSize: 13, fontWeight: 600, display: 'block', marginBottom: 8 }}>
                      Confirmer le nouveau mot de passe
                    </label>
                    <input
                      type="password"
                      value={confirmPassword}
                      onChange={(e) => setConfirmPassword(e.target.value)}
                      style={inputStyle}
                      required
                      onFocus={(e) => {
                        e.target.style.borderColor = colors.primary
                        e.target.style.background = 'rgba(15, 15, 15, 0.8)'
                      }}
                      onBlur={(e) => {
                        e.target.style.borderColor = colors.cardBorder
                        e.target.style.background = colors.inputBg
                      }}
                    />
                  </div>

                  <div style={{ display: 'flex', gap: 12 }}>
                    <button
                      type="submit"
                      style={primaryButtonStyle(loading)}
                      disabled={loading}
                      onMouseEnter={(e) => !loading && (e.currentTarget.style.transform = 'scale(1.02)')}
                      onMouseLeave={(e) => (e.currentTarget.style.transform = 'scale(1)')}
                    >
                      {loading ? 'Modification...' : 'Modifier le mot de passe'}
                    </button>
                    <button
                      type="button"
                      style={secondaryButtonStyle}
                      onClick={() => {
                        setChangingPassword(false)
                        setOldPassword('')
                        setNewPassword('')
                        setConfirmPassword('')
                        setPasswordError(null)
                      }}
                      onMouseEnter={(e) => {
                        e.currentTarget.style.background = 'rgba(255, 255, 255, 0.1)'
                        e.currentTarget.style.transform = 'scale(1.02)'
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.background = 'rgba(255, 255, 255, 0.05)'
                        e.currentTarget.style.transform = 'scale(1)'
                      }}
                    >
                      Annuler
                    </button>
                  </div>
                </form>
              )}
            </>
          ) : (
            <div style={{
              padding: 20,
              background: 'rgba(100, 108, 255, 0.05)',
              border: `1px solid ${colors.cardBorder}`,
              borderRadius: 12
            }}>
              <p style={{ color: colors.textSecondary, margin: 0 }}>
                Vous êtes connecté via <strong style={{ color: colors.primary }}>{user.provider}</strong>.
                <br />
                Le mot de passe est géré par votre fournisseur.
              </p>
            </div>
          )}
        </div>

        {/* Informations système */}
        <div style={sectionCardStyle}>
          <h2 style={{ fontSize: 22, fontWeight: 600, margin: '0 0 16px' }}>Informations système</h2>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', padding: '12px 0', borderBottom: `1px solid ${colors.cardBorder}` }}>
              <span style={{ color: colors.textSecondary }}>Rôle</span>
              <span style={{ fontWeight: 600, color: colors.primary }}>{user.role}</span>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', padding: '12px 0', borderBottom: `1px solid ${colors.cardBorder}` }}>
              <span style={{ color: colors.textSecondary }}>Statut du compte</span>
              <span style={{ fontWeight: 600, color: user.is_active ? colors.success : colors.error }}>
                {user.is_active ? 'Actif' : 'Désactivé'}
              </span>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', padding: '12px 0' }}>
              <span style={{ color: colors.textSecondary }}>Membre depuis</span>
              <span style={{ fontWeight: 600 }}>
                {new Date().toLocaleDateString('fr-FR')}
              </span>
            </div>
          </div>
        </div>
      </main>
    </div>
  )
}

function MetaRow({ label, value, mono = false }: { label: string; value?: string | null; mono?: boolean }) {
  return (
    <div style={{
      padding: 12,
      borderRadius: 10,
      border: '1px solid rgba(255,255,255,0.08)',
      background: 'rgba(255,255,255,0.02)'
    }}>
      <div style={{ color: colors.textTertiary, fontSize: 11, marginBottom: 6, textTransform: 'uppercase', letterSpacing: 0.4 }}>
        {label}
      </div>
      <div style={{
        color: colors.textPrimary,
        fontSize: 13,
        fontFamily: mono ? 'monospace' : 'inherit',
        wordBreak: mono ? 'break-all' : 'normal'
      }}>
        {value ?? 'N/A'}
      </div>
    </div>
  )
}

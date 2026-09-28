import React, { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuth } from '../hooks/useAuth'
import { IconBrandGoogleFilled } from '@tabler/icons-react'
import { getTopChromeOffset } from '../lib/runtime'

const topChromeOffset = getTopChromeOffset()

// Styles responsive
const containerStyle: React.CSSProperties = {
  minHeight: `calc(100vh - ${topChromeOffset}px)`,
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  background: 'linear-gradient(135deg, #0f0f0f 0%, #1a1a2e 100%)',
  color: '#fff',
  fontFamily: "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
  padding: '20px',
  marginTop: topChromeOffset,
  position: 'relative',
  overflow: 'hidden'
}

const backgroundPatternStyle: React.CSSProperties = {
  position: 'absolute',
  top: 0,
  left: 0,
  right: 0,
  bottom: 0,
  opacity: 0.03,
  backgroundImage: `radial-gradient(circle at 2px 2px, #646cff 1px, transparent 0)`,
  backgroundSize: '40px 40px',
  pointerEvents: 'none'
}

const cardContainerStyle: React.CSSProperties = {
  width: '100%',
  maxWidth: '440px',
  position: 'relative',
  zIndex: 1
}

const cardStyle: React.CSSProperties = {
  background: 'rgba(26, 26, 46, 0.8)',
  backdropFilter: 'blur(20px)',
  border: '1px solid rgba(100, 108, 255, 0.2)',
  borderRadius: 24,
  padding: 0,
  overflow: 'hidden',
  boxShadow: '0 20px 60px rgba(0, 0, 0, 0.3), 0 0 80px rgba(100, 108, 255, 0.1)',
  transition: 'transform 0.3s ease, box-shadow 0.3s ease'
}

const cardHeaderStyle: React.CSSProperties = {
  padding: '48px 40px 32px',
  textAlign: 'center',
  background: 'linear-gradient(180deg, rgba(100, 108, 255, 0.1) 0%, transparent 100%)',
  borderBottom: '1px solid rgba(100, 108, 255, 0.1)'
}

const logoStyle: React.CSSProperties = {
  fontSize: 48,
  marginBottom: 12
}

const titleStyle: React.CSSProperties = {
  fontSize: 32,
  fontWeight: 700,
  margin: 0,
  background: 'linear-gradient(135deg, #646cff 0%, #8b5cf6 100%)',
  WebkitBackgroundClip: 'text',
  WebkitTextFillColor: 'transparent',
  letterSpacing: '-0.02em'
}

const subtitleStyle: React.CSSProperties = {
  color: 'rgba(255, 255, 255, 0.6)',
  marginTop: 8,
  fontSize: 15,
  fontWeight: 400
}

const cardBodyStyle: React.CSSProperties = {
  padding: '32px 40px 40px'
}

const tabsContainerStyle: React.CSSProperties = {
  display: 'flex',
  gap: 8,
  marginBottom: 32,
  background: 'rgba(15, 15, 15, 0.5)',
  padding: 4,
  borderRadius: 12
}

const tabButton = (active: boolean): React.CSSProperties => ({
  flex: 1,
  padding: '12px 20px',
  background: active 
    ? 'linear-gradient(135deg, #646cff 0%, #8b5cf6 100%)' 
    : 'transparent',
  color: active ? '#fff' : 'rgba(255, 255, 255, 0.6)',
  border: 'none',
  borderRadius: 8,
  cursor: 'pointer',
  fontSize: 14,
  fontWeight: 600,
  transition: 'all 0.3s ease',
  transform: active ? 'scale(1)' : 'scale(0.98)'
})

const formStyle: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: 20
}

const inputGroupStyle: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: 8
}

const labelStyle: React.CSSProperties = {
  color: 'rgba(255, 255, 255, 0.8)',
  fontSize: 13,
  fontWeight: 600,
  letterSpacing: '0.02em',
  textTransform: 'uppercase'
}

const inputStyle: React.CSSProperties = {
  padding: '14px 16px',
  borderRadius: 12,
  border: '1px solid rgba(100, 108, 255, 0.2)',
  background: 'rgba(15, 15, 15, 0.5)',
  color: '#fff',
  fontSize: 15,
  transition: 'all 0.3s ease',
  outline: 'none'
}

const submitButtonStyle = (disabled: boolean): React.CSSProperties => ({
  width: '100%',
  padding: '14px 20px',
  background: disabled 
    ? 'rgba(100, 108, 255, 0.3)' 
    : 'linear-gradient(135deg, #646cff 0%, #8b5cf6 100%)',
  color: '#fff',
  border: 'none',
  borderRadius: 12,
  cursor: disabled ? 'not-allowed' : 'pointer',
  fontSize: 15,
  fontWeight: 700,
  transition: 'all 0.3s ease',
  transform: disabled ? 'scale(1)' : 'scale(1)',
  boxShadow: disabled 
    ? 'none' 
    : '0 4px 20px rgba(100, 108, 255, 0.4)',
  marginTop: 8
})

const dividerStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: 16,
  margin: '24px 0',
  color: 'rgba(255, 255, 255, 0.4)',
  fontSize: 13
}

const dividerLineStyle: React.CSSProperties = {
  flex: 1,
  height: 1,
  background: 'rgba(100, 108, 255, 0.2)'
}

const googleButtonStyle: React.CSSProperties = {
  width: '100%',
  padding: '14px 20px',
  borderRadius: 12,
  border: '1px solid rgba(100, 108, 255, 0.3)',
  background: 'rgba(255, 255, 255, 0.05)',
  color: '#fff',
  cursor: 'pointer',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  gap: 12,
  fontSize: 15,
  fontWeight: 600,
  transition: 'all 0.3s ease'
}

const errorStyle: React.CSSProperties = {
  padding: '12px 16px',
  background: 'rgba(239, 68, 68, 0.1)',
  border: '1px solid rgba(239, 68, 68, 0.3)',
  borderRadius: 8,
  color: '#fca5a5',
  fontSize: 13,
  textAlign: 'center'
}

// Responsive avec media queries simulées via window.innerWidth
const useResponsive = () => {
  const [isMobile, setIsMobile] = useState(window.innerWidth < 768)

  useEffect(() => {
    const handleResize = () => {
      setIsMobile(window.innerWidth < 768)
    }

    window.addEventListener('resize', handleResize)
    return () => window.removeEventListener('resize', handleResize)
  }, [])

  return { isMobile }
}

export default function Login() {
  const [isLogin, setIsLogin] = useState(true)
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [username, setUsername] = useState('')
  const navigate = useNavigate()
  const { login, register, loginWithGoogle, loading, error, user } = useAuth()
  const { isMobile } = useResponsive()

  // Ajustements responsive
  const responsiveCardBody = {
    ...cardBodyStyle,
    padding: isMobile ? '24px 24px 32px' : '32px 40px 40px'
  }

  const responsiveCardHeader = {
    ...cardHeaderStyle,
    padding: isMobile ? '32px 24px 24px' : '48px 40px 32px'
  }

  useEffect(() => {
    if (user) {
      navigate('/dashboard')
    }
  }, [user, navigate])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()

    try {
      if (isLogin) {
        await login(email, password)
      } else {
        await register(username, email, password)
      }

      navigate('/dashboard')
    } catch (err) {
      console.error(err)
    }
  }

  // Effet hover sur les inputs
  const handleInputFocus = (e: React.FocusEvent<HTMLInputElement>) => {
    e.target.style.borderColor = '#646cff'
    e.target.style.background = 'rgba(15, 15, 15, 0.8)'
  }

  const handleInputBlur = (e: React.FocusEvent<HTMLInputElement>) => {
    e.target.style.borderColor = 'rgba(100, 108, 255, 0.2)'
    e.target.style.background = 'rgba(15, 15, 15, 0.5)'
  }

  // Effet hover sur les boutons
  const handleButtonHover = (e: React.MouseEvent<HTMLButtonElement>, isEnter: boolean) => {
    if (!loading && e.currentTarget.type !== 'button') {
      e.currentTarget.style.transform = isEnter ? 'scale(1.02)' : 'scale(1)'
    }
  }

  const handleGoogleHover = (e: React.MouseEvent<HTMLButtonElement>, isEnter: boolean) => {
    if (!loading) {
      e.currentTarget.style.background = isEnter 
        ? 'rgba(255, 255, 255, 0.1)' 
        : 'rgba(255, 255, 255, 0.05)'
      e.currentTarget.style.borderColor = isEnter 
        ? 'rgba(100, 108, 255, 0.5)' 
        : 'rgba(100, 108, 255, 0.3)'
    }
  }

  return (
    <div style={containerStyle}>
      <div style={backgroundPatternStyle} />
      
      <div style={cardContainerStyle}>
        <div style={cardStyle}>
          <div style={responsiveCardHeader}>
            <div style={logoStyle}>💬</div>
            <h1 style={titleStyle}>DiSpeak</h1>
            <p style={subtitleStyle}>Communication VOIP nouvelle génération</p>
          </div>
          
          <div style={responsiveCardBody}>
            <div style={tabsContainerStyle}>
              <button 
                style={tabButton(isLogin)} 
                onClick={() => setIsLogin(true)}
                type="button"
              >
                Connexion
              </button>
              <button 
                style={tabButton(!isLogin)} 
                onClick={() => setIsLogin(false)}
                type="button"
              >
                Inscription
              </button>
            </div>

            <form onSubmit={handleSubmit} style={formStyle}>
              {!isLogin && (
                <div style={inputGroupStyle}>
                  <label style={labelStyle}>Nom d'utilisateur</label>
                  <input
                    type="text"
                    value={username}
                    onChange={(e) => setUsername(e.target.value)}
                    placeholder="Entrez votre nom d'utilisateur"
                    style={inputStyle}
                    onFocus={handleInputFocus}
                    onBlur={handleInputBlur}
                    required
                  />
                </div>
              )}

              <div style={inputGroupStyle}>
                <label style={labelStyle}>Email</label>
                <input
                  type="email"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  placeholder="votre@email.com"
                  style={inputStyle}
                  onFocus={handleInputFocus}
                  onBlur={handleInputBlur}
                  required
                />
              </div>

              <div style={inputGroupStyle}>
                <label style={labelStyle}>Mot de passe</label>
                <input
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  placeholder="••••••••"
                  style={inputStyle}
                  onFocus={handleInputFocus}
                  onBlur={handleInputBlur}
                  required
                />
              </div>

              {error && (
                <div style={errorStyle}>{error}</div>
              )}

              <button 
                type="submit" 
                style={submitButtonStyle(loading)} 
                disabled={loading}
                onMouseEnter={(e) => handleButtonHover(e, true)}
                onMouseLeave={(e) => handleButtonHover(e, false)}
              >
                {loading ? 'Chargement...' : (isLogin ? 'Se connecter' : "S'inscrire")}
              </button>
            </form>

            <div style={dividerStyle}>
              <div style={dividerLineStyle} />
              <span>ou</span>
              <div style={dividerLineStyle} />
            </div>

            <button 
              type="button" 
              style={googleButtonStyle} 
              onClick={loginWithGoogle} 
              disabled={loading}
              onMouseEnter={(e) => handleGoogleHover(e, true)}
              onMouseLeave={(e) => handleGoogleHover(e, false)}
            >
              <IconBrandGoogleFilled size={20} />
              <span>Continuer avec Google</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}

import { CSSProperties } from 'react'
import { getTopChromeOffset } from '../lib/runtime'

// Couleurs du thème
export const colors = {
  primary: '#5fe9ff',
  primaryDark: '#18bfe9',
  accentWarm: '#ffb347',
  background: `
    radial-gradient(circle at 14% 8%, rgba(24, 191, 233, 0.16) 0%, rgba(24, 191, 233, 0) 42%),
    radial-gradient(circle at 86% 18%, rgba(255, 179, 71, 0.12) 0%, rgba(255, 179, 71, 0) 38%),
    linear-gradient(135deg, #060b12 0%, #0b1220 45%, #101a2d 100%)
  `,
  cardBg: 'rgba(13, 23, 38, 0.78)',
  cardBorder: 'rgba(95, 233, 255, 0.14)',
  textPrimary: '#eef5ff',
  textSecondary: 'rgba(238, 245, 255, 0.66)',
  textTertiary: 'rgba(238, 245, 255, 0.45)',
  inputBg: 'rgba(9, 15, 24, 0.75)',
  hoverBg: 'rgba(255, 255, 255, 0.05)',
  error: '#ff5f7a',
  errorBg: 'rgba(255, 95, 122, 0.12)',
  success: '#39e7a5',
  warning: '#ffb347',
}

// Background pattern réutilisable
export const backgroundPatternStyle: CSSProperties = {
  position: 'absolute',
  top: 0,
  left: 0,
  right: 0,
  bottom: 0,
  opacity: 0.03,
  backgroundImage: `linear-gradient(${colors.primary}22 1px, transparent 1px), linear-gradient(90deg, ${colors.primary}22 1px, transparent 1px)`,
  backgroundSize: '28px 28px',
  pointerEvents: 'none'
}

// Container principal
export const mainContainerStyle: CSSProperties = {
  minHeight: `calc(100vh - ${getTopChromeOffset()}px)`,
  background: colors.background,
  color: colors.textPrimary,
  fontFamily: "'IBM Plex Sans', 'Segoe UI', system-ui, sans-serif",
  marginTop: getTopChromeOffset(),
  position: 'relative',
  overflow: 'hidden'
}

// Carte glassmorphism
export const glassCardStyle: CSSProperties = {
  background: colors.cardBg,
  backdropFilter: 'blur(20px)',
  border: `1px solid ${colors.cardBorder}`,
  borderRadius: 24,
  boxShadow: `0 20px 60px rgba(0, 0, 0, 0.35), 0 0 60px rgba(24, 191, 233, 0.08)`,
  transition: 'transform 0.3s ease, box-shadow 0.3s ease'
}

// Carte glassmorphism simple
export const glassmorphismCard: CSSProperties = {
  background: 'rgba(13, 23, 38, 0.82)',
  backdropFilter: 'blur(20px)',
  border: `1px solid ${colors.cardBorder}`,
  borderRadius: 12,
  padding: 24,
  boxShadow: '0 8px 32px rgba(0, 0, 0, 0.3)',
}

// Header de carte
export const cardHeaderStyle: CSSProperties = {
  padding: '32px 40px',
  background: 'linear-gradient(180deg, rgba(24, 191, 233, 0.12) 0%, transparent 100%)',
  borderBottom: `1px solid ${colors.cardBorder}`
}

// Titre avec gradient
export const gradientTitleStyle: CSSProperties = {
  fontSize: 28,
  fontWeight: 700,
  margin: 0,
  fontFamily: "'Rajdhani', 'IBM Plex Sans', system-ui, sans-serif",
  background: `linear-gradient(135deg, ${colors.primary} 0%, ${colors.primaryDark} 100%)`,
  WebkitBackgroundClip: 'text',
  WebkitTextFillColor: 'transparent',
  letterSpacing: '-0.02em'
}

// Bouton principal
export const primaryButtonStyle = (disabled: boolean = false): CSSProperties => ({
  padding: '12px 24px',
  background: disabled 
    ? 'rgba(24, 191, 233, 0.22)' 
    : `linear-gradient(135deg, ${colors.primary} 0%, ${colors.primaryDark} 100%)`,
  color: '#041019',
  border: 'none',
  borderRadius: 12,
  cursor: disabled ? 'not-allowed' : 'pointer',
  fontSize: 14,
  fontWeight: 600,
  transition: 'all 0.3s ease',
  boxShadow: disabled 
    ? 'none' 
    : '0 6px 22px rgba(24, 191, 233, 0.28)',
  // WebkitAppRegion: 'no-drag' as any
})

// Bouton secondaire
export const secondaryButtonStyle: CSSProperties = {
  padding: '12px 24px',
  background: 'rgba(255, 255, 255, 0.03)',
  color: colors.textPrimary,
  border: `1px solid ${colors.cardBorder}`,
  borderRadius: 12,
  cursor: 'pointer',
  fontSize: 14,
  fontWeight: 600,
  transition: 'all 0.3s ease',
  // WebkitAppRegion: 'no-drag' as any
}

// Input style
export const inputStyle: CSSProperties = {
  padding: '12px 16px',
  borderRadius: 12,
  border: `1px solid ${colors.cardBorder}`,
  background: colors.inputBg,
  color: colors.textPrimary,
  fontSize: 14,
  transition: 'all 0.3s ease',
  outline: 'none'
}

// Section card
export const sectionCardStyle: CSSProperties = {
  background: colors.cardBg,
  backdropFilter: 'blur(20px)',
  border: `1px solid ${colors.cardBorder}`,
  borderRadius: 16,
  padding: 24,
  marginBottom: 24,
  boxShadow: '0 10px 32px rgba(0, 0, 0, 0.22)'
}

// Header de page
export const pageHeaderStyle: CSSProperties = {
  padding: '24px 32px',
  background: 'rgba(9, 16, 27, 0.72)',
  backdropFilter: 'blur(10px)',
  borderBottom: `1px solid ${colors.cardBorder}`,
  display: 'flex',
  justifyContent: 'space-between',
  alignItems: 'center'
}

import React, { useCallback } from 'react'
import { TAURI_TITLEBAR_HEIGHT, isTauriRuntime } from '../lib/runtime'

declare global {
  interface Window {
    __TAURI_IPC__?: unknown
  }
}

let tauriApi: typeof import('@tauri-apps/api/window') | null = null

async function ensureTauriApi() {
  if (!tauriApi && window.__TAURI_IPC__) {
    tauriApi = await import('@tauri-apps/api/window')
  }
  return tauriApi
}

const titlebarStyle: React.CSSProperties = {
  height: TAURI_TITLEBAR_HEIGHT,
  background: '#1a1a1a',
  borderBottom: '1px solid #222',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  padding: '0 16px',
  userSelect: 'none',
  position: 'fixed',
  top: 0,
  left: 0,
  right: 0,
  zIndex: 9999
}

const brandingStyle: React.CSSProperties = {
  fontSize: 13,
  fontWeight: 600,
  color: '#e5e7eb',
  flex: 1,
  pointerEvents: 'none'
}

const controlsStyle: React.CSSProperties = {
  display: 'flex',
  gap: 0,
  pointerEvents: 'auto'
}

type TauriStyle = React.CSSProperties & {
  WebkitAppRegion?: 'drag' | 'no-drag'
}

const buttonStyle: TauriStyle = {
  width: 46,
  height: TAURI_TITLEBAR_HEIGHT,
  border: 'none',
  background: 'transparent',
  color: '#cbd5e1',
  fontSize: 16,
  cursor: 'pointer',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  transition: 'background 0.2s',
  WebkitAppRegion: 'no-drag',
  pointerEvents: 'auto'
}

const closeButtonStyle: React.CSSProperties = {
  ...buttonStyle,
  fontSize: 20,
  pointerEvents: 'auto'
}

const Titlebar: React.FC = () => {
  const minimize = useCallback(async () => {
    const api = await ensureTauriApi()
    await api?.appWindow.minimize()
  }, [])

  const toggleMaximize = useCallback(async () => {
    const api = await ensureTauriApi()
    if (!api) return
    const maximized = await api.appWindow.isMaximized()
    if (maximized) {
      await api.appWindow.unmaximize()
    } else {
      await api.appWindow.maximize()
    }
  }, [])

  const close = useCallback(async () => {
    const api = await ensureTauriApi()
    await api?.appWindow.close()
  }, [])

  if (!isTauriRuntime()) {
    return null
  }

  return (
    <div style={titlebarStyle} data-tauri-drag-region>
      <div style={brandingStyle}>
        <span>DiSpeak</span>
      </div>
      <div style={controlsStyle}>
        <button 
          style={buttonStyle} 
          onClick={minimize}
          onMouseEnter={(e) => { e.currentTarget.style.background = '#333'; }}
          onMouseLeave={(e) => { e.currentTarget.style.background = 'transparent'; }}
          title="Réduire"
          aria-label="Réduire"
        >
          <span style={{ fontSize: 20, color: '#cbd5e1', lineHeight: 1 }}>−</span>
        </button>
        <button 
          style={buttonStyle} 
          onClick={toggleMaximize}
          onMouseEnter={(e) => { e.currentTarget.style.background = '#333'; }}
          onMouseLeave={(e) => { e.currentTarget.style.background = 'transparent'; }}
          title="Agrandir/Restaurer"
          aria-label="Agrandir/Restaurer"
        >
          <span style={{ fontSize: 14, color: '#cbd5e1', lineHeight: 1 }}>☐</span>
        </button>
        <button 
          style={closeButtonStyle} 
          onClick={close}
          onMouseEnter={(e) => { e.currentTarget.style.background = '#dc2626'; }}
          onMouseLeave={(e) => { e.currentTarget.style.background = 'transparent'; }}
          title="Fermer"
          aria-label="Fermer"
        >
          <span style={{ fontSize: 20, color: '#cbd5e1', lineHeight: 1 }}>×</span>
        </button>
      </div>
    </div>
  )
}

export default Titlebar

import React, { useEffect } from 'react'
import ReactDOM from 'react-dom/client'
import { GoogleOAuthProvider } from '@react-oauth/google'
import App from './App'
import './styles/globals.css'
import './index.css'
import { AuthProvider } from './hooks/useAuth'
import Titlebar from './components/Titlebar'
import { getTopChromeOffset } from './lib/runtime'

const googleClientId = import.meta.env.VITE_GOOGLE_CLIENT_ID || 'placeholder-client-id'

const rootElement = document.getElementById('root') as HTMLElement | null

if (!rootElement) {
  throw new Error('Élément root introuvable')
}

const Root = () => {
  const topOffset = getTopChromeOffset()

  useEffect(() => {
    // Désactiver le menu contextuel par défaut (clic droit)
    const handleContextMenu = (e: MouseEvent) => {
      e.preventDefault()
      return false
    }

    document.addEventListener('contextmenu', handleContextMenu)

    return () => {
      document.removeEventListener('contextmenu', handleContextMenu)
    }
  }, [])

  return (
    <React.StrictMode>
      <GoogleOAuthProvider clientId={googleClientId}>
        <AuthProvider>
          <Titlebar />
          <div style={{ paddingTop: topOffset, height: `calc(100vh - ${topOffset}px)`, overflow: 'hidden' }}>
            <App />
          </div>
        </AuthProvider>
      </GoogleOAuthProvider>
    </React.StrictMode>
  )
}

ReactDOM.createRoot(rootElement).render(<Root />)

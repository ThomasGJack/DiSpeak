import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react'
import { googleLogout, TokenResponse, useGoogleLogin } from '@react-oauth/google'
import { api } from '../lib/api'

type UserStatus = 'online' | 'away' | 'busy' | 'offline'

export interface User {
  id: string
  user_number: number
  username: string
  email: string
  provider: string
  providerId?: string | null
  avatar?: string | null
  status: UserStatus
  role: string
  bio?: string | null
  is_active: boolean
  deleted_at?: string | null
}

interface AuthContextValue {
  user: User | null
  loading: boolean
  error: string | null
  login: (email: string, password: string) => Promise<void>
  register: (username: string, email: string, password: string) => Promise<void>
  loginWithGoogle: () => void
  logout: () => void
  updateProfile: (profileData: Partial<User>) => Promise<void>
  refresh: () => Promise<void>
}

const AuthContext = createContext<AuthContextValue | undefined>(undefined)

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<User | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [hasFetched, setHasFetched] = useState(false)

  const fetchCurrentUser = useCallback(async () => {
    try {
      setLoading(true)
      const currentUser = await api.get<User>('/auth/me')
      setUser(currentUser)
    } catch (err) {
      console.error(err)
      localStorage.removeItem('token')
      setUser(null)
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    if (hasFetched) return

    const token = localStorage.getItem('token')
    if (!token) {
      setLoading(false)
      setHasFetched(true)
      return
    }

    setHasFetched(true)
    fetchCurrentUser()
  }, [fetchCurrentUser, hasFetched])

  const login = useCallback(async (email: string, password: string) => {
    setLoading(true)
    setError(null)
    try {
      const response = await api.post<{ user: User; token: string }>('/auth/login', { email, password })
      localStorage.setItem('token', response.token)
      setUser(response.user)
    } catch (err: any) {
      setError(err.message || 'Erreur lors de la connexion')
      throw err
    } finally {
      setLoading(false)
    }
  }, [])

  const register = useCallback(async (username: string, email: string, password: string) => {
    setLoading(true)
    setError(null)
    try {
      const response = await api.post<{ user: User; token: string }>('/auth/register', {
        username,
        email,
        password,
      })
      localStorage.setItem('token', response.token)
      setUser(response.user)
    } catch (err: any) {
      setError(err.message || "Erreur lors de l'inscription")
      throw err
    } finally {
      setLoading(false)
    }
  }, [])

  const handleGoogleSuccess = useCallback(async (tokenResponse: TokenResponse) => {
    if (!tokenResponse.access_token) {
      setError('Jeton Google invalide')
      setLoading(false)
      return
    }

    try {
      const response = await api.post<{ user: User; token: string }>('/auth/login/google', {
        accessToken: tokenResponse.access_token,
      })

      localStorage.setItem('token', response.token)
      setUser(response.user)
    } catch (err: any) {
      setError(err.message || 'Erreur lors de la connexion Google')
    } finally {
      setLoading(false)
    }
  }, [])

  const googleLoginInternal = useGoogleLogin({
    scope: 'openid profile email',
    onSuccess: async (tokenResponse: TokenResponse) => {
      try {
        await handleGoogleSuccess(tokenResponse)
      } catch (err) {
        console.error('Erreur Google OAuth', err)
      }
    },
    onError: (err) => {
      console.error('Erreur Google OAuth', err)
      setError('Connexion Google échouée')
      setLoading(false)
    },
  })

  const loginWithGoogle = useCallback(() => {
    setError(null)
    setLoading(true)
    googleLoginInternal()
  }, [googleLoginInternal])

  const logout = useCallback(() => {
    localStorage.removeItem('token')
    googleLogout()
    setUser(null)
    setError(null)
    setLoading(false)
    setHasFetched(false)
  }, [])

  const updateProfile = useCallback(async (profileData: Partial<User>) => {
    setError(null)
    try {
      if (user) {
        await api.patch('/users/me', profileData)
        setUser({ ...user, ...profileData })
      }
    } catch (err: any) {
      setError(err.message || 'Erreur lors de la mise à jour du profil')
      throw err
    }
  }, [user])

  const refresh = useCallback(async () => {
    await fetchCurrentUser()
  }, [fetchCurrentUser])

  const value = useMemo<AuthContextValue>(() => ({
    user,
    loading,
    error,
    login,
    register,
    loginWithGoogle,
    logout,
    updateProfile,
    refresh,
  }), [user, loading, error, login, register, loginWithGoogle, logout, updateProfile, refresh])

  return (
    <AuthContext.Provider value={value}>
      {children}
    </AuthContext.Provider>
  )
}

export function useAuth() {
  const context = useContext(AuthContext)
  if (!context) {
    throw new Error('useAuth doit être utilisé à l’intérieur de AuthProvider')
  }
  return context
}
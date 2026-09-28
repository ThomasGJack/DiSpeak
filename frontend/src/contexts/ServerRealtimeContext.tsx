import React, { createContext, useContext } from 'react'

import { useWebSocket } from '../hooks/useWebSocket'

type ServerRealtimeValue = ReturnType<typeof useWebSocket>

const ServerRealtimeContext = createContext<ServerRealtimeValue | null>(null)

export const ServerRealtimeProvider: React.FC<{
  serverId?: string
  children: React.ReactNode
}> = ({ serverId, children }) => {
  const realtime = useWebSocket(serverId)

  return (
    <ServerRealtimeContext.Provider value={realtime}>
      {children}
    </ServerRealtimeContext.Provider>
  )
}

export const useServerRealtime = (): ServerRealtimeValue => {
  const context = useContext(ServerRealtimeContext)
  if (!context) {
    throw new Error('useServerRealtime must be used within ServerRealtimeProvider')
  }
  return context
}


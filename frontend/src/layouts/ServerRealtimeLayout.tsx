import React from 'react'
import { Outlet, useParams } from 'react-router-dom'

import { ServerRealtimeProvider } from '../contexts/ServerRealtimeContext'

const ServerRealtimeLayout: React.FC = () => {
  const { serverId } = useParams<{ serverId: string }>()

  return (
    <ServerRealtimeProvider serverId={serverId}>
      <Outlet />
    </ServerRealtimeProvider>
  )
}

export default ServerRealtimeLayout


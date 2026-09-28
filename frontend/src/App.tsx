import { HashRouter as Router, Routes, Route } from 'react-router-dom'
import Login from './pages/Login'
import Dashboard from './pages/Dashboard'
import Servers from './pages/Servers'
import ServerView from './pages/ServerView'
import ServerSettings from './pages/ServerSettings'
import Profile from './pages/Profile'
import CreateBinaryServer from './pages/CreateBinaryServer'
import ServerRealtimeLayout from './layouts/ServerRealtimeLayout'

function App() {
  return (
    <Router>
      <Routes>
        <Route path="/" element={<Login />} />
        <Route path="/dashboard" element={<Dashboard />} />
        <Route path="/profile" element={<Profile />} />
        <Route path="/servers" element={<Servers />} />
        <Route path="/servers/binary/new" element={<CreateBinaryServer />} />
        <Route path="/server/:serverId" element={<ServerRealtimeLayout />}>
          <Route index element={<ServerView />} />
          <Route path="settings" element={<ServerSettings />} />
        </Route>
      </Routes>
    </Router>
  )
}

export default App

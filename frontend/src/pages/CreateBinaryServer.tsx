import { useEffect, useMemo, useState } from 'react'
import { useNavigate, useSearchParams } from 'react-router-dom'
import { useAuth } from '../hooks/useAuth'
import { api } from '../lib/api'
import {
  backgroundPatternStyle,
  colors,
  gradientTitleStyle,
  inputStyle,
  mainContainerStyle,
  pageHeaderStyle,
  primaryButtonStyle,
  secondaryButtonStyle,
  sectionCardStyle,
} from '../styles/commonStyles'

interface Server {
  id: string
  owner_id: string
  name: string
  host: string
  port: number
  max_users: number
  is_online: boolean
  actual_host: string | null
  actual_port: number | null
}

interface CreateServerPayload {
  name: string
  host: string
  port: number
  max_users: number
}

function getDefaultBackendUrl(): string {
  if (import.meta.env.VITE_BACKEND_URL) return String(import.meta.env.VITE_BACKEND_URL)
  if (typeof window !== 'undefined') {
    return `${window.location.protocol}//${window.location.hostname}:8000`
  }
  return 'http://127.0.0.1:8000'
}

function buildToml(params: {
  serverId: string
  instanceName: string
  databaseUrl: string
  wsHost: string
  wsPort: number
  internalApiHost: string
  internalApiPort: number
  internalApiToken: string
  backendUrl: string
  bootstrapTimeoutMs: number
  heartbeatIntervalSec: number
  allowInsecureHttpBackend: boolean
  allowLegacyHs256: boolean
}): string {
  return `# DiSpeak Server Binary Configuration
# Fichier prérempli exporté depuis l'interface web (clé propriétaire non incluse)
# 1) Ouvrez Profil > Clé propriétaire serveur
# 2) Collez votre clé dans owner_key
# 3) Lancez le binaire: dispeak-server run --config ./server-config.toml

[server]
id = "${params.serverId}"
database_url = "${params.databaseUrl}"
ws_host = "${params.wsHost}"
ws_port = ${params.wsPort}
internal_api_host = "${params.internalApiHost}"
internal_api_port = ${params.internalApiPort}
instance_name = "${params.instanceName}"
internal_api_token = "${params.internalApiToken}"

[backend]
url = "${params.backendUrl}"
owner_key = ""
# owner_key = "dso_xxxxx_xxxxxxxxx"
bootstrap_timeout_ms = ${params.bootstrapTimeoutMs}
heartbeat_interval_sec = ${params.heartbeatIntervalSec}

[security]
enforce_owner_validation = true
allow_insecure_http_backend = ${params.allowInsecureHttpBackend ? 'true' : 'false'}
allow_legacy_hs256 = ${params.allowLegacyHs256 ? 'true' : 'false'}

[logging]
level = "info"
json = false
`
}

export default function CreateBinaryServer() {
  const navigate = useNavigate()
  const { user } = useAuth()
  const [searchParams] = useSearchParams()
  const preselectServerId = searchParams.get('serverId')

  const [loading, setLoading] = useState(true)
  const [pageError, setPageError] = useState<string | null>(null)
  const [pageSuccess, setPageSuccess] = useState<string | null>(null)
  const [myServers, setMyServers] = useState<Server[]>([])
  const [selectedServerId, setSelectedServerId] = useState('')

  const [createPending, setCreatePending] = useState(false)
  const [createForm, setCreateForm] = useState<CreateServerPayload>({
    name: 'Mon serveur DiSpeak',
    host: '127.0.0.1',
    port: 8001,
    max_users: 100,
  })

  const [backendUrl, setBackendUrl] = useState(getDefaultBackendUrl())
  const [databaseUrl, setDatabaseUrl] = useState('sqlite:server.db')
  const [wsHost, setWsHost] = useState('0.0.0.0')
  const [wsPort, setWsPort] = useState(8001)
  const [internalApiHost, setInternalApiHost] = useState('127.0.0.1')
  const [internalApiPort, setInternalApiPort] = useState(8002)
  const [internalApiToken, setInternalApiToken] = useState('change-me-internal-api-token')
  const [bootstrapTimeoutMs, setBootstrapTimeoutMs] = useState(5000)
  const [heartbeatIntervalSec, setHeartbeatIntervalSec] = useState(30)
  const [allowInsecureHttpBackend, setAllowInsecureHttpBackend] = useState(false)
  const [allowLegacyHs256, setAllowLegacyHs256] = useState(false)
  const [copyOk, setCopyOk] = useState(false)

  useEffect(() => {
    if (!user) {
      navigate('/')
      return
    }

    let cancelled = false
    const load = async () => {
      try {
        setLoading(true)
        setPageError(null)
        const servers = await api.get<Server[]>('/servers/my')
        if (cancelled) return
        setMyServers(servers)
        const preferred =
          (preselectServerId && servers.find((s) => s.id === preselectServerId)?.id) ||
          servers[0]?.id ||
          ''
        setSelectedServerId((current) => current || preferred)
      } catch (err: any) {
        if (!cancelled) {
          setPageError(err.message || 'Impossible de charger vos serveurs')
        }
      } finally {
        if (!cancelled) {
          setLoading(false)
        }
      }
    }
    load()
    return () => {
      cancelled = true
    }
  }, [user, navigate, preselectServerId])

  const selectedServer = useMemo(
    () => myServers.find((s) => s.id === selectedServerId) ?? null,
    [myServers, selectedServerId],
  )

  useEffect(() => {
    if (!selectedServer) return
    const selectedPort = selectedServer.actual_port ?? selectedServer.port ?? 8001
    setWsPort(selectedPort)
    setInternalApiPort(selectedPort + 1)
  }, [selectedServer])

  const tomlPreview = useMemo(() => {
    if (!selectedServer) {
      return '# Sélectionnez ou créez d’abord un serveur backend'
    }
    return buildToml({
      serverId: selectedServer.id,
      instanceName: selectedServer.name,
      databaseUrl,
      wsHost,
      wsPort,
      internalApiHost,
      internalApiPort,
      internalApiToken,
      backendUrl,
      bootstrapTimeoutMs,
      heartbeatIntervalSec,
      allowInsecureHttpBackend,
      allowLegacyHs256,
    })
  }, [
    selectedServer,
    databaseUrl,
    wsHost,
    wsPort,
    internalApiHost,
    internalApiPort,
    internalApiToken,
    backendUrl,
    bootstrapTimeoutMs,
    heartbeatIntervalSec,
    allowInsecureHttpBackend,
    allowLegacyHs256,
  ])

  const handleCreateServer = async () => {
    try {
      setCreatePending(true)
      setPageError(null)
      setPageSuccess(null)
      const created = await api.post<Server>('/servers', createForm)
      setMyServers((prev) => [created, ...prev.filter((s) => s.id !== created.id)])
      setSelectedServerId(created.id)
      setPageSuccess(`Serveur créé: ${created.name} (${created.id})`)
    } catch (err: any) {
      setPageError(err.message || 'Impossible de créer le serveur côté backend')
    } finally {
      setCreatePending(false)
    }
  }

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(tomlPreview)
      setCopyOk(true)
      setPageSuccess('server-config.toml copié (sans clé propriétaire)')
      window.setTimeout(() => setCopyOk(false), 1500)
    } catch {
      setPageError('Impossible de copier automatiquement le TOML')
    }
  }

  const handleDownload = () => {
    const blob = new Blob([tomlPreview], { type: 'text/plain;charset=utf-8' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = 'server-config.toml'
    a.click()
    URL.revokeObjectURL(url)
    setPageSuccess('Fichier server-config.toml téléchargé (clé non incluse)')
  }

  if (!user) return null

  return (
    <div style={mainContainerStyle}>
      <div style={backgroundPatternStyle} />

      <div style={{ ...pageHeaderStyle, position: 'relative', zIndex: 1 }}>
        <div>
          <h1 style={gradientTitleStyle}>Créer un serveur binaire</h1>
          <p style={{ margin: '8px 0 0', color: colors.textSecondary, fontSize: 14 }}>
            Workflow complet: créer le serveur backend, puis exporter un <code>server-config.toml</code> prérempli sans clé.
          </p>
        </div>
        <div style={{ display: 'flex', gap: 10 }}>
          <button style={secondaryButtonStyle} onClick={() => navigate('/profile')}>
            Profil / clé propriétaire
          </button>
          <button style={secondaryButtonStyle} onClick={() => navigate('/servers')}>
            ← Retour aux serveurs
          </button>
        </div>
      </div>

      <main style={{ maxWidth: 1280, margin: '0 auto', padding: 24, position: 'relative', zIndex: 1 }}>
        {pageError && (
          <div style={{ ...sectionCardStyle, border: '1px solid rgba(255,95,122,0.35)', background: 'rgba(127,29,29,0.24)', color: '#fecaca' }}>
            {pageError}
          </div>
        )}
        {pageSuccess && (
          <div style={{ ...sectionCardStyle, border: '1px solid rgba(57,231,165,0.32)', background: 'rgba(6,78,59,0.18)', color: '#6ee7b7' }}>
            {pageSuccess}
          </div>
        )}

        <div style={{ display: 'grid', gridTemplateColumns: 'minmax(340px, 1fr) minmax(420px, 1.35fr)', gap: 20 }}>
          <section style={sectionCardStyle}>
            <h2 style={{ marginTop: 0 }}>1. Créer ou sélectionner un serveur</h2>
            <p style={{ color: colors.textSecondary, marginTop: 0 }}>
              Le <code>server_id</code> est créé côté backend. Le binaire local s’y lie ensuite via votre clé propriétaire.
            </p>

            <div style={{ display: 'grid', gap: 10 }}>
              <label style={{ color: colors.textSecondary, fontSize: 13 }}>Nom du serveur</label>
              <input
                style={inputStyle}
                value={createForm.name}
                onChange={(e) => setCreateForm((prev) => ({ ...prev, name: e.target.value }))}
              />

              <div style={{ display: 'grid', gridTemplateColumns: '1fr 120px 120px', gap: 10 }}>
                <div>
                  <label style={{ color: colors.textSecondary, fontSize: 13 }}>Host (backend)</label>
                  <input
                    style={{ ...inputStyle, width: '100%' }}
                    value={createForm.host}
                    onChange={(e) => setCreateForm((prev) => ({ ...prev, host: e.target.value }))}
                  />
                </div>
                <div>
                  <label style={{ color: colors.textSecondary, fontSize: 13 }}>Port</label>
                  <input
                    style={{ ...inputStyle, width: '100%' }}
                    type="number"
                    min={1}
                    max={65535}
                    value={createForm.port}
                    onChange={(e) => setCreateForm((prev) => ({ ...prev, port: Number(e.target.value) || 8001 }))}
                  />
                </div>
                <div>
                  <label style={{ color: colors.textSecondary, fontSize: 13 }}>Slots</label>
                  <input
                    style={{ ...inputStyle, width: '100%' }}
                    type="number"
                    min={1}
                    max={10000}
                    value={createForm.max_users}
                    onChange={(e) => setCreateForm((prev) => ({ ...prev, max_users: Number(e.target.value) || 100 }))}
                  />
                </div>
              </div>

              <button style={primaryButtonStyle(createPending)} disabled={createPending} onClick={handleCreateServer}>
                {createPending ? 'Création…' : 'Créer le serveur côté backend'}
              </button>
            </div>

            <div style={{ marginTop: 18, paddingTop: 16, borderTop: `1px solid ${colors.cardBorder}` }}>
              <label style={{ color: colors.textSecondary, fontSize: 13, display: 'block', marginBottom: 6 }}>
                Serveur cible pour l’export
              </label>
              <select
                style={{ ...inputStyle, width: '100%' }}
                value={selectedServerId}
                onChange={(e) => setSelectedServerId(e.target.value)}
                disabled={loading}
              >
                <option value="">{loading ? 'Chargement...' : 'Choisir un serveur'}</option>
                {myServers.map((server) => (
                  <option key={server.id} value={server.id}>
                    {server.name} ({server.id.slice(0, 8)}…)
                  </option>
                ))}
              </select>

              {selectedServer && (
                <div style={{
                  marginTop: 10,
                  padding: 12,
                  borderRadius: 12,
                  border: `1px solid ${colors.cardBorder}`,
                  background: 'rgba(255,255,255,0.02)',
                }}>
                  <div style={{ fontWeight: 600, color: colors.textPrimary }}>{selectedServer.name}</div>
                  <div style={{ marginTop: 4, fontFamily: 'monospace', fontSize: 12, color: colors.textSecondary }}>
                    server_id = {selectedServer.id}
                  </div>
                  <div style={{ marginTop: 4, fontSize: 12, color: colors.textSecondary }}>
                    Backend host/port déclarés: {selectedServer.host}:{selectedServer.port}
                  </div>
                </div>
              )}
            </div>
          </section>

          <section style={sectionCardStyle}>
            <h2 style={{ marginTop: 0 }}>2. Préremplir et exporter `server-config.toml`</h2>
            <p style={{ color: colors.textSecondary, marginTop: 0 }}>
              La clé propriétaire n’est pas incluse. Copiez-la depuis <strong>Profil</strong> puis collez-la dans <code>owner_key</code>.
            </p>

            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
              <div>
                <label style={{ color: colors.textSecondary, fontSize: 13 }}>Backend URL</label>
                <input style={{ ...inputStyle, width: '100%' }} value={backendUrl} onChange={(e) => setBackendUrl(e.target.value)} />
              </div>
              <div>
                <label style={{ color: colors.textSecondary, fontSize: 13 }}>Database URL (local)</label>
                <input style={{ ...inputStyle, width: '100%' }} value={databaseUrl} onChange={(e) => setDatabaseUrl(e.target.value)} />
              </div>
              <div>
                <label style={{ color: colors.textSecondary, fontSize: 13 }}>WS Host</label>
                <input style={{ ...inputStyle, width: '100%' }} value={wsHost} onChange={(e) => setWsHost(e.target.value)} />
              </div>
              <div>
                <label style={{ color: colors.textSecondary, fontSize: 13 }}>WS Port</label>
                <input style={{ ...inputStyle, width: '100%' }} type="number" min={1} max={65535} value={wsPort} onChange={(e) => setWsPort(Number(e.target.value) || 8001)} />
              </div>
              <div>
                <label style={{ color: colors.textSecondary, fontSize: 13 }}>API interne Host</label>
                <input style={{ ...inputStyle, width: '100%' }} value={internalApiHost} onChange={(e) => setInternalApiHost(e.target.value)} />
              </div>
              <div>
                <label style={{ color: colors.textSecondary, fontSize: 13 }}>API interne Port</label>
                <input style={{ ...inputStyle, width: '100%' }} type="number" min={1} max={65535} value={internalApiPort} onChange={(e) => setInternalApiPort(Number(e.target.value) || 8002)} />
              </div>
              <div>
                <label style={{ color: colors.textSecondary, fontSize: 13 }}>Bootstrap timeout (ms)</label>
                <input style={{ ...inputStyle, width: '100%' }} type="number" min={1000} max={60000} value={bootstrapTimeoutMs} onChange={(e) => setBootstrapTimeoutMs(Number(e.target.value) || 5000)} />
              </div>
              <div>
                <label style={{ color: colors.textSecondary, fontSize: 13 }}>Heartbeat (sec)</label>
                <input style={{ ...inputStyle, width: '100%' }} type="number" min={5} max={600} value={heartbeatIntervalSec} onChange={(e) => setHeartbeatIntervalSec(Number(e.target.value) || 30)} />
              </div>
              <div style={{ gridColumn: '1 / -1' }}>
                <label style={{ color: colors.textSecondary, fontSize: 13 }}>Token API interne</label>
                <input style={{ ...inputStyle, width: '100%' }} value={internalApiToken} onChange={(e) => setInternalApiToken(e.target.value)} />
              </div>
            </div>

            <div style={{ display: 'flex', gap: 16, flexWrap: 'wrap', marginTop: 12 }}>
              <label style={{ color: colors.textSecondary, fontSize: 13, display: 'flex', alignItems: 'center', gap: 8 }}>
                <input type="checkbox" checked={allowInsecureHttpBackend} onChange={(e) => setAllowInsecureHttpBackend(e.target.checked)} />
                Autoriser backend HTTP (dev)
              </label>
              <label style={{ color: colors.textSecondary, fontSize: 13, display: 'flex', alignItems: 'center', gap: 8 }}>
                <input type="checkbox" checked={allowLegacyHs256} onChange={(e) => setAllowLegacyHs256(e.target.checked)} />
                Autoriser HS256 legacy (dev)
              </label>
            </div>

            <div style={{ display: 'flex', gap: 10, flexWrap: 'wrap', marginTop: 14 }}>
              <button style={secondaryButtonStyle} onClick={handleCopy} disabled={!selectedServer}>
                {copyOk ? 'Copié' : 'Copier le TOML'}
              </button>
              <button style={primaryButtonStyle(!selectedServer)} onClick={handleDownload} disabled={!selectedServer}>
                Télécharger `server-config.toml`
              </button>
            </div>

            <div style={{
              marginTop: 12,
              padding: 12,
              borderRadius: 12,
              border: '1px solid rgba(255,179,71,0.24)',
              background: 'rgba(255,179,71,0.08)',
              color: '#ffd9a6',
              fontSize: 13,
              lineHeight: 1.5,
            }}>
              Checklist: <strong>1)</strong> exporter le fichier, <strong>2)</strong> récupérer votre clé propriétaire depuis <strong>Profil</strong>, <strong>3)</strong> coller la clé dans <code>owner_key</code>, <strong>4)</strong> lancer le binaire.
            </div>

            <div style={{ marginTop: 16 }}>
              <label style={{ display: 'block', color: colors.textSecondary, fontSize: 13, marginBottom: 6 }}>
                Aperçu
              </label>
              <textarea
                readOnly
                value={tomlPreview}
                style={{
                  ...inputStyle,
                  width: '100%',
                  minHeight: 360,
                  resize: 'vertical',
                  fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Consolas, monospace',
                  lineHeight: 1.45,
                }}
              />
            </div>
          </section>
        </div>
      </main>
    </div>
  )
}

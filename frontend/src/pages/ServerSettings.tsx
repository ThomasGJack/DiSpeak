import React, { useCallback, useMemo, useState } from 'react'
import { useParams } from 'react-router-dom'
import { IconHash, IconLock, IconSettings, IconShield, IconUsers } from '@tabler/icons-react'

import { useAuth } from '../hooks/useAuth'
import { useServerManagement } from '../hooks/useServerManagement'
import { useRoles } from '../hooks/useRoles'
import { useServerRealtime } from '../contexts/ServerRealtimeContext'
import { PermissionMatrix } from '../components/PermissionMatrix'
import { DraggableChannelList } from '../components/DraggableChannelList'

type SettingsTab = 'overview' | 'channels' | 'roles' | 'permissions'

const cardClass = 'rounded-2xl border border-cyan-300/10 bg-slate-950/55 backdrop-blur-xl p-5 shadow-[0_18px_40px_rgba(2,8,20,0.35)]'
const tabClass = 'px-3 py-2 rounded-xl text-sm font-medium transition-colors border'

const ServerSettings: React.FC = () => {
  const { serverId } = useParams<{ serverId: string }>()
  const { user } = useAuth()
  const [activeTab, setActiveTab] = useState<SettingsTab>('overview')
  const [notice, setNotice] = useState<string | null>(null)
  const [wsActionLoading, setWsActionLoading] = useState(false)

  const [newCategoryName, setNewCategoryName] = useState('')
  const [newChannelName, setNewChannelName] = useState('')
  const [newChannelType, setNewChannelType] = useState<'text' | 'voice'>('text')
  const [newChannelCategoryId, setNewChannelCategoryId] = useState<string>('')
  const [newChannelDescription, setNewChannelDescription] = useState('')

  const [newRoleName, setNewRoleName] = useState('')
  const [newRoleColor, setNewRoleColor] = useState('#99AAB5')
  const [memberRoleSelections, setMemberRoleSelections] = useState<Record<string, string>>({})

  const {
    connected,
    error,
    server: liveServer,
    channels,
    categories,
    categoryGroups,
    uncategorizedChannels,
    roles,
    serverMembers,
    requestServerStructure,
    sendCommand,
    request,
  } = useServerRealtime()

  const { server: apiServer, loading: apiLoading, error: apiError } = useServerManagement(serverId)

  const {
    roles: restRoles,
    membersWithRoles,
    loading: rolesLoading,
    error: rolesError,
    createRole,
    updateRole,
    deleteRole,
    assignRole,
    removeRole,
    fetchRoles,
    fetchMembersWithRoles,
  } = useRoles(serverId, { webSocketServerId: undefined })

  const server = liveServer ?? apiServer

  const userMember = useMemo(
    () => serverMembers.find(member => member.user_id === user?.id) ?? null,
    [serverMembers, user?.id]
  )

  const isOwner = server?.owner_id === user?.id
  const canManage = isOwner || userMember?.role === 'owner' || userMember?.role === 'admin'

  const effectiveRoles = restRoles.length > 0 ? restRoles : roles
  const memberRows = membersWithRoles.length > 0 ? membersWithRoles : []

  const refreshAll = useCallback(() => {
    if (!serverId) return
    requestServerStructure()
    void fetchRoles(serverId)
    void fetchMembersWithRoles(serverId)
  }, [serverId, requestServerStructure, fetchRoles, fetchMembersWithRoles])

  const refreshStructureSoon = useCallback(() => {
    window.setTimeout(() => {
      requestServerStructure()
    }, 250)
  }, [requestServerStructure])

  const runWsAction = useCallback(
    async (action: string, data: Record<string, unknown>, successMessage?: string) => {
      if (!user?.id || !serverId) {
        throw new Error('Utilisateur ou serveur manquant')
      }

      setWsActionLoading(true)
      setNotice(null)

      try {
        const payload = {
          ...data,
          user_id: user.id,
        }

        const ackByAction: Record<string, string> = {
          create_channel: 'channel_create_ack',
          update_channel: 'channel_update_ack',
          delete_channel: 'channel_delete_ack',
          move_channel: 'channel_move_ack',
          reorder_channels: 'channel_reorder_ack',
          create_category: 'category_create_ack',
          update_category: 'category_update_ack',
          delete_category: 'category_delete_ack',
          move_category: 'category_move_ack',
          reorder_categories: 'category_reorder_ack',
        }

        const expectedAction = ackByAction[action]
        if (expectedAction) {
          await request({
            action,
            data: payload,
            expectedActions: [expectedAction],
            timeoutMs: 5000,
          })
        } else {
          await sendCommand({
            action,
            data: payload,
          })
        }

        refreshStructureSoon()
        if (successMessage) {
          setNotice(successMessage)
        }
      } catch (actionError) {
        const message = actionError instanceof Error ? actionError.message : 'Erreur WebSocket'
        setNotice(message)
        throw actionError
      } finally {
        setWsActionLoading(false)
      }
    },
    [request, sendCommand, user?.id, serverId, refreshStructureSoon]
  )

  const handleCreateCategory = useCallback(async () => {
    const name = newCategoryName.trim()
    if (!serverId || !name) return

    await runWsAction(
      'create_category',
      {
        server_id: serverId,
        name,
      },
      'Catégorie créée'
    )
    setNewCategoryName('')
  }, [newCategoryName, runWsAction, serverId])

  const handleCreateChannel = useCallback(async () => {
    const name = newChannelName.trim()
    if (!serverId || !name) return

    await runWsAction(
      'create_channel',
      {
        server_id: serverId,
        name,
        channel_type: newChannelType,
        category_id: newChannelCategoryId || null,
        description: newChannelDescription.trim() || null,
      },
      'Canal créé'
    )

    setNewChannelName('')
    setNewChannelDescription('')
    setNewChannelCategoryId('')
    setNewChannelType('text')
  }, [newChannelName, newChannelType, newChannelCategoryId, newChannelDescription, runWsAction, serverId])

  const handleRenameCategory = useCallback(async (categoryId: string, currentName: string) => {
    const nextName = window.prompt('Nouveau nom de la catégorie', currentName)?.trim()
    if (!nextName || nextName === currentName) return

    await runWsAction(
      'update_category',
      { category_id: categoryId, name: nextName },
      'Catégorie mise à jour'
    )
  }, [runWsAction])

  const handleDeleteCategory = useCallback(async (categoryId: string) => {
    if (!window.confirm('Supprimer cette catégorie ?')) return
    await runWsAction('delete_category', { category_id: categoryId }, 'Catégorie supprimée')
  }, [runWsAction])

  const handleMoveCategory = useCallback(async (categoryId: string, direction: -1 | 1) => {
    const ordered = [...categories].sort((a, b) => a.position - b.position)
    const currentIndex = ordered.findIndex(category => category.id === categoryId)
    if (currentIndex < 0) return

    const target = ordered[currentIndex + direction]
    if (!target) return

    await runWsAction(
      'move_category',
      { category_id: categoryId, position: target.position },
      'Position de catégorie mise à jour'
    )
  }, [categories, runWsAction])

  const handleMoveCategoryToPosition = useCallback(async (categoryId: string, position: number) => {
    await runWsAction(
      'move_category',
      { category_id: categoryId, position },
      'Position de catégorie mise à jour'
    )
  }, [runWsAction])

  const handleRenameChannel = useCallback(async (channelId: string, currentName: string, currentDescription?: string | null) => {
    const nextName = window.prompt('Nouveau nom du canal', currentName)?.trim()
    if (!nextName) return

    const nextDescription = window.prompt('Description (laisser vide pour aucune)', currentDescription ?? '')

    await runWsAction(
      'update_channel',
      {
        channel_id: channelId,
        name: nextName,
        description: nextDescription?.trim() ? nextDescription.trim() : null,
      },
      'Canal mis à jour'
    )
  }, [runWsAction])

  const handleDeleteChannel = useCallback(async (channelId: string) => {
    if (!window.confirm('Supprimer ce canal ?')) return
    await runWsAction('delete_channel', { channel_id: channelId }, 'Canal supprimé')
  }, [runWsAction])

  const handleMoveChannel = useCallback(async (channelId: string, direction: -1 | 1) => {
    const current = channels.find(channel => channel.id === channelId)
    if (!current) return

    const siblings = channels
      .filter(channel => (channel.category_id ?? null) === (current.category_id ?? null))
      .sort((a, b) => a.position - b.position)
    const currentIndex = siblings.findIndex(channel => channel.id === channelId)
    if (currentIndex < 0) return

    const target = siblings[currentIndex + direction]
    if (!target) return

    await runWsAction(
      'move_channel',
      {
        channel_id: channelId,
        position: target.position,
        category_id: current.category_id ?? null,
      },
      'Position de canal mise à jour'
    )
  }, [channels, runWsAction])

  const handleMoveChannelToPosition = useCallback(
    async (channelId: string, position: number, categoryId?: string | null) => {
      await runWsAction(
        'move_channel',
        {
          channel_id: channelId,
          position,
          category_id: categoryId ?? null,
        },
        'Position de canal mise à jour'
      )
    },
    [runWsAction]
  )

  const handleReorderCategories = useCallback(async (orderedCategoryIds: string[]) => {
    if (!serverId || orderedCategoryIds.length === 0) return
    await runWsAction(
      'reorder_categories',
      {
        server_id: serverId,
        ordered_category_ids: orderedCategoryIds,
      },
      'Ordre des catégories mis à jour'
    )
  }, [runWsAction, serverId])

  const handleReorderChannels = useCallback(async (categoryId: string | null, orderedChannelIds: string[]) => {
    if (!serverId || orderedChannelIds.length === 0) return
    await runWsAction(
      'reorder_channels',
      {
        server_id: serverId,
        category_id: categoryId,
        ordered_channel_ids: orderedChannelIds,
      },
      'Ordre des canaux mis à jour'
    )
  }, [runWsAction, serverId])

  const handleCreateRole = useCallback(async () => {
    if (!serverId || !newRoleName.trim()) return

    await createRole(serverId, {
      name: newRoleName.trim(),
      color: newRoleColor,
    })
    setNotice('Rôle créé')
    setNewRoleName('')
    await fetchRoles(serverId)
    await fetchMembersWithRoles(serverId)
  }, [createRole, serverId, newRoleName, newRoleColor, fetchRoles, fetchMembersWithRoles])

  const handleEditRole = useCallback(async (roleId: string, currentName: string, currentColor: string) => {
    if (!serverId) return

    const name = window.prompt('Nouveau nom du rôle', currentName)?.trim()
    if (!name) return

    const color = window.prompt('Couleur hex (#RRGGBB)', currentColor)?.trim() || currentColor
    await updateRole(serverId, roleId, { name, color })
    setNotice('Rôle mis à jour')
    await fetchRoles(serverId)
    await fetchMembersWithRoles(serverId)
  }, [serverId, updateRole, fetchRoles, fetchMembersWithRoles])

  const handleDeleteRole = useCallback(async (roleId: string, isDefault: boolean) => {
    if (!serverId || isDefault) return
    if (!window.confirm('Supprimer ce rôle ?')) return

    await deleteRole(serverId, roleId)
    setNotice('Rôle supprimé')
    await fetchRoles(serverId)
    await fetchMembersWithRoles(serverId)
  }, [serverId, deleteRole, fetchRoles, fetchMembersWithRoles])

  const handleAssignRole = useCallback(async (memberUserId: string) => {
    if (!serverId) return
    const selectedRoleId = memberRoleSelections[memberUserId]
    if (!selectedRoleId) return

    await assignRole(memberUserId, selectedRoleId)
    setNotice('Rôle assigné')
    await fetchMembersWithRoles(serverId)
    setMemberRoleSelections(prev => ({ ...prev, [memberUserId]: '' }))
  }, [serverId, memberRoleSelections, assignRole, fetchMembersWithRoles])

  const handleRemoveRoleFromMember = useCallback(async (memberUserId: string, roleId: string) => {
    if (!serverId) return
    await removeRole(memberUserId, roleId)
    setNotice('Rôle retiré')
    await fetchMembersWithRoles(serverId)
  }, [serverId, removeRole, fetchMembersWithRoles])

  const handleSetChannelRolePermissions = useCallback(
    async (channelId: string, roleId: string, allow: number, deny: number) => {
      if (!user?.id) {
        throw new Error('Utilisateur non connecté')
      }

      setWsActionLoading(true)
      setNotice(null)

      try {
        await request({
          action: 'set_channel_permissions',
          data: {
            channel_id: channelId,
            role_id: roleId,
            allow_bitfield: allow,
            deny_bitfield: deny,
            user_id: user.id,
          },
          expectedActions: ['set_channel_permissions_ack'],
          timeoutMs: 5000,
        })

        setNotice('Permissions de canal mises à jour')
      } catch (permissionError) {
        const message =
          permissionError instanceof Error ? permissionError.message : 'Erreur de mise à jour des permissions'
        setNotice(message)
        throw permissionError
      } finally {
        setWsActionLoading(false)
      }
    },
    [request, user?.id]
  )

  const handleSetCategoryRolePermissions = useCallback(
    async (categoryId: string, roleId: string, allow: number, deny: number) => {
      if (!user?.id) {
        throw new Error('Utilisateur non connecté')
      }

      setWsActionLoading(true)
      setNotice(null)

      try {
        await request({
          action: 'set_category_permissions',
          data: {
            category_id: categoryId,
            role_id: roleId,
            allow_bitfield: allow,
            deny_bitfield: deny,
            user_id: user.id,
          },
          expectedActions: ['set_category_permissions_ack'],
          timeoutMs: 5000,
        })

        setNotice('Permissions de catégorie mises à jour')
      } catch (permissionError) {
        const message =
          permissionError instanceof Error ? permissionError.message : 'Erreur de mise à jour des permissions'
        setNotice(message)
        throw permissionError
      } finally {
        setWsActionLoading(false)
      }
    },
    [request, user?.id]
  )

  const handleSetMemberChannelPermissions = useCallback(
    async (channelId: string, memberId: string, allow: number, deny: number) => {
      if (!user?.id) {
        throw new Error('Utilisateur non connecté')
      }

      setWsActionLoading(true)
      setNotice(null)

      try {
        await request({
          action: 'set_member_channel_permissions',
          data: {
            channel_id: channelId,
            member_id: memberId,
            allow_bitfield: allow,
            deny_bitfield: deny,
            user_id: user.id,
          },
          expectedActions: ['set_member_channel_permissions_ack'],
          timeoutMs: 5000,
        })

        setNotice('Overrides membre/canal mis à jour')
      } catch (permissionError) {
        const message =
          permissionError instanceof Error ? permissionError.message : 'Erreur de mise à jour des overrides membre/canal'
        setNotice(message)
        throw permissionError
      } finally {
        setWsActionLoading(false)
      }
    },
    [request, user?.id]
  )

  const handleSetMemberCategoryPermissions = useCallback(
    async (categoryId: string, memberId: string, allow: number, deny: number) => {
      if (!user?.id) {
        throw new Error('Utilisateur non connecté')
      }

      setWsActionLoading(true)
      setNotice(null)

      try {
        await request({
          action: 'set_member_category_permissions',
          data: {
            category_id: categoryId,
            member_id: memberId,
            allow_bitfield: allow,
            deny_bitfield: deny,
            user_id: user.id,
          },
          expectedActions: ['set_member_category_permissions_ack'],
          timeoutMs: 5000,
        })

        setNotice('Overrides membre/catégorie mis à jour')
      } catch (permissionError) {
        const message =
          permissionError instanceof Error ? permissionError.message : 'Erreur de mise à jour des overrides membre/catégorie'
        setNotice(message)
        throw permissionError
      } finally {
        setWsActionLoading(false)
      }
    },
    [request, user?.id]
  )

  const handleGetEffectivePermissions = useCallback(
    async (targetUserId: string, channelId?: string, categoryId?: string) => {
      if (!user?.id) {
        throw new Error('Utilisateur non connecté')
      }

      if (targetUserId !== user.id) {
        throw new Error("Le calcul des permissions effectives via WS est actuellement limité à l'utilisateur connecté")
      }

      const response = await request({
        action: 'get_effective_permissions',
        data: {
          user_id: targetUserId,
          channel_id: channelId,
          category_id: categoryId,
        },
        expectedActions: ['effective_permissions'],
        timeoutMs: 5000,
      })

      return response as {
        allow_bitfield: number
        deny_bitfield: number
        final_permissions: number
        channel_id?: string
        category_id?: string
      }
    },
    [request, user?.id]
  )

  if (!serverId) {
    return (
      <div className="h-full min-h-screen bg-[radial-gradient(circle_at_10%_10%,rgba(24,191,233,0.16),transparent_42%),radial-gradient(circle_at_90%_12%,rgba(255,179,71,0.12),transparent_38%),#070b12] text-white flex items-center justify-center">
        <div className={cardClass}>ID de serveur manquant.</div>
      </div>
    )
  }

  if (apiLoading && !server) {
    return (
      <div className="h-full min-h-screen bg-[radial-gradient(circle_at_10%_10%,rgba(24,191,233,0.16),transparent_42%),radial-gradient(circle_at_90%_12%,rgba(255,179,71,0.12),transparent_38%),#070b12] text-white flex items-center justify-center">
        <div className="text-center">
          <div className="animate-spin rounded-full h-10 w-10 border-b-2 border-cyan-400 mx-auto mb-3" />
          <p className="text-slate-300">Chargement des paramètres du serveur...</p>
        </div>
      </div>
    )
  }

  if (!canManage) {
    return (
      <div className="h-full min-h-screen bg-[radial-gradient(circle_at_10%_10%,rgba(24,191,233,0.16),transparent_42%),radial-gradient(circle_at_90%_12%,rgba(255,179,71,0.12),transparent_38%),#070b12] text-white flex items-center justify-center px-4">
        <div className={`${cardClass} max-w-lg w-full text-center`}>
          <IconLock className="w-12 h-12 text-red-400 mx-auto mb-3" />
          <h1 className="text-xl font-semibold mb-2">Accès refusé</h1>
          <p className="text-slate-300">
            Vous n&apos;avez pas les permissions nécessaires pour gérer ce serveur.
          </p>
        </div>
      </div>
    )
  }

  return (
    <div className="min-h-screen text-white bg-[radial-gradient(circle_at_8%_8%,rgba(24,191,233,0.16),transparent_40%),radial-gradient(circle_at_92%_10%,rgba(255,179,71,0.12),transparent_36%),linear-gradient(180deg,#070b12_0%,#0a1220_100%)]">
      <div className="border-b border-cyan-300/10 bg-slate-950/70 backdrop-blur-xl">
        <div className="max-w-6xl mx-auto px-4 py-4 flex items-center justify-between gap-4">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-cyan-400/10 border border-cyan-300/20 flex items-center justify-center shadow-[0_0_0_1px_rgba(95,233,255,0.04)]">
              <IconSettings className="w-5 h-5 text-cyan-200" />
            </div>
            <div>
              <h1 className="text-xl font-semibold tracking-wide">Console serveur</h1>
              <p className="text-sm text-slate-400">{server?.name ?? 'Serveur inconnu'}</p>
            </div>
          </div>
          <div className="text-right text-sm">
            <div className="text-slate-400">Temps réel</div>
            <div className={connected ? 'text-emerald-300' : 'text-amber-300'}>
              {connected ? 'Connecté' : 'Déconnecté'}
            </div>
          </div>
        </div>
      </div>

      <div className="max-w-6xl mx-auto px-4 py-6 space-y-6">
        {(error || apiError || rolesError) && (
          <div className="bg-rose-500/10 border border-rose-400/30 text-rose-100 rounded-xl px-4 py-3">
            {error || apiError || rolesError}
          </div>
        )}

        {notice && (
          <div className="bg-cyan-500/10 border border-cyan-400/30 text-cyan-50 rounded-xl px-4 py-3 flex items-center justify-between gap-4 shadow-[0_8px_24px_rgba(24,191,233,0.08)]">
            <span>{notice}</span>
            <button onClick={() => setNotice(null)} className="text-cyan-100 hover:text-white">Fermer</button>
          </div>
        )}

        <div className="bg-amber-400/10 border border-amber-300/30 text-amber-100 rounded-xl px-4 py-3">
          La console permissions est en bêta: matrix canal/rôle + overrides catégorie/membre passent par le protocole WS corrélé (`request_id` + `*_ack`).
        </div>

        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
          <StatCard icon={<IconUsers className="w-5 h-5" />} label="Membres" value={serverMembers.length} />
          <StatCard icon={<IconHash className="w-5 h-5" />} label="Canaux" value={channels.length} />
          <StatCard icon={<IconSettings className="w-5 h-5" />} label="Catégories" value={categories.length} />
          <StatCard icon={<IconShield className="w-5 h-5" />} label="Rôles" value={effectiveRoles.length} />
        </div>

        <div className="flex flex-wrap gap-2">
          <button
            className={`${tabClass} ${activeTab === 'overview' ? 'border-cyan-300/30 bg-cyan-400/15 text-cyan-100 shadow-[0_0_0_1px_rgba(95,233,255,0.08)]' : 'border-white/5 bg-slate-950/45 text-slate-300 hover:bg-slate-900/70'}`}
            onClick={() => setActiveTab('overview')}
          >
            Vue d'ensemble
          </button>
          <button
            className={`${tabClass} ${activeTab === 'channels' ? 'border-cyan-300/30 bg-cyan-400/15 text-cyan-100 shadow-[0_0_0_1px_rgba(95,233,255,0.08)]' : 'border-white/5 bg-slate-950/45 text-slate-300 hover:bg-slate-900/70'}`}
            onClick={() => setActiveTab('channels')}
          >
            Canaux / Catégories
          </button>
          <button
            className={`${tabClass} ${activeTab === 'roles' ? 'border-cyan-300/30 bg-cyan-400/15 text-cyan-100 shadow-[0_0_0_1px_rgba(95,233,255,0.08)]' : 'border-white/5 bg-slate-950/45 text-slate-300 hover:bg-slate-900/70'}`}
            onClick={() => setActiveTab('roles')}
          >
            Rôles / Membres
          </button>
          <button
            className={`${tabClass} ${activeTab === 'permissions' ? 'border-cyan-300/30 bg-cyan-400/15 text-cyan-100 shadow-[0_0_0_1px_rgba(95,233,255,0.08)]' : 'border-white/5 bg-slate-950/45 text-slate-300 hover:bg-slate-900/70'}`}
            onClick={() => setActiveTab('permissions')}
          >
            Permissions (beta)
          </button>
          <button
            className="ml-auto px-3 py-2 rounded-xl text-sm font-medium border border-white/5 bg-slate-950/45 text-slate-300 hover:bg-slate-900/70"
            onClick={refreshAll}
          >
            Rafraîchir
          </button>
        </div>

        {activeTab === 'overview' && (
          <OverviewTab
            server={server}
            serverId={serverId}
            userMemberRole={userMember?.role ?? (isOwner ? 'owner' : 'N/A')}
            connected={connected}
          />
        )}

        {activeTab === 'channels' && (
          <ChannelsTab
            serverId={serverId}
            categories={categories}
            categoryGroups={categoryGroups}
            uncategorizedChannels={uncategorizedChannels}
            loading={wsActionLoading}
            newCategoryName={newCategoryName}
            setNewCategoryName={setNewCategoryName}
            onCreateCategory={handleCreateCategory}
            newChannelName={newChannelName}
            setNewChannelName={setNewChannelName}
            newChannelType={newChannelType}
            setNewChannelType={setNewChannelType}
            newChannelCategoryId={newChannelCategoryId}
            setNewChannelCategoryId={setNewChannelCategoryId}
            newChannelDescription={newChannelDescription}
            setNewChannelDescription={setNewChannelDescription}
            onCreateChannel={handleCreateChannel}
            onRenameCategory={handleRenameCategory}
            onDeleteCategory={handleDeleteCategory}
            onMoveCategory={handleMoveCategory}
            onMoveCategoryToPosition={handleMoveCategoryToPosition}
            onReorderCategories={handleReorderCategories}
            onRenameChannel={handleRenameChannel}
            onDeleteChannel={handleDeleteChannel}
            onMoveChannel={handleMoveChannel}
            onMoveChannelToPosition={handleMoveChannelToPosition}
            onReorderChannels={handleReorderChannels}
          />
        )}

        {activeTab === 'roles' && (
          <RolesTab
            roles={effectiveRoles}
            membersWithRoles={memberRows}
            loading={rolesLoading}
            newRoleName={newRoleName}
            setNewRoleName={setNewRoleName}
            newRoleColor={newRoleColor}
            setNewRoleColor={setNewRoleColor}
            onCreateRole={handleCreateRole}
            onEditRole={handleEditRole}
            onDeleteRole={handleDeleteRole}
            memberRoleSelections={memberRoleSelections}
            setMemberRoleSelections={setMemberRoleSelections}
            onAssignRole={handleAssignRole}
            onRemoveRoleFromMember={handleRemoveRoleFromMember}
          />
        )}

        {activeTab === 'permissions' && (
          <PermissionsTab
            serverId={serverId}
            roles={effectiveRoles}
            channels={channels}
            categories={categories}
            members={serverMembers}
            loading={wsActionLoading}
            onSetChannelRolePermissions={handleSetChannelRolePermissions}
            onSetCategoryRolePermissions={handleSetCategoryRolePermissions}
            onSetMemberChannelPermissions={handleSetMemberChannelPermissions}
            onSetMemberCategoryPermissions={handleSetMemberCategoryPermissions}
            onGetEffectivePermissions={handleGetEffectivePermissions}
          />
        )}
      </div>
    </div>
  )
}

const OverviewTab: React.FC<{
  server: {
    id?: string
    owner_id?: string
    name?: string
    is_online?: boolean
    created_at?: string
    max_users?: number
    description?: string | null
  } | null
  serverId: string
  userMemberRole: string
  connected: boolean
}> = ({ server, serverId, userMemberRole, connected }) => (
  <div className="grid grid-cols-1 xl:grid-cols-2 gap-6">
    <section className={cardClass}>
      <h2 className="text-lg font-semibold mb-4">Informations générales</h2>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-sm">
        <InfoRow label="Nom" value={server?.name ?? 'N/A'} />
        <InfoRow label="ID" value={server?.id ?? serverId} mono />
        <InfoRow label="Propriétaire" value={server?.owner_id ?? 'N/A'} mono />
        <InfoRow label="Statut serveur" value={server?.is_online ? 'En ligne' : 'Hors ligne'} />
        <InfoRow label="Connexion WS" value={connected ? 'Connecté' : 'Déconnecté'} />
        <InfoRow label="Rôle courant" value={userMemberRole} />
        <InfoRow label="Max membres" value={String(server?.max_users ?? 'N/A')} />
        <InfoRow label="Créé le" value={server?.created_at ?? 'N/A'} />
      </div>
    </section>

    <section className={cardClass}>
      <h2 className="text-lg font-semibold mb-4">Description</h2>
      <p className="text-sm text-gray-300 whitespace-pre-wrap min-h-[120px]">
        {server?.description?.trim() || 'Aucune description.'}
      </p>
      <div className="mt-4 text-xs text-gray-500">
        Édition des paramètres généraux du serveur restera branchée sur REST/WS après stabilisation des contrats de settings.
      </div>
    </section>
  </div>
)

type ChannelRecord = {
  id: string
  name: string
  type: 'text' | 'voice'
  position: number
  category_id?: string | null
  description?: string | null
}

type CategoryRecord = {
  id: string
  name: string
  position: number
}

type CategoryGroupRecord = {
  category: CategoryRecord
  channels: ChannelRecord[]
}

const ChannelsTab: React.FC<{
  serverId: string
  categories: CategoryRecord[]
  categoryGroups: CategoryGroupRecord[]
  uncategorizedChannels: ChannelRecord[]
  loading: boolean
  newCategoryName: string
  setNewCategoryName: (value: string) => void
  onCreateCategory: () => Promise<void>
  newChannelName: string
  setNewChannelName: (value: string) => void
  newChannelType: 'text' | 'voice'
  setNewChannelType: (value: 'text' | 'voice') => void
  newChannelCategoryId: string
  setNewChannelCategoryId: (value: string) => void
  newChannelDescription: string
  setNewChannelDescription: (value: string) => void
  onCreateChannel: () => Promise<void>
  onRenameCategory: (categoryId: string, currentName: string) => Promise<void>
  onDeleteCategory: (categoryId: string) => Promise<void>
  onMoveCategory: (categoryId: string, direction: -1 | 1) => Promise<void>
  onMoveCategoryToPosition: (categoryId: string, position: number) => Promise<void>
  onReorderCategories: (orderedCategoryIds: string[]) => Promise<void>
  onRenameChannel: (channelId: string, currentName: string, currentDescription?: string | null) => Promise<void>
  onDeleteChannel: (channelId: string) => Promise<void>
  onMoveChannel: (channelId: string, direction: -1 | 1) => Promise<void>
  onMoveChannelToPosition: (channelId: string, position: number, categoryId?: string | null) => Promise<void>
  onReorderChannels: (categoryId: string | null, orderedChannelIds: string[]) => Promise<void>
}> = ({
  serverId,
  categories,
  categoryGroups,
  uncategorizedChannels,
  loading,
  newCategoryName,
  setNewCategoryName,
  onCreateCategory,
  newChannelName,
  setNewChannelName,
  newChannelType,
  setNewChannelType,
  newChannelCategoryId,
  setNewChannelCategoryId,
  newChannelDescription,
  setNewChannelDescription,
  onCreateChannel,
  onRenameCategory,
  onDeleteCategory,
  onMoveCategory,
  onMoveCategoryToPosition,
  onReorderCategories,
  onRenameChannel,
  onDeleteChannel,
  onMoveChannel,
  onMoveChannelToPosition,
  onReorderChannels,
}) => (
  <div className="grid grid-cols-1 xl:grid-cols-3 gap-6">
    <section className={`${cardClass} xl:col-span-1 space-y-6`}>
      <div>
        <h2 className="text-lg font-semibold mb-3">Créer une catégorie</h2>
        <div className="space-y-2">
          <input
            value={newCategoryName}
            onChange={e => setNewCategoryName(e.target.value)}
            placeholder="Nom de la catégorie"
            className="w-full rounded-lg border border-gray-700 bg-gray-900 px-3 py-2 text-sm"
          />
          <button
            type="button"
            onClick={() => void onCreateCategory()}
            disabled={loading || !newCategoryName.trim()}
            className="w-full rounded-lg bg-blue-600 px-3 py-2 text-sm font-medium disabled:opacity-50"
          >
            {loading ? 'Envoi...' : 'Créer la catégorie'}
          </button>
        </div>
      </div>

      <div className="border-t border-gray-700 pt-5">
        <h2 className="text-lg font-semibold mb-3">Créer un canal</h2>
        <div className="space-y-2">
          <input
            value={newChannelName}
            onChange={e => setNewChannelName(e.target.value)}
            placeholder="Nom du canal"
            className="w-full rounded-lg border border-gray-700 bg-gray-900 px-3 py-2 text-sm"
          />
          <select
            value={newChannelType}
            onChange={e => setNewChannelType(e.target.value as 'text' | 'voice')}
            className="w-full rounded-lg border border-gray-700 bg-gray-900 px-3 py-2 text-sm"
          >
            <option value="text">Texte</option>
            <option value="voice">Vocal</option>
          </select>
          <select
            value={newChannelCategoryId}
            onChange={e => setNewChannelCategoryId(e.target.value)}
            className="w-full rounded-lg border border-gray-700 bg-gray-900 px-3 py-2 text-sm"
          >
            <option value="">Sans catégorie</option>
            {categories
              .slice()
              .sort((a, b) => a.position - b.position)
              .map(category => (
                <option key={category.id} value={category.id}>{category.name}</option>
              ))}
          </select>
          <textarea
            value={newChannelDescription}
            onChange={e => setNewChannelDescription(e.target.value)}
            placeholder="Description (optionnel)"
            rows={3}
            className="w-full rounded-lg border border-gray-700 bg-gray-900 px-3 py-2 text-sm"
          />
          <button
            type="button"
            onClick={() => void onCreateChannel()}
            disabled={loading || !newChannelName.trim()}
            className="w-full rounded-lg bg-blue-600 px-3 py-2 text-sm font-medium disabled:opacity-50"
          >
            {loading ? 'Envoi...' : 'Créer le canal'}
          </button>
        </div>
      </div>

      <div className="border-t border-gray-700 pt-5 text-xs text-gray-400">
        Réorganisation par glisser-déposer (batch WS `reorder_channels` / `reorder_categories`).
        Les déplacements inter-catégories utilisent un fallback `move_channel`.
      </div>
    </section>

    <section className={`${cardClass} xl:col-span-2`}>
      <div className="flex items-center justify-between gap-4 mb-4">
        <h2 className="text-lg font-semibold">Structure actuelle</h2>
        <div className="text-xs text-gray-400">
          Serveur: <span className="font-mono">{serverId}</span>
        </div>
      </div>
      <div className="max-h-[700px] overflow-auto pr-2">
        <DraggableChannelList
          channels={[...categoryGroups.flatMap(group => group.channels), ...uncategorizedChannels]}
          categories={categories}
          canManageChannels
          onReorderCategories={orderedCategoryIds => void onReorderCategories(orderedCategoryIds)}
          onReorderChannels={(categoryId, orderedChannelIds) => void onReorderChannels(categoryId, orderedChannelIds)}
          onChannelMove={(channelId, newPosition, categoryId) => {
            // Fallback pour les déplacements inter-catégories (batch scope séparé non encore fusionné)
            void onMoveChannelToPosition(channelId, newPosition, categoryId ?? null)
          }}
          onCategoryMove={(categoryId, newPosition) => {
            void onMoveCategoryToPosition(categoryId, newPosition)
          }}
          onChannelUpdate={channel => void onRenameChannel(channel.id, channel.name, channel.description)}
          onChannelDelete={channelId => void onDeleteChannel(channelId)}
          onCategoryUpdate={category => void onRenameCategory(category.id, category.name)}
          onCategoryDelete={categoryId => void onDeleteCategory(categoryId)}
        />
      </div>
    </section>
  </div>
)

type RoleRecord = {
  id: string
  name: string
  color: string
  position: number
  permissions_bitfield: number
  is_default: boolean
}

type MemberWithRolesRecord = {
  member: {
    id: string
    user_id: string
    username: string
    is_online: boolean
  }
  roles: Array<{
    id: string
    name: string
    color: string
    is_default: boolean
  }>
}

const RolesTab: React.FC<{
  roles: RoleRecord[]
  membersWithRoles: MemberWithRolesRecord[]
  loading: boolean
  newRoleName: string
  setNewRoleName: (value: string) => void
  newRoleColor: string
  setNewRoleColor: (value: string) => void
  onCreateRole: () => Promise<void>
  onEditRole: (roleId: string, currentName: string, currentColor: string) => Promise<void>
  onDeleteRole: (roleId: string, isDefault: boolean) => Promise<void>
  memberRoleSelections: Record<string, string>
  setMemberRoleSelections: React.Dispatch<React.SetStateAction<Record<string, string>>>
  onAssignRole: (memberUserId: string) => Promise<void>
  onRemoveRoleFromMember: (memberUserId: string, roleId: string) => Promise<void>
}> = ({
  roles,
  membersWithRoles,
  loading,
  newRoleName,
  setNewRoleName,
  newRoleColor,
  setNewRoleColor,
  onCreateRole,
  onEditRole,
  onDeleteRole,
  memberRoleSelections,
  setMemberRoleSelections,
  onAssignRole,
  onRemoveRoleFromMember,
}) => (
  <div className="grid grid-cols-1 xl:grid-cols-3 gap-6">
    <section className={`${cardClass} xl:col-span-1 space-y-6`}>
      <div>
        <h2 className="text-lg font-semibold mb-3">Créer un rôle</h2>
        <div className="space-y-2">
          <input
            value={newRoleName}
            onChange={e => setNewRoleName(e.target.value)}
            placeholder="Nom du rôle"
            className="w-full rounded-lg border border-gray-700 bg-gray-900 px-3 py-2 text-sm"
          />
          <div className="flex items-center gap-2">
            <input
              type="color"
              value={newRoleColor}
              onChange={e => setNewRoleColor(e.target.value)}
              className="h-10 w-12 rounded border border-gray-700 bg-gray-900"
            />
            <input
              value={newRoleColor}
              onChange={e => setNewRoleColor(e.target.value)}
              placeholder="#99AAB5"
              className="flex-1 rounded-lg border border-gray-700 bg-gray-900 px-3 py-2 text-sm"
            />
          </div>
          <button
            type="button"
            onClick={() => void onCreateRole()}
            disabled={loading || !newRoleName.trim()}
            className="w-full rounded-lg bg-blue-600 px-3 py-2 text-sm font-medium disabled:opacity-50"
          >
            {loading ? 'Chargement...' : 'Créer le rôle'}
          </button>
        </div>
      </div>

      <div className="border-t border-gray-700 pt-5 text-xs text-gray-400 space-y-2">
        <p>Les permissions de rôle fines et la matrix d&apos;overrides seront réactivées après généralisation des réponses WS corrélées.</p>
        <p>Les CRUD rôles et assignations membres sont déjà opérationnels via le backend REST.</p>
      </div>
    </section>

    <section className={`${cardClass} xl:col-span-2`}>
      <h2 className="text-lg font-semibold mb-4">Rôles</h2>
      {roles.length === 0 ? (
        <p className="text-sm text-gray-400 mb-6">Aucun rôle chargé.</p>
      ) : (
        <div className="space-y-3 mb-6">
          {roles
            .slice()
            .sort((a, b) => b.position - a.position)
            .map(role => (
              <div key={role.id} className="border border-gray-700 rounded-lg p-3 bg-gray-900/40">
                <div className="flex items-center justify-between gap-3">
                  <div className="flex items-center gap-3 min-w-0">
                    <span className="w-3 h-3 rounded-full" style={{ backgroundColor: role.color || '#99AAB5' }} />
                    <div className="min-w-0">
                      <div className="text-sm font-medium truncate">{role.name}</div>
                      <div className="text-xs text-gray-500">pos {role.position} • perm {role.permissions_bitfield}</div>
                    </div>
                    {role.is_default && (
                      <span className="text-[11px] px-2 py-1 rounded bg-blue-500/20 text-blue-200 border border-blue-500/30">Défaut</span>
                    )}
                  </div>
                  <div className="flex gap-2 text-xs">
                    <button className="px-2 py-1 rounded bg-gray-700 hover:bg-gray-600" onClick={() => void onEditRole(role.id, role.name, role.color)}>Éditer</button>
                    <button
                      className="px-2 py-1 rounded bg-red-600/80 hover:bg-red-600 disabled:opacity-50"
                      onClick={() => void onDeleteRole(role.id, role.is_default)}
                      disabled={role.is_default}
                    >
                      Supprimer
                    </button>
                  </div>
                </div>
              </div>
            ))}
        </div>
      )}

      <h2 className="text-lg font-semibold mb-4">Assignations membres</h2>
      {membersWithRoles.length === 0 ? (
        <p className="text-sm text-gray-400">Aucun membre chargé.</p>
      ) : (
        <div className="space-y-3 max-h-[600px] overflow-auto pr-2">
          {membersWithRoles.map(entry => {
            const assignedRoleIds = new Set(entry.roles.map(role => role.id))
            const addableRoles = roles.filter(role => !assignedRoleIds.has(role.id))
            const selectedRole = memberRoleSelections[entry.member.user_id] ?? ''

            return (
              <div key={entry.member.id} className="border border-gray-700 rounded-lg p-3 bg-gray-900/40">
                <div className="flex items-center justify-between gap-4 mb-3">
                  <div className="flex items-center gap-2 min-w-0">
                    <span className={`w-2 h-2 rounded-full ${entry.member.is_online ? 'bg-green-400' : 'bg-gray-500'}`} />
                    <span className="text-sm font-medium truncate">{entry.member.username}</span>
                    <span className="text-xs text-gray-500 font-mono">{entry.member.user_id}</span>
                  </div>
                  <div className="flex items-center gap-2">
                    <select
                      value={selectedRole}
                      onChange={e => setMemberRoleSelections(prev => ({ ...prev, [entry.member.user_id]: e.target.value }))}
                      className="rounded-lg border border-gray-700 bg-gray-900 px-2 py-1 text-xs"
                    >
                      <option value="">Ajouter un rôle...</option>
                      {addableRoles.map(role => (
                        <option key={role.id} value={role.id}>{role.name}</option>
                      ))}
                    </select>
                    <button
                      className="px-2 py-1 rounded bg-blue-600 text-xs disabled:opacity-50"
                      disabled={!selectedRole || loading}
                      onClick={() => void onAssignRole(entry.member.user_id)}
                    >
                      Assigner
                    </button>
                  </div>
                </div>

                <div className="flex flex-wrap gap-2">
                  {entry.roles.length === 0 ? (
                    <span className="text-xs text-gray-500">Aucun rôle</span>
                  ) : (
                    entry.roles.map(role => (
                      <span
                        key={role.id}
                        className="inline-flex items-center gap-2 text-xs rounded-full border border-gray-700 bg-gray-800/80 px-2 py-1"
                      >
                        <span className="w-2 h-2 rounded-full" style={{ backgroundColor: role.color || '#99AAB5' }} />
                        <span>{role.name}</span>
                        {!role.is_default && (
                          <button
                            className="text-red-300 hover:text-red-200"
                            onClick={() => void onRemoveRoleFromMember(entry.member.user_id, role.id)}
                            title="Retirer le rôle"
                          >
                            x
                          </button>
                        )}
                      </span>
                    ))
                  )}
                </div>
              </div>
            )
          })}
        </div>
      )}
    </section>
  </div>
)

const PermissionsTab: React.FC<{
  serverId: string
  roles: RoleRecord[]
  channels: ChannelRecord[]
  categories: CategoryRecord[]
  members: Array<{
    id: string
    user_id: string
    username: string
    is_online: boolean
  }>
  loading: boolean
  onSetChannelRolePermissions: (channelId: string, roleId: string, allow: number, deny: number) => Promise<void>
  onSetCategoryRolePermissions: (categoryId: string, roleId: string, allow: number, deny: number) => Promise<void>
  onSetMemberChannelPermissions: (channelId: string, memberId: string, allow: number, deny: number) => Promise<void>
  onSetMemberCategoryPermissions: (categoryId: string, memberId: string, allow: number, deny: number) => Promise<void>
  onGetEffectivePermissions: (
    targetUserId: string,
    channelId?: string,
    categoryId?: string
  ) => Promise<{
    allow_bitfield: number
    deny_bitfield: number
    final_permissions: number
    channel_id?: string
    category_id?: string
  }>
}> = ({
  serverId,
  roles,
  channels,
  categories,
  members,
  loading,
  onSetChannelRolePermissions,
  onSetCategoryRolePermissions,
  onSetMemberChannelPermissions,
  onSetMemberCategoryPermissions,
  onGetEffectivePermissions,
}) => {
  const matrixChannels = channels.map(channel => ({
    id: channel.id,
    name: channel.name,
    type: channel.type,
    category_id: channel.category_id ?? undefined,
  }))

  const matrixCategories = categories.map(category => ({
    id: category.id,
    name: category.name,
  }))

  const [categoryRoleForm, setCategoryRoleForm] = useState({
    categoryId: '',
    roleId: '',
    allow: '0',
    deny: '0',
  })
  const [memberChannelForm, setMemberChannelForm] = useState({
    channelId: '',
    memberId: '',
    allow: '0',
    deny: '0',
  })
  const [memberCategoryForm, setMemberCategoryForm] = useState({
    categoryId: '',
    memberId: '',
    allow: '0',
    deny: '0',
  })
  const [effectiveForm, setEffectiveForm] = useState({
    userId: '',
    channelId: '',
    categoryId: '',
  })
  const [effectiveResult, setEffectiveResult] = useState<{
    allow_bitfield: number
    deny_bitfield: number
    final_permissions: number
    channel_id?: string
    category_id?: string
  } | null>(null)
  const [advancedError, setAdvancedError] = useState<string | null>(null)

  const parseBitfield = (value: string) => {
    const parsed = Number.parseInt(value, 10)
    return Number.isFinite(parsed) ? parsed : 0
  }

  const roleOptions = roles.slice().sort((a, b) => b.position - a.position)
  const categoryOptions = categories.slice().sort((a, b) => a.position - b.position)
  const channelOptions = channels.slice().sort((a, b) => a.position - b.position)
  const memberOptions = members.slice().sort((a, b) => a.username.localeCompare(b.username))

  return (
    <div className="space-y-6">
      <section className={cardClass}>
        <h2 className="text-lg font-semibold mb-2">Permissions avancées (bêta)</h2>
        <p className="text-sm text-gray-300">
          Matrix canal/rôle avec sauvegarde WebSocket corrélée (`request_id` + `set_channel_permissions_ack`).
        </p>
        <p className="text-xs text-gray-500 mt-2">
          Les overrides de catégorie et de membre sont supportés par le protocole, mais l&apos;UI dédiée sera ajoutée ensuite.
        </p>
        <p className="text-xs text-gray-500 mt-1">
          Le calcul des permissions effectives via WS est limité à l&apos;utilisateur connecté (contrôle d&apos;identité côté serveur).
        </p>
        {loading && <p className="text-xs text-blue-300 mt-2">Une action WebSocket est en cours...</p>}
      </section>

      <PermissionMatrix
        serverId={serverId}
        roles={roles}
        channels={matrixChannels}
        categories={matrixCategories}
        onSetChannelPermissions={onSetChannelRolePermissions}
      />

      <div className="grid grid-cols-1 xl:grid-cols-2 gap-6">
        <section className={cardClass}>
          <h3 className="text-base font-semibold mb-3">Override rôle sur catégorie</h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            <select
              value={categoryRoleForm.categoryId}
              onChange={e => setCategoryRoleForm(prev => ({ ...prev, categoryId: e.target.value }))}
              className="rounded-lg border border-gray-700 bg-gray-900 px-3 py-2 text-sm"
            >
              <option value="">Catégorie</option>
              {categoryOptions.map(category => (
                <option key={category.id} value={category.id}>{category.name}</option>
              ))}
            </select>
            <select
              value={categoryRoleForm.roleId}
              onChange={e => setCategoryRoleForm(prev => ({ ...prev, roleId: e.target.value }))}
              className="rounded-lg border border-gray-700 bg-gray-900 px-3 py-2 text-sm"
            >
              <option value="">Rôle</option>
              {roleOptions.map(role => (
                <option key={role.id} value={role.id}>{role.name}</option>
              ))}
            </select>
            <input
              type="number"
              value={categoryRoleForm.allow}
              onChange={e => setCategoryRoleForm(prev => ({ ...prev, allow: e.target.value }))}
              className="rounded-lg border border-gray-700 bg-gray-900 px-3 py-2 text-sm"
              placeholder="allow_bitfield"
            />
            <input
              type="number"
              value={categoryRoleForm.deny}
              onChange={e => setCategoryRoleForm(prev => ({ ...prev, deny: e.target.value }))}
              className="rounded-lg border border-gray-700 bg-gray-900 px-3 py-2 text-sm"
              placeholder="deny_bitfield"
            />
          </div>
          <button
            type="button"
            disabled={loading || !categoryRoleForm.categoryId || !categoryRoleForm.roleId}
            className="mt-3 rounded-lg bg-blue-600 px-3 py-2 text-sm font-medium disabled:opacity-50"
            onClick={() => {
              setAdvancedError(null)
              void onSetCategoryRolePermissions(
                categoryRoleForm.categoryId,
                categoryRoleForm.roleId,
                parseBitfield(categoryRoleForm.allow),
                parseBitfield(categoryRoleForm.deny)
              ).catch(err => setAdvancedError(err instanceof Error ? err.message : 'Erreur'))
            }}
          >
            Appliquer override catégorie/rôle
          </button>
        </section>

        <section className={cardClass}>
          <h3 className="text-base font-semibold mb-3">Override membre sur canal</h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            <select
              value={memberChannelForm.channelId}
              onChange={e => setMemberChannelForm(prev => ({ ...prev, channelId: e.target.value }))}
              className="rounded-lg border border-gray-700 bg-gray-900 px-3 py-2 text-sm"
            >
              <option value="">Canal</option>
              {channelOptions.map(channel => (
                <option key={channel.id} value={channel.id}>
                  {channel.type === 'voice' ? '🔊' : '#'} {channel.name}
                </option>
              ))}
            </select>
            <select
              value={memberChannelForm.memberId}
              onChange={e => setMemberChannelForm(prev => ({ ...prev, memberId: e.target.value }))}
              className="rounded-lg border border-gray-700 bg-gray-900 px-3 py-2 text-sm"
            >
              <option value="">Membre</option>
              {memberOptions.map(member => (
                <option key={member.id} value={member.id}>{member.username}</option>
              ))}
            </select>
            <input
              type="number"
              value={memberChannelForm.allow}
              onChange={e => setMemberChannelForm(prev => ({ ...prev, allow: e.target.value }))}
              className="rounded-lg border border-gray-700 bg-gray-900 px-3 py-2 text-sm"
            />
            <input
              type="number"
              value={memberChannelForm.deny}
              onChange={e => setMemberChannelForm(prev => ({ ...prev, deny: e.target.value }))}
              className="rounded-lg border border-gray-700 bg-gray-900 px-3 py-2 text-sm"
            />
          </div>
          <button
            type="button"
            disabled={loading || !memberChannelForm.channelId || !memberChannelForm.memberId}
            className="mt-3 rounded-lg bg-blue-600 px-3 py-2 text-sm font-medium disabled:opacity-50"
            onClick={() => {
              setAdvancedError(null)
              void onSetMemberChannelPermissions(
                memberChannelForm.channelId,
                memberChannelForm.memberId,
                parseBitfield(memberChannelForm.allow),
                parseBitfield(memberChannelForm.deny)
              ).catch(err => setAdvancedError(err instanceof Error ? err.message : 'Erreur'))
            }}
          >
            Appliquer override membre/canal
          </button>
        </section>

        <section className={cardClass}>
          <h3 className="text-base font-semibold mb-3">Override membre sur catégorie</h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            <select
              value={memberCategoryForm.categoryId}
              onChange={e => setMemberCategoryForm(prev => ({ ...prev, categoryId: e.target.value }))}
              className="rounded-lg border border-gray-700 bg-gray-900 px-3 py-2 text-sm"
            >
              <option value="">Catégorie</option>
              {categoryOptions.map(category => (
                <option key={category.id} value={category.id}>{category.name}</option>
              ))}
            </select>
            <select
              value={memberCategoryForm.memberId}
              onChange={e => setMemberCategoryForm(prev => ({ ...prev, memberId: e.target.value }))}
              className="rounded-lg border border-gray-700 bg-gray-900 px-3 py-2 text-sm"
            >
              <option value="">Membre</option>
              {memberOptions.map(member => (
                <option key={member.id} value={member.id}>{member.username}</option>
              ))}
            </select>
            <input
              type="number"
              value={memberCategoryForm.allow}
              onChange={e => setMemberCategoryForm(prev => ({ ...prev, allow: e.target.value }))}
              className="rounded-lg border border-gray-700 bg-gray-900 px-3 py-2 text-sm"
            />
            <input
              type="number"
              value={memberCategoryForm.deny}
              onChange={e => setMemberCategoryForm(prev => ({ ...prev, deny: e.target.value }))}
              className="rounded-lg border border-gray-700 bg-gray-900 px-3 py-2 text-sm"
            />
          </div>
          <button
            type="button"
            disabled={loading || !memberCategoryForm.categoryId || !memberCategoryForm.memberId}
            className="mt-3 rounded-lg bg-blue-600 px-3 py-2 text-sm font-medium disabled:opacity-50"
            onClick={() => {
              setAdvancedError(null)
              void onSetMemberCategoryPermissions(
                memberCategoryForm.categoryId,
                memberCategoryForm.memberId,
                parseBitfield(memberCategoryForm.allow),
                parseBitfield(memberCategoryForm.deny)
              ).catch(err => setAdvancedError(err instanceof Error ? err.message : 'Erreur'))
            }}
          >
            Appliquer override membre/catégorie
          </button>
        </section>

        <section className={cardClass}>
          <h3 className="text-base font-semibold mb-3">Vérifier permissions effectives</h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            <select
              value={effectiveForm.userId}
              onChange={e => setEffectiveForm(prev => ({ ...prev, userId: e.target.value }))}
              className="rounded-lg border border-gray-700 bg-gray-900 px-3 py-2 text-sm"
            >
              <option value="">Utilisateur (user_id)</option>
              {memberOptions.map(member => (
                <option key={member.id} value={member.user_id}>
                  {member.username} ({member.user_id})
                </option>
              ))}
            </select>
            <select
              value={effectiveForm.channelId}
              onChange={e =>
                setEffectiveForm(prev => ({
                  ...prev,
                  channelId: e.target.value,
                  categoryId: e.target.value ? '' : prev.categoryId,
                }))
              }
              className="rounded-lg border border-gray-700 bg-gray-900 px-3 py-2 text-sm"
            >
              <option value="">Canal (optionnel)</option>
              {channelOptions.map(channel => (
                <option key={channel.id} value={channel.id}>
                  {channel.type === 'voice' ? '🔊' : '#'} {channel.name}
                </option>
              ))}
            </select>
            <select
              value={effectiveForm.categoryId}
              onChange={e =>
                setEffectiveForm(prev => ({
                  ...prev,
                  categoryId: e.target.value,
                  channelId: e.target.value ? '' : prev.channelId,
                }))
              }
              className="rounded-lg border border-gray-700 bg-gray-900 px-3 py-2 text-sm"
            >
              <option value="">Catégorie (optionnel)</option>
              {categoryOptions.map(category => (
                <option key={category.id} value={category.id}>{category.name}</option>
              ))}
            </select>
            <button
              type="button"
              disabled={loading || !effectiveForm.userId || (!effectiveForm.channelId && !effectiveForm.categoryId)}
              className="rounded-lg bg-blue-600 px-3 py-2 text-sm font-medium disabled:opacity-50"
              onClick={() => {
                setAdvancedError(null)
                void onGetEffectivePermissions(
                  effectiveForm.userId,
                  effectiveForm.channelId || undefined,
                  effectiveForm.categoryId || undefined
                )
                  .then(setEffectiveResult)
                  .catch(err => setAdvancedError(err instanceof Error ? err.message : 'Erreur'))
              }}
            >
              Calculer
            </button>
          </div>

          {effectiveResult && (
            <div className="mt-4 grid grid-cols-1 md:grid-cols-3 gap-3 text-sm">
              <InfoRow label="Allow" value={String(effectiveResult.allow_bitfield)} mono />
              <InfoRow label="Deny" value={String(effectiveResult.deny_bitfield)} mono />
              <InfoRow label="Final" value={String(effectiveResult.final_permissions)} mono />
            </div>
          )}
        </section>
      </div>

      {advancedError && (
        <div className="bg-red-500/10 border border-red-500/30 text-red-200 rounded-lg px-4 py-3 text-sm">
          {advancedError}
        </div>
      )}
    </div>
  )
}

const StatCard: React.FC<{ icon: React.ReactNode; label: string; value: number }> = ({ icon, label, value }) => {
  return (
    <div className={cardClass}>
      <div className="flex items-center justify-between mb-2 text-slate-300">
        <span>{label}</span>
        {icon}
      </div>
      <div className="text-2xl font-semibold">{value}</div>
    </div>
  )
}

const InfoRow: React.FC<{ label: string; value: string; mono?: boolean }> = ({ label, value, mono = false }) => {
  return (
    <div className="border border-cyan-300/10 rounded-xl p-3 bg-slate-950/40">
      <div className="text-slate-400 text-xs mb-1">{label}</div>
      <div className={mono ? 'font-mono text-xs break-all text-slate-200' : 'text-slate-200'}>{value}</div>
    </div>
  )
}

export default ServerSettings

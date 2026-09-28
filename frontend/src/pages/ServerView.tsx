import React, { useMemo, useState } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import * as Dialog from '@radix-ui/react-dialog'
import * as Tabs from '@radix-ui/react-tabs'
import * as Tooltip from '@radix-ui/react-tooltip'
import { IconBell, IconChevronDown, IconChevronRight, IconHash, IconLogout, IconMicrophone, IconMicrophoneOff, IconMoodSmile, IconPlus, IconSearch, IconSettings, IconUserPlus, IconUsersGroup, IconVolume, IconVolume2, IconVolumeOff } from '@tabler/icons-react'

import { useAuth } from '../hooks/useAuth'
import { useServerRealtime } from '../contexts/ServerRealtimeContext'
import {
  type CategoryWithChannels,
  type Category,
  type Channel as ServerChannel,
  type ServerMember,
  type VoiceParticipant,
  type ChatMessage,
  type ServerInfo,
} from '../hooks/useWebSocket'
import { Button } from '../components/ui/button'
import { Avatar, AvatarFallback } from '../components/ui/avatar'
import { ScrollArea } from '../components/ui/scroll-area'
import { Separator } from '../components/ui/separator'

import { cn } from '../lib/utils'

type ChannelFormState = {
  name: string
  type: 'text' | 'voice'
  category_id: string | 'uncategorized'
  description?: string
}

const ServerView: React.FC = () => {
  const { serverId } = useParams<{ serverId: string }>()
  const navigate = useNavigate()
  const { user } = useAuth()

  const {
    connected,
    error,
    server,
    categories,
    categoryGroups,
    uncategorizedChannels,
    channels,
    serverMembers,
    messages,
    currentChannel,
    currentChannelId,
    voiceParticipants,
    activeVoiceChannelId,
    joinChannel,
    sendChannelMessage,
    createChannel,
    createCategory,
    joinVoiceChannel,
    leaveVoiceChannel,
    setVoiceAudioEnabled,
    requestServerStructure,
  } = useServerRealtime()

  const [channelDialogOpen, setChannelDialogOpen] = useState(false)
  const [categoryDialogOpen, setCategoryDialogOpen] = useState(false)
  const [expandedCategories, setExpandedCategories] = useState<string[]>([])
  const [channelForm, setChannelForm] = useState<ChannelFormState>({
    name: '',
    type: 'text',
    category_id: 'uncategorized',
    description: '',
  })
  const [categoryName, setCategoryName] = useState('')
  const [messageDraft, setMessageDraft] = useState('')
  const [isSending, setIsSending] = useState(false)
  const [isMutingAudio, setIsMutingAudio] = useState(false)
  const [channelDialogPending, setChannelDialogPending] = useState(false)
  const [categoryDialogPending, setCategoryDialogPending] = useState(false)
  const [channelDialogError, setChannelDialogError] = useState<string | null>(null)
  const [categoryDialogError, setCategoryDialogError] = useState<string | null>(null)

  const onlineMembers = useMemo(
    () => serverMembers.filter(member => member.is_online),
    [serverMembers]
  )
  const offlineMembers = useMemo(
    () => serverMembers.filter(member => !member.is_online),
    [serverMembers]
  )

  const voiceChannels = useMemo(
    () => channels.filter(channel => channel.type === 'voice'),
    [channels]
  )

  const handleToggleCategory = (categoryId: string) => {
    setExpandedCategories(prev =>
      prev.includes(categoryId)
        ? prev.filter(id => id !== categoryId)
        : [...prev, categoryId]
    )
  }

  const handleSendMessage = async () => {
    const trimmed = messageDraft.trim()
    if (!trimmed || !currentChannelId) return

    try {
      setIsSending(true)
      sendChannelMessage(trimmed)
      setMessageDraft('')
    } finally {
      setIsSending(false)
    }
  }

  const handleCreateChannel = async (event: React.FormEvent) => {
    event.preventDefault()
    if (!channelForm.name.trim()) return
    setChannelDialogError(null)
    setChannelDialogPending(true)

    try {
      await createChannel({
        name: channelForm.name.trim(),
        type: channelForm.type,
        category_id: channelForm.category_id === 'uncategorized' ? undefined : channelForm.category_id,
        description: channelForm.description?.trim() || undefined,
      })
      setChannelDialogOpen(false)
      setChannelForm({ name: '', type: 'text', category_id: 'uncategorized', description: '' })
    } catch (err) {
      setChannelDialogError(err instanceof Error ? err.message : 'Impossible de créer le salon')
    } finally {
      setChannelDialogPending(false)
    }
  }

  const handleCreateCategory = async (event: React.FormEvent) => {
    event.preventDefault()
    const trimmed = categoryName.trim()
    if (!trimmed) return
    setCategoryDialogError(null)
    setCategoryDialogPending(true)

    try {
      await createCategory(trimmed)
      setCategoryDialogOpen(false)
      setCategoryName('')
    } catch (err) {
      setCategoryDialogError(err instanceof Error ? err.message : 'Impossible de créer la catégorie')
    } finally {
      setCategoryDialogPending(false)
    }
  }

  const handleVoiceJoin = (channelId: string) => {
    if (activeVoiceChannelId === channelId) {
      leaveVoiceChannel(channelId)
    } else {
      joinVoiceChannel(channelId)
    }
  }

  const toggleAudio = () => {
    const isCurrentlyMuted = voiceParticipants.some(participant => participant.user_id === user?.id && participant.is_muted)
    setIsMutingAudio(true)
    setVoiceAudioEnabled(!isCurrentlyMuted)
    setTimeout(() => setIsMutingAudio(false), 150)
  }

  if (!connected) {
    return (
      <div className="h-screen bg-[#1f2226] flex items-center justify-center">
        <div className="text-center space-y-4">
          <div className="w-16 h-16 mx-auto border-4 border-cyan-300/20 border-t-cyan-300 rounded-full animate-spin" />
          <div className="space-y-1">
            <p className="text-white font-medium text-lg">Connexion au serveur</p>
            <p className="text-sm text-white/60">Merci de patienter...</p>
            {error && <p className="text-sm text-red-400">{error}</p>}
          </div>
        </div>
      </div>
    )
  }

  return (
    <Tooltip.Provider delayDuration={150}>
      <div className="h-full w-full bg-[#0b1220] text-white grid grid-cols-[84px_320px_1fr_300px] overflow-hidden">
        <ServerRail name={server?.name} onRefresh={requestServerStructure} />

        <ChannelSidebar
          serverName={server?.name}
          categoryGroups={categoryGroups}
          uncategorizedChannels={uncategorizedChannels}
          expandedCategories={expandedCategories}
          onToggleCategory={handleToggleCategory}
          currentChannelId={currentChannelId}
          onSelectChannel={joinChannel}
          onOpenChannelDialog={() => setChannelDialogOpen(true)}
          onOpenCategoryDialog={() => setCategoryDialogOpen(true)}
          voiceChannels={voiceChannels}
          activeVoiceChannelId={activeVoiceChannelId}
          onJoinVoice={handleVoiceJoin}
          onOpenSettings={() => navigate(`/server/${serverId}/settings`)}
        />

        <MainChatPane
          server={server}
          currentChannel={currentChannel}
          messages={messages}
          messageDraft={messageDraft}
          setMessageDraft={setMessageDraft}
          onSendMessage={handleSendMessage}
          isSending={isSending}
        />

        <MemberSidebar
          onlineMembers={onlineMembers}
          offlineMembers={offlineMembers}
          voiceParticipants={voiceParticipants}
          voiceChannels={voiceChannels}
          activeVoiceChannelId={activeVoiceChannelId}
          onJoinVoice={handleVoiceJoin}
          onLeaveVoice={() => leaveVoiceChannel()}
          onToggleAudio={toggleAudio}
          isMutingAudio={isMutingAudio}
          currentUserId={user?.id ?? null}
        />
      </div>

      <ChannelDialog
        open={channelDialogOpen}
        onOpenChange={setChannelDialogOpen}
        categories={categories}
        form={channelForm}
        onFormChange={setChannelForm}
          onSubmit={handleCreateChannel}
        pending={channelDialogPending}
        error={channelDialogError}
      />

      <CategoryDialog
        open={categoryDialogOpen}
        onOpenChange={setCategoryDialogOpen}
        categoryName={categoryName}
        onCategoryNameChange={setCategoryName}
        onSubmit={handleCreateCategory}
        pending={categoryDialogPending}
        error={categoryDialogError}
      />
    </Tooltip.Provider>
  )
}

interface ServerRailProps {
  name?: string
  onRefresh: () => void
}

const ServerRail: React.FC<ServerRailProps> = ({ name, onRefresh }) => {
  const initials = name?.slice(0, 2).toUpperCase() || 'DS'

  return (
    <div className="bg-[#08101b] flex flex-col items-center py-4 space-y-3 border-r border-cyan-400/10">
      <Tooltip.Root>
        <Tooltip.Trigger asChild>
          <div className="w-12 h-12 rounded-2xl bg-gradient-to-br from-cyan-400 to-blue-500 flex items-center justify-center text-base font-semibold shadow-lg shadow-cyan-500/30 transition-all duration-200 hover:scale-105">
            {initials}
          </div>
        </Tooltip.Trigger>
        <Tooltip.Content side="right" className="rounded bg-black/80 px-2 py-1 text-xs">
          {name || 'Serveur DiSpeak'}
        </Tooltip.Content>
      </Tooltip.Root>

      <Separator className="w-8 bg-white/5" />

      <Tooltip.Root>
        <Tooltip.Trigger asChild>
          <button
            onClick={onRefresh}
            className="w-12 h-12 rounded-2xl bg-white/5 flex items-center justify-center text-xl text-white/80 transition hover:bg-white/10"
          >
            ↻
          </button>
        </Tooltip.Trigger>
        <Tooltip.Content side="right" className="rounded bg-black/80 px-2 py-1 text-xs">
          Rafraîchir la structure
        </Tooltip.Content>
      </Tooltip.Root>

    </div>
  )
}

interface ChannelSidebarProps {
  serverName?: string
  categoryGroups: CategoryWithChannels[]
  uncategorizedChannels: ServerChannel[]
  expandedCategories: string[]
  onToggleCategory: (categoryId: string) => void
  currentChannelId: string | null
  onSelectChannel: (channelId: string) => void
  onOpenChannelDialog: () => void
  onOpenCategoryDialog: () => void
  voiceChannels: ServerChannel[]
  activeVoiceChannelId: string | null
  onJoinVoice: (channelId: string) => void
  onOpenSettings: () => void
}

const ChannelSidebar: React.FC<ChannelSidebarProps> = ({
  serverName,
  categoryGroups,
  uncategorizedChannels,
  expandedCategories,
  onToggleCategory,
  currentChannelId,
  onSelectChannel,
  onOpenChannelDialog,
  onOpenCategoryDialog,
  voiceChannels,
  activeVoiceChannelId,
  onJoinVoice,
  onOpenSettings,
}) => {
  return (
    <div className="bg-[#121b2b] border-r border-cyan-400/10 flex flex-col h-full overflow-hidden">
      <div className="h-14 px-4 flex items-center justify-between border-b border-white/5">
        <div className="space-y-0.5">
          <p className="text-sm font-semibold text-white truncate">{serverName || 'Serveur'}</p>
          <p className="text-xs text-cyan-100/60">Hub tactique en direct</p>
        </div>
        <Button variant="ghost" size="icon" className="text-white/70 hover:text-white" onClick={onOpenCategoryDialog}>
          <IconPlus className="w-4 h-4" />
        </Button>
      </div>

      <ScrollArea className="flex-1 px-3 py-4 space-y-6">
        <section className="space-y-2">
          <SectionHeader title="Salons textuels" onAdd={onOpenChannelDialog} />

          {categoryGroups.map(group => (
            <div key={group.category.id} className="space-y-1">
              <button
                className="flex w-full items-center justify-between text-[11px] uppercase tracking-[0.2em] text-white/40 hover:text-white/70"
                onClick={() => onToggleCategory(group.category.id)}
              >
                <span>{group.category.name}</span>
                {expandedCategories.includes(group.category.id) ? (
                  <IconChevronDown className="w-3 h-3" />
                ) : (
                  <IconChevronRight className="w-3 h-3" />
                )}
              </button>

              <div className={cn('space-y-1 overflow-hidden transition-all', expandedCategories.includes(group.category.id) ? 'max-h-96' : 'max-h-0')}>
                {group.channels.map(channel => (
                  <ChannelButton
                    key={channel.id}
                    channel={channel}
                    isActive={currentChannelId === channel.id}
                    onSelect={onSelectChannel}
                  />
                ))}
              </div>
            </div>
          ))}

          {uncategorizedChannels.length > 0 && (
            <div className="space-y-1">
              <p className="text-[11px] uppercase tracking-[0.2em] text-white/40">Sans catégorie</p>
              {uncategorizedChannels.map(channel => (
                <ChannelButton
                  key={channel.id}
                  channel={channel}
                  isActive={currentChannelId === channel.id}
                  onSelect={onSelectChannel}
                />
              ))}
            </div>
          )}
        </section>

        <section className="space-y-2">
          <SectionHeader title="Salons vocaux" />
          {voiceChannels.map(channel => (
            <button
              key={channel.id}
              onClick={() => onJoinVoice(channel.id)}
              className={cn(
                'w-full flex items-center gap-2 rounded px-2 py-2 text-sm text-white/70 hover:bg-white/5 transition',
                activeVoiceChannelId === channel.id && 'bg-white/10 text-white shadow-inner'
              )}
            >
              <IconVolume className="w-4 h-4" />
              <span className="truncate">{channel.name}</span>
            </button>
          ))}
          {voiceChannels.length === 0 && (
            <p className="text-xs text-white/40">Aucun salon vocal pour l'instant.</p>
          )}
        </section>
      </ScrollArea>

      <div className="border-t border-cyan-400/10 px-3 py-3">
        <Button
          variant="ghost"
          size="sm"
          className="w-full justify-between text-white/80 hover:text-cyan-200"
          onClick={onOpenSettings}
        >
          Gérer le serveur
          <IconSettings className="w-4 h-4" />
        </Button>
      </div>
    </div>
  )
}

interface ChannelButtonProps {
  channel: ServerChannel
  isActive: boolean
  onSelect: (channelId: string) => void
}

const ChannelButton: React.FC<ChannelButtonProps> = ({ channel, isActive, onSelect }) => {
  return (
    <button
      onClick={() => onSelect(channel.id)}
      className={cn(
        'group w-full flex items-center gap-2 rounded px-2 py-2 text-sm transition',
        isActive ? 'bg-white/10 text-white shadow-inner' : 'text-white/60 hover:bg-white/5 hover:text-white'
      )}
    >
      <IconHash className="w-4 h-4 text-white/30 group-hover:text-white/50" />
      <span className="truncate">{channel.name}</span>
    </button>
  )
}

interface SectionHeaderProps {
  title: string
  onAdd?: () => void
}

const SectionHeader: React.FC<SectionHeaderProps> = ({ title, onAdd }) => (
  <div className="flex items-center justify-between">
    <span className="text-[11px] uppercase tracking-[0.2em] text-white/40">{title}</span>
    {onAdd && (
      <Button variant="ghost" size="icon" className="h-6 w-6 text-white/70 hover:text-white" onClick={onAdd}>
        <IconPlus className="w-3 h-3" />
      </Button>
    )}
  </div>
)

interface MainChatPaneProps {
  server: ServerInfo | null
  currentChannel: ServerChannel | null
  messages: ChatMessage[]
  messageDraft: string
  setMessageDraft: (value: string) => void
  onSendMessage: () => void
  isSending: boolean
}

const MainChatPane: React.FC<MainChatPaneProps> = ({
  server,
  currentChannel,
  messages,
  messageDraft,
  setMessageDraft,
  onSendMessage,
  isSending,
}) => {
  const formatTimestamp = (timestamp: string) => {
    try {
      return new Date(timestamp).toLocaleTimeString(undefined, {
        hour: '2-digit',
        minute: '2-digit',
      })
    } catch (err) {
      return timestamp
    }
  }

  return (
    <div className="flex flex-col bg-[#121a28] h-full overflow-hidden">
      <div className="h-14 border-b border-cyan-400/10 px-5 flex items-center justify-between">
        <div className="space-y-0.5">
          <div className="flex items-center gap-2">
            <IconHash className="w-4 h-4 text-white/40" />
            <h2 className="text-sm font-semibold">{currentChannel?.name || 'Choisir un salon'}</h2>
          </div>
          <p className="text-xs text-white/40 truncate">
            {currentChannel?.description || server?.description || 'Bienvenue sur DiSpeak'}
          </p>
        </div>

        <div className="text-xs text-cyan-100/60">Temps réel sécurisé</div>
      </div>

      <ScrollArea className="flex-1 overflow-y-auto">
        <div className="flex flex-col gap-3 px-6 py-6">
          {currentChannel ? (
            messages.length > 0 ? (
              messages.map(message => (
                <div key={message.id} className="flex gap-3 rounded-lg px-3 py-2 hover:bg-white/5 transition">
                  <Avatar className="h-10 w-10">
                    <AvatarFallback className="bg-gradient-to-br from-cyan-400 to-blue-500 text-white text-sm font-semibold">
                      {message.username?.slice(0, 2).toUpperCase() || '??'}
                    </AvatarFallback>
                  </Avatar>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-baseline gap-2">
                      <span className="text-sm font-semibold text-white">{message.username}</span>
                      <span className="text-xs text-white/40">{formatTimestamp(message.created_at)}</span>
                    </div>
                    <p className="text-sm leading-relaxed text-white/80 whitespace-pre-wrap break-words">{message.content}</p>
                  </div>
                </div>
              ))
            ) : (
              <div className="flex flex-col items-center justify-center py-24 text-center space-y-2">
                <div className="w-16 h-16 rounded-full bg-white/5 flex items-center justify-center">
                  <IconHash className="w-6 h-6 text-white/30" />
                </div>
                <p className="text-lg font-semibold text-white">Bienvenue dans #{currentChannel.name}</p>
                <p className="text-sm text-white/50">Commencez la discussion en envoyant un message.</p>
              </div>
            )
          ) : (
            <div className="flex flex-col items-center justify-center py-32 text-center space-y-2">
              <div className="w-20 h-20 rounded-full bg-white/5 flex items-center justify-center">
                <IconHash className="w-8 h-8 text-white/40" />
              </div>
              <p className="text-lg font-semibold text-white">Sélectionnez un salon</p>
              <p className="text-sm text-white/50">Choisissez un canal pour commencer à discuter.</p>
            </div>
          )}
        </div>
      </ScrollArea>

      <div className="border-t border-white/5 px-5 py-4">
        <div className="relative">
          <textarea
            value={messageDraft}
            onChange={event => setMessageDraft(event.target.value)}
            onKeyDown={event => {
              if (event.key === 'Enter' && !event.shiftKey) {
                event.preventDefault()
                onSendMessage()
              }
            }}
            disabled={!currentChannel}
            placeholder={currentChannel ? `Message #${currentChannel.name}` : 'Sélectionnez un canal pour écrire'}
            className="w-full min-h-[56px] max-h-[160px] resize-none rounded-2xl bg-[#172235] px-4 py-3 text-sm text-white placeholder:text-white/40 focus:outline-none focus:ring-2 focus:ring-cyan-300/60 disabled:cursor-not-allowed disabled:opacity-50"
          />
          <div className="absolute right-3 bottom-3 flex items-center gap-2">
            <Button variant="ghost" size="icon" className="text-white/40 hover:text-white">
              <IconMoodSmile className="w-4 h-4" />
            </Button>
            <Button
              variant="default"
              size="sm"
              disabled={!currentChannel || !messageDraft.trim() || isSending}
              onClick={onSendMessage}
              className="bg-cyan-500 hover:bg-cyan-400 text-slate-950"
            >
              Envoyer
            </Button>
          </div>
        </div>
      </div>
    </div>
  )
}

interface MemberSidebarProps {
  onlineMembers: ServerMember[]
  offlineMembers: ServerMember[]
  voiceParticipants: VoiceParticipant[]
  voiceChannels: ServerChannel[]
  activeVoiceChannelId: string | null
  onJoinVoice: (channelId: string) => void
  onLeaveVoice: () => void
  onToggleAudio: () => void
  isMutingAudio: boolean
  currentUserId: string | null
}

const MemberSidebar: React.FC<MemberSidebarProps> = ({
  onlineMembers,
  offlineMembers,
  voiceParticipants,
  voiceChannels,
  activeVoiceChannelId,
  onJoinVoice,
  onLeaveVoice,
  onToggleAudio,
  isMutingAudio,
  currentUserId,
}) => {
  const currentVoiceRoomName = useMemo(() => {
    if (!activeVoiceChannelId) return null
    return voiceChannels.find(channel => channel.id === activeVoiceChannelId)?.name ?? null
  }, [activeVoiceChannelId, voiceChannels])

  const currentUserParticipant = voiceParticipants.find(participant => participant.user_id === currentUserId)

  return (
    <div className="bg-[#101722] border-l border-cyan-300/10 flex flex-col h-full overflow-hidden">
      <div className="h-14 flex items-center justify-between px-4 border-b border-white/5">
        <p className="text-sm font-semibold">Membres</p>
        <Button variant="ghost" size="icon" className="text-white/60 hover:text-white">
          <IconSearch className="w-4 h-4" />
        </Button>
      </div>

      <ScrollArea className="flex-1 px-3 py-4 space-y-6">
        <MemberSection title={`Connectés — ${onlineMembers.length}`}>
          {onlineMembers.map(member => (
            <MemberItem key={member.id} member={member} isOnline />
          ))}
          {onlineMembers.length === 0 && <EmptyState message="Personne n'est en ligne pour le moment." />}
        </MemberSection>

        <MemberSection title={`Déconnectés — ${offlineMembers.length}`}>
          {offlineMembers.map(member => (
            <MemberItem key={member.id} member={member} />
          ))}
        </MemberSection>

        <MemberSection title="Salon vocal">
          {activeVoiceChannelId ? (
            <div className="space-y-2 rounded-lg border border-white/10 bg-white/5 p-3">
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm font-semibold">{currentVoiceRoomName || 'Salon vocal'}</p>
                  <p className="text-xs text-white/40">{voiceParticipants.length} participant(s)</p>
                </div>
                <div className="flex items-center gap-2">
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={onToggleAudio}
                    disabled={isMutingAudio}
                    className={cn(
                      'text-white/70 hover:text-white',
                      currentUserParticipant?.is_muted && 'text-red-400 hover:text-red-200'
                    )}
                  >
                    {currentUserParticipant?.is_muted ? (
                      <IconMicrophoneOff className="w-4 h-4" />
                    ) : (
                      <IconMicrophone className="w-4 h-4" />
                    )}
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={onLeaveVoice}
                    className="text-white/70 hover:text-red-400"
                  >
                    <IconLogout className="w-4 h-4" />
                  </Button>
                </div>
              </div>

              <div className="space-y-2">
                {voiceParticipants.map(participant => (
                  <div
                    key={participant.user_id}
                    className="flex items-center justify-between rounded px-2 py-1.5 bg-black/10"
                  >
                    <div className="flex items-center gap-2">
                      <Avatar className="h-7 w-7">
                        <AvatarFallback className="bg-gradient-to-br from-cyan-400 to-sky-600 text-white text-xs">
                          {participant.username.slice(0, 2).toUpperCase()}
                        </AvatarFallback>
                      </Avatar>
                      <span className="text-sm text-white/80">{participant.username}</span>
                    </div>
                    <div className="flex items-center gap-2 text-white/50">
                      {participant.is_muted ? (
                        <IconMicrophoneOff className="w-4 h-4" />
                      ) : (
                        <IconMicrophone className="w-4 h-4" />
                      )}
                      {participant.is_deafened ? (
                        <IconVolumeOff className="w-4 h-4" />
                      ) : (
                        <IconVolume2 className="w-4 h-4" />
                      )}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          ) : (
            <div className="space-y-2">
              {voiceChannels.length > 0 ? (
                voiceChannels.map(channel => (
                  <button
                    key={channel.id}
                    onClick={() => onJoinVoice(channel.id)}
                    className="w-full rounded-lg border border-dashed border-white/10 px-3 py-2 text-left text-sm text-white/60 hover:border-white/30 hover:text-white"
                  >
                    Rejoindre {channel.name}
                  </button>
                ))
              ) : (
                <EmptyState message="Ajoutez un salon vocal pour commencer à discuter." />
              )}
            </div>
          )}
        </MemberSection>
      </ScrollArea>

      <div className="border-t border-white/5 px-4 py-3 flex items-center justify-between bg-black/20">
        <div className="space-y-0.5">
          <p className="text-xs text-white/40">Statut vocal</p>
          <p className="text-sm font-medium">
            {activeVoiceChannelId ? currentVoiceRoomName || 'Salon vocal' : 'Déconnecté'}
          </p>
        </div>
        <div className="flex items-center gap-2">
          {activeVoiceChannelId && (
            <Tooltip.Root>
              <Tooltip.Trigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  className="text-white/60 hover:text-red-400"
                  onClick={onLeaveVoice}
                  disabled={isMutingAudio}
                >
                  <IconVolumeOff className="w-4 h-4" />
                </Button>
              </Tooltip.Trigger>
              <Tooltip.Content className="rounded bg-black/80 px-2 py-1 text-xs">
                Quitter le salon vocal
              </Tooltip.Content>
            </Tooltip.Root>
          )}
        </div>
      </div>
    </div>
  )
}

interface MemberSectionProps {
  title: string
  children: React.ReactNode
}

const MemberSection: React.FC<MemberSectionProps> = ({ title, children }) => (
  <div className="space-y-2">
    <p className="text-[11px] uppercase tracking-[0.2em] text-white/40">{title}</p>
    <div className="space-y-1.5">{children}</div>
  </div>
)

interface MemberItemProps {
  member: ServerMember
  isOnline?: boolean
}

const MemberItem: React.FC<MemberItemProps> = ({ member, isOnline }) => (
  <div className="flex items-center gap-3 rounded-lg px-2 py-1.5 hover:bg-white/5 transition">
    <div className="relative">
      <Avatar className="h-8 w-8">
        <AvatarFallback className="bg-gradient-to-br from-cyan-400 to-sky-600 text-white text-xs">
          {member.username.slice(0, 2).toUpperCase()}
        </AvatarFallback>
      </Avatar>
      <span
        className={cn(
          'absolute -bottom-0.5 -right-0.5 h-2.5 w-2.5 rounded-full border border-[#101722]',
          isOnline ? 'bg-emerald-400' : 'bg-white/20'
        )}
      />
    </div>
    <div className="flex-1 min-w-0">
      <p className="text-sm text-white/80 truncate">{member.username}</p>
      <p className="text-xs text-white/40 capitalize">{member.role}</p>
    </div>
  </div>
)

const EmptyState: React.FC<{ message: string }> = ({ message }) => (
  <p className="text-xs text-white/40">{message}</p>
)

interface ChannelDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  categories: Category[]
  form: ChannelFormState
  onFormChange: (value: ChannelFormState) => void
  onSubmit: (event: React.FormEvent) => void
  pending: boolean
  error: string | null
}

const ChannelDialog: React.FC<ChannelDialogProps> = ({ open, onOpenChange, categories, form, onFormChange, onSubmit, pending, error }) => (
  <Dialog.Root open={open} onOpenChange={onOpenChange}>
    <Dialog.Portal>
      <Dialog.Overlay className="fixed inset-0 bg-black/70 backdrop-blur-sm" />
      <Dialog.Content className="fixed left-1/2 top-1/2 w-[420px] -translate-x-1/2 -translate-y-1/2 rounded-xl border border-cyan-400/15 bg-[#0d1726] p-6 shadow-2xl shadow-cyan-900/30">
        <Dialog.Title className="text-lg font-semibold text-white">Créer un salon</Dialog.Title>
        <Dialog.Description className="text-sm text-white/50 mt-1">
          Choisissez le type de salon et configurez ses paramètres.
        </Dialog.Description>

        <Tabs.Root
          value={form.type}
          onValueChange={value => onFormChange({ ...form, type: value as 'text' | 'voice' })}
          className="mt-4"
        >
          <Tabs.List className="inline-flex rounded-full bg-white/10 p-1 text-xs">
            <Tabs.Trigger
              value="text"
              className={cn(
                'rounded-full px-4 py-1 transition',
                form.type === 'text' ? 'bg-cyan-500 text-slate-950 shadow' : 'text-white/60 hover:text-white'
              )}
            >
              Texte
            </Tabs.Trigger>
            <Tabs.Trigger
              value="voice"
              className={cn(
                'rounded-full px-4 py-1 transition',
                form.type === 'voice' ? 'bg-cyan-500 text-slate-950 shadow' : 'text-white/60 hover:text-white'
              )}
            >
              Vocal
            </Tabs.Trigger>
          </Tabs.List>
        </Tabs.Root>

        <form onSubmit={onSubmit} className="mt-5 space-y-4">
          <div className="space-y-2">
            <label className="text-xs font-medium uppercase tracking-wide text-white/60">Nom du salon</label>
            <input
              value={form.name}
              onChange={event => onFormChange({ ...form, name: event.target.value })}
              className="w-full rounded-lg border border-white/10 bg-[#162235] px-3 py-2 text-sm text-white focus:border-cyan-400 focus:outline-none"
              placeholder="général"
              autoFocus
            />
          </div>

          <div className="space-y-2">
            <label className="text-xs font-medium uppercase tracking-wide text-white/60">Catégorie</label>
            <select
              value={form.category_id}
              onChange={event => onFormChange({ ...form, category_id: event.target.value })}
              className="w-full rounded-lg border border-white/10 bg-[#162235] px-3 py-2 text-sm text-white focus:border-cyan-400 focus:outline-none"
            >
              <option value="uncategorized">Sans catégorie</option>
              {categories.map(category => (
                <option key={category.id} value={category.id}>
                  {category.name}
                </option>
              ))}
            </select>
          </div>

          {form.type === 'text' && (
            <div className="space-y-2">
              <label className="text-xs font-medium uppercase tracking-wide text-white/60">Description</label>
              <textarea
                value={form.description}
                onChange={event => onFormChange({ ...form, description: event.target.value })}
                className="w-full rounded-lg border border-white/10 bg-[#162235] px-3 py-2 text-sm text-white focus:border-cyan-400 focus:outline-none"
                placeholder="Expliquez le but de ce salon"
                rows={3}
              />
            </div>
          )}

          {error && <p className="text-sm text-rose-300">{error}</p>}
          <div className="flex justify-end gap-2 pt-2">
            <Button type="button" variant="ghost" onClick={() => onOpenChange(false)} className="text-white/70 hover:text-white">
              Annuler
            </Button>
            <Button type="submit" className="bg-cyan-500 hover:bg-cyan-400 text-slate-950" disabled={pending}>
              {pending ? 'Création...' : 'Créer'}
            </Button>
          </div>
        </form>
      </Dialog.Content>
    </Dialog.Portal>
  </Dialog.Root>
)

interface CategoryDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  categoryName: string
  onCategoryNameChange: (name: string) => void
  onSubmit: (event: React.FormEvent) => void
  pending: boolean
  error: string | null
}

const CategoryDialog: React.FC<CategoryDialogProps> = ({ open, onOpenChange, categoryName, onCategoryNameChange, onSubmit, pending, error }) => (
  <Dialog.Root open={open} onOpenChange={onOpenChange}>
    <Dialog.Portal>
      <Dialog.Overlay className="fixed inset-0 bg-black/70 backdrop-blur-sm" />
      <Dialog.Content className="fixed left-1/2 top-1/2 w-[360px] -translate-x-1/2 -translate-y-1/2 rounded-xl border border-cyan-400/15 bg-[#0d1726] p-6 shadow-2xl shadow-cyan-900/30">
        <Dialog.Title className="text-lg font-semibold text-white">Nouvelle catégorie</Dialog.Title>
        <Dialog.Description className="text-sm text-white/50 mt-1">
          Organisez vos salons en créant une catégorie.
        </Dialog.Description>

        <form onSubmit={onSubmit} className="mt-5 space-y-4">
          <div className="space-y-2">
            <label className="text-xs font-medium uppercase tracking-wide text-white/60">Nom</label>
            <input
              value={categoryName}
              onChange={event => onCategoryNameChange(event.target.value)}
              className="w-full rounded-lg border border-white/10 bg-[#162235] px-3 py-2 text-sm text-white focus:border-cyan-400 focus:outline-none"
              placeholder="Catégorie"
              autoFocus
            />
          </div>

          {error && <p className="text-sm text-rose-300">{error}</p>}
          <div className="flex justify-end gap-2 pt-2">
            <Button type="button" variant="ghost" onClick={() => onOpenChange(false)} className="text-white/70 hover:text-white">
              Annuler
            </Button>
            <Button type="submit" className="bg-cyan-500 hover:bg-cyan-400 text-slate-950" disabled={pending}>
              {pending ? 'Création...' : 'Créer la catégorie'}
            </Button>
          </div>
        </form>
      </Dialog.Content>
    </Dialog.Portal>
  </Dialog.Root>
)

export default ServerView

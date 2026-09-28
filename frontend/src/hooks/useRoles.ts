import { useState, useEffect, useCallback } from 'react';
import { api } from '../lib/api';
import { useWebSocket } from './useWebSocket';
import { useAuth } from './useAuth';

// Constantes pour les permissions (bitfield)
export const PERMISSIONS = {
  VIEW_CHANNELS: 1,
  SEND_MESSAGES: 2,
  MANAGE_MESSAGES: 4,
  MANAGE_CHANNELS: 8,
  MANAGE_SERVER: 16,
  MANAGE_ROLES: 32,
  CONNECT_VOICE: 64,
  SPEAK_VOICE: 128,
  MUTE_MEMBERS: 256,
  DEAFEN_MEMBERS: 512,
  MOVE_MEMBERS: 1024,
  ADMINISTRATOR: 2147483647, // Toutes permissions
} as const;

export interface ServerRole {
  id: string;
  server_id: string;
  name: string;
  color: string;
  position: number;
  permissions_bitfield: number;
  is_default: boolean;
  created_at: string;
  members_count?: number;
}

export interface CreateRole {
  name: string;
  color?: string;
  permissions_bitfield?: number;
  position?: number;
}

export interface UpdateRole {
  name?: string;
  color?: string;
  permissions_bitfield?: number;
  position?: number;
}

export interface ServerMemberWithRoles {
  member: {
    id: string;
    server_id: string;
    user_id: string;
    username: string;
    avatar?: string;
    role: string;
    joined_at: string;
    last_seen: string;
    is_online: boolean;
  };
  roles: ServerRole[];
}

export interface EffectivePermissions {
  allow_bitfield: number;
  deny_bitfield: number;
  final_permissions: number;
}

export interface PermissionOverride {
  allow_bitfield: number;
  deny_bitfield: number;
}

export interface ChannelPermissionOverride {
  channel_id: string;
  role_id: string;
  allow_bitfield: number;
  deny_bitfield: number;
}

export interface CategoryPermissionOverride {
  category_id: string;
  role_id: string;
  allow_bitfield: number;
  deny_bitfield: number;
}

export interface MemberChannelOverride {
  channel_id: string;
  member_id: string;
  allow_bitfield: number;
  deny_bitfield: number;
}

export interface MemberCategoryOverride {
  category_id: string;
  member_id: string;
  allow_bitfield: number;
  deny_bitfield: number;
}

interface UseRolesOptions {
  webSocketServerId?: string;
}

export const useRoles = (serverId?: string, options?: UseRolesOptions) => {
  const [roles, setRoles] = useState<ServerRole[]>([]);
  const [membersWithRoles, setMembersWithRoles] = useState<ServerMemberWithRoles[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const wsServerId = options && 'webSocketServerId' in options ? options.webSocketServerId : serverId;
  const { sendCommand, request } = useWebSocket(wsServerId);
  const { user } = useAuth();

  const fetchRoles = useCallback(async (id: string) => {
    try {
      setLoading(true);
      setError(null);
      const rolesData = await api.get<ServerRole[]>(`/servers/${id}/roles`);
      setRoles(rolesData);
    } catch (err: any) {
      setError(err.message || 'Erreur lors du chargement des rôles');
    } finally {
      setLoading(false);
    }
  }, []);

  const fetchMembersWithRoles = useCallback(async (id: string) => {
    try {
      const membersData = await api.get<ServerMemberWithRoles[]>(`/servers/${id}/members-with-roles`);
      setMembersWithRoles(membersData);
    } catch (err: any) {
      setError(err.message || 'Erreur lors du chargement des membres avec rôles');
    }
  }, []);

  const createRole = useCallback(async (id: string, roleData: CreateRole) => {
    try {
      const newRole = await api.post<ServerRole>(`/servers/${id}/roles`, roleData);
      setRoles(prev => [...prev, newRole].sort((a, b) => b.position - a.position));
      return newRole;
    } catch (err: any) {
      setError(err.message || 'Erreur lors de la création du rôle');
      throw err;
    }
  }, []);

  const updateRole = useCallback(async (id: string, roleId: string, roleData: UpdateRole) => {
    try {
      const updatedRole = await api.put<ServerRole>(`/servers/${id}/roles/${roleId}`, roleData);
      setRoles(prev => prev.map(role => 
        role.id === roleId ? updatedRole : role
      ).sort((a, b) => b.position - a.position));
      return updatedRole;
    } catch (err: any) {
      setError(err.message || 'Erreur lors de la mise à jour du rôle');
      throw err;
    }
  }, []);

  const deleteRole = useCallback(async (id: string, roleId: string) => {
    try {
      await api.delete(`/servers/${id}/roles/${roleId}`);
      setRoles(prev => prev.filter(role => role.id !== roleId));
    } catch (err: any) {
      setError(err.message || 'Erreur lors de la suppression du rôle');
      throw err;
    }
  }, []);

  const assignRoles = useCallback(async (id: string, userId: string, roleIds: string[]) => {
    try {
      await api.post(`/servers/${id}/members/${userId}/roles`, { role_ids: roleIds });
      await fetchMembersWithRoles(id);
    } catch (err: any) {
      setError(err.message || 'Erreur lors de l\'assignation des rôles');
      throw err;
    }
  }, [fetchMembersWithRoles]);

  const removeRoleFromMember = useCallback(async (id: string, userId: string, roleId: string) => {
    try {
      const member = membersWithRoles.find(m => m.member.user_id === userId);
      if (member) {
        const remainingRoleIds = member.roles
          .filter(role => role.id !== roleId)
          .map(role => role.id);
        await assignRoles(id, userId, remainingRoleIds);
      }
    } catch (err: any) {
      setError(err.message || 'Erreur lors de la suppression du rôle');
      throw err;
    }
  }, [membersWithRoles, assignRoles]);

  // Fonctions utilitaires pour les permissions
  const hasPermission = useCallback((permissions: number, permission: keyof typeof PERMISSIONS) => {
    return (permissions & PERMISSIONS[permission]) !== 0;
  }, []);

  const combinePermissions = useCallback((permissionsList: number[]) => {
    return permissionsList.reduce((combined, permissions) => combined | permissions, 0);
  }, []);

  const getPermissionNames = useCallback((permissions: number) => {
    const names: string[] = [];
    Object.entries(PERMISSIONS).forEach(([name, value]) => {
      if (name !== 'ADMINISTRATOR' && (permissions & value) !== 0) {
        names.push(name);
      }
    });
    if (permissions === PERMISSIONS.ADMINISTRATOR) {
      names.push('ADMINISTRATOR');
    }
    return names;
  }, []);

  // Gestion des permissions de rôles
  const updateRolePermissions = useCallback(async (roleId: string, permissions: number) => {
    if (!serverId || !user?.id) {
      setError('Utilisateur non connecté ou serveur non spécifié');
      return;
    }

    try {
      setLoading(true);
      setError(null);
      
      await request({
        action: 'update_role',
        data: {
          role_id: roleId,
          permissions_bitfield: permissions,
          user_id: user.id,
        },
        expectedActions: ['role_update_ack'],
        timeoutMs: 5000,
      });
    } catch (err: any) {
      setError(err.message || 'Erreur lors de la mise à jour des permissions');
      throw err;
    } finally {
      setLoading(false);
    }
  }, [serverId, user?.id, request]);

  // Gestion des overrides de permissions par canal
  const setChannelPermissions = useCallback(async (channelId: string, roleId: string, allow: number, deny: number) => {
    if (!user?.id) {
      setError('Utilisateur non connecté');
      return;
    }

    try {
      setLoading(true);
      setError(null);
      
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
      });
    } catch (err: any) {
      setError(err.message || 'Erreur lors de la définition des permissions du canal');
      throw err;
    } finally {
      setLoading(false);
    }
  }, [user?.id, request]);

  // Gestion des overrides de permissions par catégorie
  const setCategoryPermissions = useCallback(async (categoryId: string, roleId: string, allow: number, deny: number) => {
    if (!user?.id) {
      setError('Utilisateur non connecté');
      return;
    }

    try {
      setLoading(true);
      setError(null);
      
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
      });
    } catch (err: any) {
      setError(err.message || 'Erreur lors de la définition des permissions de la catégorie');
      throw err;
    } finally {
      setLoading(false);
    }
  }, [user?.id, request]);

  // Gestion des overrides de permissions par membre pour un canal
  const setMemberChannelPermissions = useCallback(async (channelId: string, memberId: string, allow: number, deny: number) => {
    if (!user?.id) {
      setError('Utilisateur non connecté');
      return;
    }

    try {
      setLoading(true);
      setError(null);

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
      });
    } catch (err: any) {
      setError(err.message || 'Erreur lors de la définition des permissions du membre pour le canal');
      throw err;
    } finally {
      setLoading(false);
    }
  }, [user?.id, request]);

  // Gestion des overrides de permissions par membre pour une catégorie
  const setMemberCategoryPermissions = useCallback(async (categoryId: string, memberId: string, allow: number, deny: number) => {
    if (!user?.id) {
      setError('Utilisateur non connecté');
      return;
    }

    try {
      setLoading(true);
      setError(null);

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
      });
    } catch (err: any) {
      setError(err.message || 'Erreur lors de la définition des permissions du membre pour la catégorie');
      throw err;
    } finally {
      setLoading(false);
    }
  }, [user?.id, request]);

  // Récupération des permissions effectives
  const getEffectivePermissions = useCallback(async (channelId?: string, categoryId?: string): Promise<EffectivePermissions | null> => {
    if (!user?.id) {
      setError('Utilisateur non connecté');
      return null;
    }

    if (!channelId && !categoryId) {
      setError('Channel ID ou Category ID requis');
      return null;
    }

    try {
      setLoading(true);
      setError(null);
      
      const response = await request({
        action: 'get_effective_permissions',
        data: {
          channel_id: channelId,
          category_id: categoryId,
          user_id: user.id,
        },
        expectedActions: ['effective_permissions'],
        timeoutMs: 5000,
      });

      return response as EffectivePermissions;
    } catch (err: any) {
      setError(err.message || 'Erreur lors de la récupération des permissions effectives');
      return null;
    } finally {
      setLoading(false);
    }
  }, [user?.id, request]);

  // Fonctions utilitaires pour les overrides
  const calculateOverridePermissions = useCallback((basePermissions: number, allow: number, deny: number) => {
    // Appliquer les overrides : (base | allow) & ~deny
    return (basePermissions | allow) & ~deny;
  }, []);

  const hasOverridePermission = useCallback((permissions: number, permission: keyof typeof PERMISSIONS) => {
    return (permissions & PERMISSIONS[permission]) !== 0;
  }, []);

  const getOverridePermissionNames = useCallback((permissions: number) => {
    const names: string[] = [];
    Object.entries(PERMISSIONS).forEach(([name, value]) => {
      if (name !== 'ADMINISTRATOR' && (permissions & value) !== 0) {
        names.push(name);
      }
    });
    if (permissions === PERMISSIONS.ADMINISTRATOR) {
      names.push('ADMINISTRATOR');
    }
    return names;
  }, []);

  useEffect(() => {
    if (serverId) {
      fetchRoles(serverId);
      fetchMembersWithRoles(serverId);
    }
  }, [serverId, fetchRoles, fetchMembersWithRoles]);

  return {
    roles,
    membersWithRoles,
    loading,
    error,
    setError,
    fetchRoles,
    fetchMembersWithRoles,
    createRole,
    updateRole,
    deleteRole,
    assignRoles,
    removeRoleFromMember,
    // Backward-compatible aliases used by older screens.
    assignRole: (memberId: string, roleId: string) =>
      serverId ? assignRoles(serverId, memberId, [roleId]) : Promise.resolve(),
    removeRole: (memberId: string, roleId: string) =>
      serverId ? removeRoleFromMember(serverId, memberId, roleId) : Promise.resolve(),
    hasPermission,
    combinePermissions,
    getPermissionNames,
    PERMISSIONS,
    // Nouvelles fonctions pour les permissions et overrides
    updateRolePermissions,
    setChannelPermissions,
    setCategoryPermissions,
    setMemberChannelPermissions,
    setMemberCategoryPermissions,
    getEffectivePermissions,
    calculateOverridePermissions,
    hasOverridePermission,
    getOverridePermissionNames,
  };
};


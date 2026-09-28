import { useState, useCallback } from 'react';
import { api } from '../lib/api';
import { PERMISSIONS } from './useRoles';

export interface ChannelPermission {
  channel_id: string;
  role_id: string;
  permissions_bitfield: number;
}

export interface CategoryPermission {
  category_id: string;
  role_id: string;
  permissions_bitfield: number;
}

export const usePermissions = () => {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchChannelPermissions = useCallback(async (channelId: string) => {
    try {
      setLoading(true);
      setError(null);
      const permissions = await api.get<ChannelPermission[]>(`/channels/${channelId}/permissions`);
      return permissions;
    } catch (err: any) {
      setError(err.message || 'Erreur lors du chargement des permissions du canal');
      throw err;
    } finally {
      setLoading(false);
    }
  }, []);

  const updateChannelPermission = useCallback(async (
    channelId: string, 
    roleId: string, 
    permissions: number
  ) => {
    try {
      await api.put(`/channels/${channelId}/permissions/${roleId}`, {
        permissions_bitfield: permissions
      });
    } catch (err: any) {
      setError(err.message || 'Erreur lors de la mise à jour des permissions');
      throw err;
    }
  }, []);

  const fetchCategoryPermissions = useCallback(async (categoryId: string) => {
    try {
      setLoading(true);
      setError(null);
      const permissions = await api.get<CategoryPermission[]>(`/categories/${categoryId}/permissions`);
      return permissions;
    } catch (err: any) {
      setError(err.message || 'Erreur lors du chargement des permissions de la catégorie');
      throw err;
    } finally {
      setLoading(false);
    }
  }, []);

  const updateCategoryPermission = useCallback(async (
    categoryId: string, 
    roleId: string, 
    permissions: number
  ) => {
    try {
      await api.put(`/categories/${categoryId}/permissions/${roleId}`, {
        permissions_bitfield: permissions
      });
    } catch (err: any) {
      setError(err.message || 'Erreur lors de la mise à jour des permissions');
      throw err;
    }
  }, []);

  // Fonctions utilitaires pour les permissions
  const hasPermission = useCallback((permissions: number, permission: keyof typeof PERMISSIONS) => {
    return (permissions & PERMISSIONS[permission]) !== 0;
  }, []);

  const togglePermission = useCallback((permissions: number, permission: keyof typeof PERMISSIONS) => {
    return permissions ^ PERMISSIONS[permission];
  }, []);

  const setPermission = useCallback((permissions: number, permission: keyof typeof PERMISSIONS, enabled: boolean) => {
    if (enabled) {
      return permissions | PERMISSIONS[permission];
    } else {
      return permissions & ~PERMISSIONS[permission];
    }
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

  const getPermissionDescription = useCallback((permission: keyof typeof PERMISSIONS) => {
    const descriptions: Record<keyof typeof PERMISSIONS, string> = {
      VIEW_CHANNELS: 'Voir les canaux',
      SEND_MESSAGES: 'Envoyer des messages',
      MANAGE_MESSAGES: 'Gérer les messages',
      MANAGE_CHANNELS: 'Gérer les canaux',
      MANAGE_SERVER: 'Gérer le serveur',
      MANAGE_ROLES: 'Gérer les rôles',
      CONNECT_VOICE: 'Se connecter aux canaux vocaux',
      SPEAK_VOICE: 'Parler dans les canaux vocaux',
      MUTE_MEMBERS: 'Couper le micro des membres',
      DEAFEN_MEMBERS: 'Couper le son des membres',
      MOVE_MEMBERS: 'Déplacer les membres',
      ADMINISTRATOR: 'Toutes les permissions',
    };
    return descriptions[permission];
  }, []);

  return {
    loading,
    error,
    setError,
    fetchChannelPermissions,
    updateChannelPermission,
    fetchCategoryPermissions,
    updateCategoryPermission,
    hasPermission,
    togglePermission,
    setPermission,
    getPermissionNames,
    getPermissionDescription,
    PERMISSIONS,
  };
};




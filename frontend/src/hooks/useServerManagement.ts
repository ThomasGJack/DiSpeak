import { useState, useEffect, useCallback } from 'react';
import { api } from '../lib/api';

export interface Server {
  id: string;
  name: string;
  icon?: string;
  color?: string;
  description?: string;
  owner_id: string;
  is_online: boolean;
  max_users: number;
  created_at: string;
}

export interface ServerMember {
  id: string;
  user_id: string;
  username: string;
  avatar?: string;
  role: string;
  joined_at: string;
  last_seen: string;
  is_online: boolean;
}

export interface Channel {
  id: string;
  server_id: string;
  name: string;
  type: string;
  category_id?: string;
  position: number;
  description?: string;
  created_at: string;
}

export interface Category {
  id: string;
  server_id: string;
  name: string;
  position: number;
  created_at: string;
}

export interface CategoryWithChannels {
  category: Category;
  channels: Channel[];
}

export interface ServerStructure {
  server_id: string;
  categories: CategoryWithChannels[];
  uncategorized_channels: Channel[];
  members: ServerMember[];
}

export const useServerManagement = (serverId?: string) => {
  const [server, setServer] = useState<Server | null>(null);
  const [structure, setStructure] = useState<ServerStructure | null>(null);
  const [members, setMembers] = useState<ServerMember[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadServer = useCallback(async (id: string) => {
    try {
      setLoading(true);
      setError(null);
      const serverData = await api.get<Server>(`/servers/${id}`);
      setServer(serverData);
    } catch (err: any) {
      setError(err.message || 'Erreur lors du chargement du serveur');
    } finally {
      setLoading(false);
    }
  }, []);

  const loadServerStructure = useCallback(async (id: string) => {
    try {
      setLoading(true);
      setError(null);
      const structureData = await api.get<ServerStructure>(`/servers/${id}/structure`);
      setStructure(structureData);
      setMembers(structureData.members);
    } catch (err: any) {
      setError(err.message || 'Erreur lors du chargement de la structure');
    } finally {
      setLoading(false);
    }
  }, []);

  const loadServerMembers = useCallback(async (id: string) => {
    try {
      const membersData = await api.get<ServerMember[]>(`/servers/${id}/members`);
      setMembers(membersData);
    } catch (err: any) {
      setError(err.message || 'Erreur lors du chargement des membres');
    }
  }, []);

  const updateServerSettings = useCallback(async (id: string, updates: Partial<Server>) => {
    try {
      const updatedServer = await api.put<Server>(`/servers/${id}/settings`, updates);
      setServer(updatedServer);
      return updatedServer;
    } catch (err: any) {
      setError(err.message || 'Erreur lors de la mise à jour');
      throw err;
    }
  }, []);

  const addMember = useCallback(async (id: string, userId: string, role: string = 'user') => {
    try {
      await api.post(`/servers/${id}/members`, { user_id: userId, role });
      await loadServerMembers(id);
    } catch (err: any) {
      setError(err.message || "Erreur lors de l'ajout du membre");
      throw err;
    }
  }, [loadServerMembers]);

  const updateMemberRole = useCallback(async (id: string, userId: string, role: string) => {
    try {
      await api.put(`/servers/${id}/members/${userId}/role`, { role });
      await loadServerMembers(id);
    } catch (err: any) {
      setError(err.message || 'Erreur lors du changement de rôle');
      throw err;
    }
  }, [loadServerMembers]);

  const removeMember = useCallback(async (id: string, userId: string) => {
    try {
      await api.delete(`/servers/${id}/members/${userId}`);
      await loadServerMembers(id);
    } catch (err: any) {
      setError(err.message || 'Erreur lors de la suppression du membre');
      throw err;
    }
  }, [loadServerMembers]);

  const createChannel = useCallback(async (id: string, channelData: {
    name: string;
    type: string;
    category_id?: string;
    description?: string;
  }) => {
    try {
      const newChannel = await api.post<Channel>(`/servers/${id}/channels`, channelData);
      await loadServerStructure(id);
      return newChannel;
    } catch (err: any) {
      setError(err.message || 'Erreur lors de la création du canal');
      throw err;
    }
  }, [loadServerStructure]);

  const createCategory = useCallback(async (id: string, categoryData: { name: string }) => {
    try {
      const newCategory = await api.post<Category>(`/servers/${id}/categories`, categoryData);
      await loadServerStructure(id);
      return newCategory;
    } catch (err: any) {
      setError(err.message || 'Erreur lors de la création de la catégorie');
      throw err;
    }
  }, [loadServerStructure]);

  const deleteChannel = useCallback(async (channelId: string) => {
    try {
      await api.delete(`/channels/${channelId}`);
      if (serverId) {
        await loadServerStructure(serverId);
      }
    } catch (err: any) {
      setError(err.message || 'Erreur lors de la suppression du canal');
      throw err;
    }
  }, [serverId, loadServerStructure]);

  const deleteCategory = useCallback(async (categoryId: string) => {
    try {
      await api.delete(`/categories/${categoryId}`);
      if (serverId) {
        await loadServerStructure(serverId);
      }
    } catch (err: any) {
      setError(err.message || 'Erreur lors de la suppression de la catégorie');
      throw err;
    }
  }, [serverId, loadServerStructure]);

  useEffect(() => {
    if (serverId) {
      loadServer(serverId);
      loadServerStructure(serverId);
    }
  }, [serverId, loadServer, loadServerStructure]);

  return {
    server,
    structure,
    members,
    loading,
    error,
    setError,
    loadServer,
    loadServerStructure,
    loadServerMembers,
    updateServerSettings,
    addMember,
    updateMemberRole,
    removeMember,
    createChannel,
    createCategory,
    deleteChannel,
    deleteCategory,
  };
};


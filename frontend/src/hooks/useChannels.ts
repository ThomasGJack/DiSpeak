import { useState, useCallback } from 'react';
import { useWebSocket } from './useWebSocket';
import { useAuth } from './useAuth';

export interface Channel {
  id: string;
  server_id: string;
  name: string;
  type: 'text' | 'voice';
  category_id?: string | null;
  position: number;
  description?: string | null;
  created_at: string;
}

export interface Category {
  id: string;
  server_id: string;
  name: string;
  position: number;
  created_at: string;
}

export interface CreateChannelData {
  server_id: string;
  name: string;
  channel_type: 'text' | 'voice';
  category_id?: string;
}

export interface CreateCategoryData {
  server_id: string;
  name: string;
}

export interface UpdateChannelData {
  channel_id: string;
  name?: string;
  description?: string;
}

export interface UpdateCategoryData {
  category_id: string;
  name?: string;
}

export interface MoveChannelData {
  channel_id: string;
  position: number;
  category_id?: string | null;
}

export interface MoveCategoryData {
  category_id: string;
  position: number;
}

export const useChannels = (serverId?: string) => {
  const { sendCommand } = useWebSocket(serverId);
  const { user } = useAuth();
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const clearError = useCallback(() => {
    setError(null);
  }, []);

  // Gestion des canaux
  const createChannel = useCallback(async (data: CreateChannelData) => {
    if (!user?.id) {
      setError('Utilisateur non connecté');
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      await sendCommand({
        action: 'create_channel',
        data: {
          ...data,
          user_id: user.id,
        },
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Erreur lors de la création du canal');
    } finally {
      setIsLoading(false);
    }
  }, [sendCommand, user?.id]);

  const updateChannel = useCallback(async (data: UpdateChannelData) => {
    if (!user?.id) {
      setError('Utilisateur non connecté');
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      await sendCommand({
        action: 'update_channel',
        data: {
          ...data,
          user_id: user.id,
        },
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Erreur lors de la mise à jour du canal');
    } finally {
      setIsLoading(false);
    }
  }, [sendCommand, user?.id]);

  const deleteChannel = useCallback(async (channelId: string) => {
    if (!user?.id) {
      setError('Utilisateur non connecté');
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      await sendCommand({
        action: 'delete_channel',
        data: {
          channel_id: channelId,
          user_id: user.id,
        },
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Erreur lors de la suppression du canal');
    } finally {
      setIsLoading(false);
    }
  }, [sendCommand, user?.id]);

  const moveChannel = useCallback(async (data: MoveChannelData) => {
    if (!user?.id) {
      setError('Utilisateur non connecté');
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      await sendCommand({
        action: 'move_channel',
        data: {
          ...data,
          user_id: user.id,
        },
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Erreur lors du déplacement du canal');
    } finally {
      setIsLoading(false);
    }
  }, [sendCommand, user?.id]);

  // Gestion des catégories
  const createCategory = useCallback(async (data: CreateCategoryData) => {
    if (!user?.id) {
      setError('Utilisateur non connecté');
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      await sendCommand({
        action: 'create_category',
        data: {
          ...data,
          user_id: user.id,
        },
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Erreur lors de la création de la catégorie');
    } finally {
      setIsLoading(false);
    }
  }, [sendCommand, user?.id]);

  const updateCategory = useCallback(async (data: UpdateCategoryData) => {
    if (!user?.id) {
      setError('Utilisateur non connecté');
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      await sendCommand({
        action: 'update_category',
        data: {
          ...data,
          user_id: user.id,
        },
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Erreur lors de la mise à jour de la catégorie');
    } finally {
      setIsLoading(false);
    }
  }, [sendCommand, user?.id]);

  const deleteCategory = useCallback(async (categoryId: string) => {
    if (!user?.id) {
      setError('Utilisateur non connecté');
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      await sendCommand({
        action: 'delete_category',
        data: {
          category_id: categoryId,
          user_id: user.id,
        },
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Erreur lors de la suppression de la catégorie');
    } finally {
      setIsLoading(false);
    }
  }, [sendCommand, user?.id]);

  const moveCategory = useCallback(async (data: MoveCategoryData) => {
    if (!user?.id) {
      setError('Utilisateur non connecté');
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      await sendCommand({
        action: 'move_category',
        data: {
          ...data,
          user_id: user.id,
        },
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Erreur lors du déplacement de la catégorie');
    } finally {
      setIsLoading(false);
    }
  }, [sendCommand, user?.id]);

  // Fonctions utilitaires
  const sortChannelsByPosition = useCallback((channels: Channel[]) => {
    return [...channels].sort((a, b) => a.position - b.position);
  }, []);

  const sortCategoriesByPosition = useCallback((categories: Category[]) => {
    return [...categories].sort((a, b) => a.position - b.position);
  }, []);

  const groupChannelsByCategory = useCallback((channels: Channel[]) => {
    const grouped: { [categoryId: string]: Channel[] } = {};
    const uncategorized: Channel[] = [];

    channels.forEach(channel => {
      if (channel.category_id) {
        if (!grouped[channel.category_id]) {
          grouped[channel.category_id] = [];
        }
        grouped[channel.category_id].push(channel);
      } else {
        uncategorized.push(channel);
      }
    });

    // Trier les canaux dans chaque groupe
    Object.keys(grouped).forEach(categoryId => {
      grouped[categoryId] = sortChannelsByPosition(grouped[categoryId]);
    });

    return { grouped, uncategorized: sortChannelsByPosition(uncategorized) };
  }, [sortChannelsByPosition]);

  return {
    // État
    isLoading,
    error,
    
    // Actions canaux
    createChannel,
    updateChannel,
    deleteChannel,
    moveChannel,
    updateChannelPosition: (channelId: string, position: number, categoryId?: string | null) =>
      moveChannel({ channel_id: channelId, position, category_id: categoryId }),
    
    // Actions catégories
    createCategory,
    updateCategory,
    deleteCategory,
    moveCategory,
    updateCategoryPosition: (categoryId: string, position: number) =>
      moveCategory({ category_id: categoryId, position }),
    
    // Utilitaires
    sortChannelsByPosition,
    sortCategoriesByPosition,
    groupChannelsByCategory,
    clearError,
  };
};

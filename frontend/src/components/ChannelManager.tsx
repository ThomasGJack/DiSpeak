import React, { useState, useCallback } from 'react';
import { useChannels, Channel, Category } from '../hooks/useChannels';
import { useRoles } from '../hooks/useRoles';
import { DraggableChannelList } from './DraggableChannelList';

interface ChannelManagerProps {
  serverId: string;
  channels: Channel[];
  categories: Category[];
  onChannelSelect?: (channelId: string) => void;
  onChannelUpdate?: (channel: Channel) => void;
  onChannelDelete?: (channelId: string) => void;
  onCategoryUpdate?: (category: Category) => void;
  onCategoryDelete?: (categoryId: string) => void;
}

interface CreateChannelModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (data: { name: string; channel_type: 'text' | 'voice'; category_id?: string }) => void;
  categories: Category[];
}

interface EditChannelModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (data: { name?: string; description?: string }) => void;
  channel: Channel | null;
}

interface CreateCategoryModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (data: { name: string }) => void;
}

const CreateChannelModal: React.FC<CreateChannelModalProps> = ({
  isOpen,
  onClose,
  onSubmit,
  categories,
}) => {
  const [formData, setFormData] = useState({
    name: '',
    channel_type: 'text' as 'text' | 'voice',
    category_id: '',
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (formData.name.trim()) {
      onSubmit({
        name: formData.name.trim(),
        channel_type: formData.channel_type,
        category_id: formData.category_id || undefined,
      });
      setFormData({ name: '', channel_type: 'text', category_id: '' });
      onClose();
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-gray-800 rounded-lg p-6 w-96">
        <h2 className="text-xl font-bold text-white mb-4">Créer un canal</h2>
        <form onSubmit={handleSubmit}>
          <div className="mb-4">
            <label className="block text-sm font-medium text-gray-300 mb-2">
              Nom du canal
            </label>
            <input
              type="text"
              value={formData.name}
              onChange={(e) => setFormData({ ...formData, name: e.target.value })}
              className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
              placeholder="nom-du-canal"
              required
            />
          </div>
          
          <div className="mb-4">
            <label className="block text-sm font-medium text-gray-300 mb-2">
              Type de canal
            </label>
            <select
              value={formData.channel_type}
              onChange={(e) => setFormData({ ...formData, channel_type: e.target.value as 'text' | 'voice' })}
              className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              <option value="text">Texte</option>
              <option value="voice">Vocal</option>
            </select>
          </div>

          <div className="mb-6">
            <label className="block text-sm font-medium text-gray-300 mb-2">
              Catégorie (optionnel)
            </label>
            <select
              value={formData.category_id}
              onChange={(e) => setFormData({ ...formData, category_id: e.target.value })}
              className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              <option value="">Aucune catégorie</option>
              {categories.map((category) => (
                <option key={category.id} value={category.id}>
                  {category.name}
                </option>
              ))}
            </select>
          </div>

          <div className="flex justify-end space-x-3">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 text-gray-300 hover:text-white transition-colors"
            >
              Annuler
            </button>
            <button
              type="submit"
              className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
            >
              Créer
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

const EditChannelModal: React.FC<EditChannelModalProps> = ({
  isOpen,
  onClose,
  onSubmit,
  channel,
}) => {
  const [formData, setFormData] = useState({
    name: channel?.name || '',
    description: channel?.description || '',
  });

  React.useEffect(() => {
    if (channel) {
      setFormData({
        name: channel.name,
        description: channel.description || '',
      });
    }
  }, [channel]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (formData.name.trim()) {
      onSubmit({
        name: formData.name.trim(),
        description: formData.description.trim() || undefined,
      });
      onClose();
    }
  };

  if (!isOpen || !channel) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-gray-800 rounded-lg p-6 w-96">
        <h2 className="text-xl font-bold text-white mb-4">Modifier le canal</h2>
        <form onSubmit={handleSubmit}>
          <div className="mb-4">
            <label className="block text-sm font-medium text-gray-300 mb-2">
              Nom du canal
            </label>
            <input
              type="text"
              value={formData.name}
              onChange={(e) => setFormData({ ...formData, name: e.target.value })}
              className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
              required
            />
          </div>
          
          <div className="mb-6">
            <label className="block text-sm font-medium text-gray-300 mb-2">
              Description (optionnel)
            </label>
            <textarea
              value={formData.description}
              onChange={(e) => setFormData({ ...formData, description: e.target.value })}
              className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
              rows={3}
              placeholder="Description du canal..."
            />
          </div>

          <div className="flex justify-end space-x-3">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 text-gray-300 hover:text-white transition-colors"
            >
              Annuler
            </button>
            <button
              type="submit"
              className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
            >
              Modifier
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

const CreateCategoryModal: React.FC<CreateCategoryModalProps> = ({
  isOpen,
  onClose,
  onSubmit,
}) => {
  const [name, setName] = useState('');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (name.trim()) {
      onSubmit({ name: name.trim() });
      setName('');
      onClose();
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-gray-800 rounded-lg p-6 w-96">
        <h2 className="text-xl font-bold text-white mb-4">Créer une catégorie</h2>
        <form onSubmit={handleSubmit}>
          <div className="mb-6">
            <label className="block text-sm font-medium text-gray-300 mb-2">
              Nom de la catégorie
            </label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
              placeholder="Nom de la catégorie"
              required
            />
          </div>

          <div className="flex justify-end space-x-3">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 text-gray-300 hover:text-white transition-colors"
            >
              Annuler
            </button>
            <button
              type="submit"
              className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
            >
              Créer
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

export const ChannelManager: React.FC<ChannelManagerProps> = ({
  serverId,
  channels,
  categories,
  onChannelSelect,
  onChannelUpdate,
  onChannelDelete,
  onCategoryUpdate,
  onCategoryDelete,
}) => {
  const {
    createChannel,
    updateChannel,
    deleteChannel,
    moveChannel,
    createCategory,
    updateCategory,
    deleteCategory,
    moveCategory,
    groupChannelsByCategory,
    sortCategoriesByPosition,
    isLoading,
    error,
    clearError,
  } = useChannels();

  const [showCreateChannel, setShowCreateChannel] = useState(false);
  const [showEditChannel, setShowEditChannel] = useState(false);
  const [showCreateCategory, setShowCreateCategory] = useState(false);
  const [editingChannel, setEditingChannel] = useState<Channel | null>(null);
  const [editingCategory, setEditingCategory] = useState<Category | null>(null);


  const handleCreateChannel = useCallback(async (data: { name: string; channel_type: 'text' | 'voice'; category_id?: string }) => {
    try {
      await createChannel({
        server_id: serverId,
        ...data,
      });
      setShowCreateChannel(false);
    } catch (err) {
      console.error('Erreur lors de la création du canal:', err);
    }
  }, [createChannel, serverId]);

  const handleUpdateChannel = useCallback(async (data: { name?: string; description?: string }) => {
    if (!editingChannel) return;
    
    try {
      await updateChannel({
        channel_id: editingChannel.id,
        ...data,
      });
      setShowEditChannel(false);
      setEditingChannel(null);
    } catch (err) {
      console.error('Erreur lors de la mise à jour du canal:', err);
    }
  }, [updateChannel, editingChannel]);

  const handleDeleteChannel = useCallback(async (channelId: string) => {
    if (window.confirm('Êtes-vous sûr de vouloir supprimer ce canal ?')) {
      try {
        await deleteChannel(channelId);
      } catch (err) {
        console.error('Erreur lors de la suppression du canal:', err);
      }
    }
  }, [deleteChannel]);

  const handleCreateCategory = useCallback(async (data: { name: string }) => {
    try {
      await createCategory({
        server_id: serverId,
        ...data,
      });
      setShowCreateCategory(false);
    } catch (err) {
      console.error('Erreur lors de la création de la catégorie:', err);
    }
  }, [createCategory, serverId]);

  const handleUpdateCategory = useCallback(async (data: { name?: string }) => {
    if (!editingCategory) return;
    
    try {
      await updateCategory({
        category_id: editingCategory.id,
        ...data,
      });
      setEditingCategory(null);
    } catch (err) {
      console.error('Erreur lors de la mise à jour de la catégorie:', err);
    }
  }, [updateCategory, editingCategory]);

  const handleDeleteCategory = useCallback(async (categoryId: string) => {
    if (window.confirm('Êtes-vous sûr de vouloir supprimer cette catégorie ?')) {
      try {
        await deleteCategory(categoryId);
      } catch (err) {
        console.error('Erreur lors de la suppression de la catégorie:', err);
      }
    }
  }, [deleteCategory]);

  const handleChannelMove = useCallback(async (channelId: string, newPosition: number, categoryId?: string) => {
    try {
      await moveChannel({
        channel_id: channelId,
        position: newPosition,
        category_id: categoryId,
      });
    } catch (err) {
      console.error('Erreur lors du déplacement du canal:', err);
    }
  }, [moveChannel]);

  const handleCategoryMove = useCallback(async (categoryId: string, newPosition: number) => {
    try {
      await moveCategory({
        category_id: categoryId,
        position: newPosition,
      });
    } catch (err) {
      console.error('Erreur lors du déplacement de la catégorie:', err);
    }
  }, [moveCategory]);

  if (error) {
    return (
      <div className="p-4">
        <div className="bg-red-900 border border-red-700 text-red-100 px-4 py-3 rounded">
          <p>{error}</p>
          <button
            onClick={clearError}
            className="mt-2 text-red-300 hover:text-red-100 underline"
          >
            Fermer
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="p-4">
      <div className="flex justify-between items-center mb-6">
        <h2 className="text-2xl font-bold text-white">Gestion des canaux</h2>
        <div className="space-x-2">
          <button
            onClick={() => setShowCreateCategory(true)}
            className="px-4 py-2 bg-gray-600 text-white rounded-md hover:bg-gray-700 transition-colors"
            disabled={isLoading}
          >
            + Catégorie
          </button>
          <button
            onClick={() => setShowCreateChannel(true)}
            className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
            disabled={isLoading}
          >
            + Canal
          </button>
        </div>
      </div>

      <DraggableChannelList
        channels={channels}
        categories={categories}
        onChannelMove={handleChannelMove}
        onCategoryMove={handleCategoryMove}
        onChannelSelect={onChannelSelect}
        onChannelUpdate={(channel) => {
          setEditingChannel(channel as unknown as Channel);
          setShowEditChannel(true);
        }}
        onChannelDelete={handleDeleteChannel}
        onCategoryUpdate={(category) => {
          setEditingCategory(category as unknown as Category);
        }}
        onCategoryDelete={handleDeleteCategory}
        canManageChannels={true}
      />

      {/* Modales */}
      <CreateChannelModal
        isOpen={showCreateChannel}
        onClose={() => setShowCreateChannel(false)}
        onSubmit={handleCreateChannel}
        categories={categories}
      />

      <EditChannelModal
        isOpen={showEditChannel}
        onClose={() => {
          setShowEditChannel(false);
          setEditingChannel(null);
        }}
        onSubmit={handleUpdateChannel}
        channel={editingChannel}
      />

      <CreateCategoryModal
        isOpen={showCreateCategory}
        onClose={() => setShowCreateCategory(false)}
        onSubmit={handleCreateCategory}
      />
    </div>
  );
};

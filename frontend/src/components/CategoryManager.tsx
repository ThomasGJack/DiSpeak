import React, { useState, useCallback } from 'react';
import {
  DndContext,
  closestCenter,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  DragEndEvent,
  DragStartEvent,
  DragOverlay,
} from '@dnd-kit/core';
import {
  arrayMove,
  SortableContext,
  sortableKeyboardCoordinates,
  verticalListSortingStrategy,
  useSortable,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { useChannels, Category } from '../hooks/useChannels';
import { useRoles } from '../hooks/useRoles';

interface CategoryManagerProps {
  serverId: string;
  categories: Category[];
  onCategoryUpdate?: (category: Category) => void;
  onCategoryDelete?: (categoryId: string) => void;
}

interface EditCategoryModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (data: { name?: string }) => void;
  category: Category | null;
}

interface PermissionOverrideModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (data: { roleId: string; allow: number; deny: number }) => void;
  category: Category | null;
  roles: any[];
}

interface DraggableCategoryItemProps {
  category: Category;
  onUpdate?: (category: Category) => void;
  onDelete?: (categoryId: string) => void;
  canManage?: boolean;
}

const EditCategoryModal: React.FC<EditCategoryModalProps> = ({
  isOpen,
  onClose,
  onSubmit,
  category,
}) => {
  const [name, setName] = useState(category?.name || '');

  React.useEffect(() => {
    if (category) {
      setName(category.name);
    }
  }, [category]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (name.trim()) {
      onSubmit({ name: name.trim() });
      onClose();
    }
  };

  if (!isOpen || !category) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-gray-800 rounded-lg p-6 w-96">
        <h2 className="text-xl font-bold text-white mb-4">Modifier la catégorie</h2>
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
              Modifier
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

const PermissionOverrideModal: React.FC<PermissionOverrideModalProps> = ({
  isOpen,
  onClose,
  onSubmit,
  category,
  roles,
}) => {
  const [selectedRoleId, setSelectedRoleId] = useState('');
  const [permissions, setPermissions] = useState({
    allow: 0,
    deny: 0,
  });

  const handlePermissionChange = (permission: string, value: number, type: 'allow' | 'deny') => {
    setPermissions(prev => ({
      ...prev,
      [type]: type === 'allow' 
        ? prev[type] | value 
        : prev[type] & ~value
    }));
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (selectedRoleId) {
      onSubmit({
        roleId: selectedRoleId,
        allow: permissions.allow,
        deny: permissions.deny,
      });
      setSelectedRoleId('');
      setPermissions({ allow: 0, deny: 0 });
      onClose();
    }
  };

  if (!isOpen || !category) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-gray-800 rounded-lg p-6 w-96 max-h-96 overflow-y-auto">
        <h2 className="text-xl font-bold text-white mb-4">
          Permissions pour {category.name}
        </h2>
        <form onSubmit={handleSubmit}>
          <div className="mb-4">
            <label className="block text-sm font-medium text-gray-300 mb-2">
              Rôle
            </label>
            <select
              value={selectedRoleId}
              onChange={(e) => setSelectedRoleId(e.target.value)}
              className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
              required
            >
              <option value="">Sélectionner un rôle</option>
              {roles.map((role) => (
                <option key={role.id} value={role.id}>
                  {role.name}
                </option>
              ))}
            </select>
          </div>

          <div className="mb-6">
            <label className="block text-sm font-medium text-gray-300 mb-2">
              Permissions
            </label>
            <div className="space-y-2">
              {[
                { name: 'Voir les canaux', value: 1 },
                { name: 'Envoyer des messages', value: 2 },
                { name: 'Gérer les messages', value: 4 },
                { name: 'Gérer les canaux', value: 8 },
                { name: 'Gérer le serveur', value: 16 },
                { name: 'Gérer les rôles', value: 32 },
                { name: 'Se connecter en vocal', value: 64 },
                { name: 'Parler en vocal', value: 128 },
              ].map((permission) => (
                <div key={permission.value} className="flex items-center space-x-4">
                  <span className="text-white text-sm w-32">{permission.name}</span>
                  <div className="flex space-x-2">
                    <label className="flex items-center">
                      <input
                        type="checkbox"
                        checked={(permissions.allow & permission.value) !== 0}
                        onChange={(e) => handlePermissionChange(permission.name, permission.value, 'allow')}
                        className="mr-1"
                      />
                      <span className="text-green-400 text-xs">Autoriser</span>
                    </label>
                    <label className="flex items-center">
                      <input
                        type="checkbox"
                        checked={(permissions.deny & permission.value) !== 0}
                        onChange={(e) => handlePermissionChange(permission.name, permission.value, 'deny')}
                        className="mr-1"
                      />
                      <span className="text-red-400 text-xs">Refuser</span>
                    </label>
                  </div>
                </div>
              ))}
            </div>
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
              Appliquer
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

const DraggableCategoryItem: React.FC<DraggableCategoryItemProps> = ({
  category,
  onUpdate,
  onDelete,
  canManage = false,
}) => {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: category.id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={`flex items-center justify-between p-4 bg-gray-800 rounded-lg hover:bg-gray-700 transition-colors ${
        isDragging ? 'bg-gray-600' : ''
      }`}
    >
      <div className="flex items-center space-x-3 flex-1">
        {canManage && (
          <div
            {...attributes}
            {...listeners}
            className="cursor-grab hover:cursor-grabbing text-gray-400 hover:text-white p-1"
          >
            ⋮⋮
          </div>
        )}
        <span className="text-2xl">📁</span>
        <div>
          <div className="text-white font-semibold text-lg">{category.name}</div>
          <div className="text-gray-400 text-sm">
            Position: {category.position}
          </div>
        </div>
      </div>
      
      {canManage && (
        <div className="flex space-x-2">
          <button
            onClick={() => onUpdate?.(category)}
            className="px-3 py-1 text-sm text-yellow-400 hover:text-yellow-300 transition-colors"
          >
            ✏️ Modifier
          </button>
          <button
            onClick={() => onDelete?.(category.id)}
            className="px-3 py-1 text-sm text-red-400 hover:text-red-300 transition-colors"
          >
            🗑️ Supprimer
          </button>
        </div>
      )}
    </div>
  );
};

export const CategoryManager: React.FC<CategoryManagerProps> = ({
  serverId,
  categories,
  onCategoryUpdate,
  onCategoryDelete,
}) => {
  const {
    updateCategory,
    deleteCategory,
    moveCategory,
    sortCategoriesByPosition,
    isLoading,
    error,
    clearError,
  } = useChannels();

  const { roles, setCategoryPermissions } = useRoles(serverId);

  const [showEditCategory, setShowEditCategory] = useState(false);
  const [showPermissions, setShowPermissions] = useState(false);
  const [editingCategory, setEditingCategory] = useState<Category | null>(null);

  const sortedCategories = sortCategoriesByPosition(categories);

  const handleUpdateCategory = useCallback(async (data: { name?: string }) => {
    if (!editingCategory) return;
    
    try {
      await updateCategory({
        category_id: editingCategory.id,
        ...data,
      });
      setShowEditCategory(false);
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

  const handleSetPermissions = useCallback(async (data: { roleId: string; allow: number; deny: number }) => {
    if (!editingCategory) return;
    
    try {
      await setCategoryPermissions(editingCategory.id, data.roleId, data.allow, data.deny);
      setShowPermissions(false);
      setEditingCategory(null);
    } catch (err) {
      console.error('Erreur lors de la définition des permissions:', err);
    }
  }, [setCategoryPermissions, editingCategory]);

  const [activeId, setActiveId] = useState<string | null>(null);

  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    })
  );

  const handleDragStart = (event: DragStartEvent) => {
    setActiveId(event.active.id as string);
  };

  const handleDragEnd = useCallback(async (event: DragEndEvent) => {
    const { active, over } = event;
    
    if (!over) {
      setActiveId(null);
      return;
    }

    const categoryId = active.id as string;
    const overId = over.id as string;
    
    if (categoryId !== overId) {
      const overCategory = categories.find(c => c.id === overId);
      if (overCategory) {
        try {
          await moveCategory({
            category_id: categoryId,
            position: overCategory.position,
          });
        } catch (err) {
          console.error('Erreur lors du déplacement de la catégorie:', err);
        }
      }
    }

    setActiveId(null);
  }, [moveCategory, categories]);

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
        <h2 className="text-2xl font-bold text-white">Gestion des catégories</h2>
      </div>

      <DndContext
        sensors={sensors}
        collisionDetection={closestCenter}
        onDragStart={handleDragStart}
        onDragEnd={handleDragEnd}
      >
        <div className="space-y-4">
          {sortedCategories.length === 0 ? (
            <div className="text-center py-8">
              <p className="text-gray-400">Aucune catégorie créée</p>
              <p className="text-gray-500 text-sm mt-2">
                Créez des catégories pour organiser vos canaux
              </p>
            </div>
          ) : (
            <SortableContext items={sortedCategories.map(c => c.id)} strategy={verticalListSortingStrategy}>
              {sortedCategories.map((category) => (
                <DraggableCategoryItem
                  key={category.id}
                  category={category}
                  onUpdate={(category) => {
                    setEditingCategory(category);
                    setShowEditCategory(true);
                  }}
                  onDelete={handleDeleteCategory}
                  canManage={true}
                />
              ))}
            </SortableContext>
          )}
        </div>

        <DragOverlay>
          {activeId ? (
            <div className="bg-gray-700 rounded-lg p-4 opacity-90">
              <div className="flex items-center space-x-3">
                <span className="text-2xl">📁</span>
                <div>
                  <div className="text-white font-semibold text-lg">
                    {categories.find(c => c.id === activeId)?.name}
                  </div>
                </div>
              </div>
            </div>
          ) : null}
        </DragOverlay>
      </DndContext>

      {/* Modales */}
      <EditCategoryModal
        isOpen={showEditCategory}
        onClose={() => {
          setShowEditCategory(false);
          setEditingCategory(null);
        }}
        onSubmit={handleUpdateCategory}
        category={editingCategory}
      />

      <PermissionOverrideModal
        isOpen={showPermissions}
        onClose={() => {
          setShowPermissions(false);
          setEditingCategory(null);
        }}
        onSubmit={handleSetPermissions}
        category={editingCategory}
        roles={roles}
      />
    </div>
  );
};

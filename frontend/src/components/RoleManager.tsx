import React, { useState } from 'react';
import { ServerRole, CreateRole, UpdateRole, PERMISSIONS } from '../hooks/useRoles';

interface PermissionGroup {
  name: string;
  permissions: Array<{
    key: keyof typeof PERMISSIONS;
    name: string;
    description: string;
  }>;
}

interface RoleManagerProps {
  roles: ServerRole[];
  onCreateRole: (roleData: CreateRole) => Promise<void>;
  onUpdateRole: (roleId: string, roleData: UpdateRole) => Promise<void>;
  onDeleteRole: (roleId: string) => Promise<void>;
  loading?: boolean;
}

interface RoleModalProps {
  role?: ServerRole;
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (roleData: CreateRole | UpdateRole) => Promise<void>;
  loading?: boolean;
}

const PERMISSION_GROUPS: PermissionGroup[] = [
  {
    name: 'Général',
    permissions: [
      { key: 'ADMINISTRATOR', name: 'Administrateur', description: 'Accès complet au serveur' },
    ],
  },
  {
    name: 'Canaux',
    permissions: [
      { key: 'VIEW_CHANNELS', name: 'Voir les canaux', description: 'Voir les canaux du serveur' },
      { key: 'MANAGE_CHANNELS', name: 'Gérer les canaux', description: 'Créer, modifier et supprimer les canaux' },
    ],
  },
  {
    name: 'Messages',
    permissions: [
      { key: 'SEND_MESSAGES', name: 'Envoyer des messages', description: 'Envoyer des messages dans les canaux texte' },
      { key: 'MANAGE_MESSAGES', name: 'Gérer les messages', description: 'Supprimer et modifier les messages d\'autres utilisateurs' },
    ],
  },
  {
    name: 'Vocal',
    permissions: [
      { key: 'CONNECT_VOICE', name: 'Se connecter en vocal', description: 'Rejoindre les salons vocaux' },
      { key: 'SPEAK_VOICE', name: 'Parler en vocal', description: 'Parler dans les salons vocaux' },
    ],
  },
  {
    name: 'Membres',
    permissions: [
      { key: 'MUTE_MEMBERS', name: 'Couper le micro', description: 'Couper le micro d\'autres membres' },
      { key: 'DEAFEN_MEMBERS', name: 'Couper le son', description: 'Couper le son d\'autres membres' },
      { key: 'MOVE_MEMBERS', name: 'Déplacer les membres', description: 'Déplacer les membres entre les salons' },
    ],
  },
  {
    name: 'Serveur',
    permissions: [
      { key: 'MANAGE_SERVER', name: 'Gérer le serveur', description: 'Modifier les paramètres du serveur' },
      { key: 'MANAGE_ROLES', name: 'Gérer les rôles', description: 'Créer, modifier et supprimer les rôles' },
    ],
  },
];

const RoleModal: React.FC<RoleModalProps> = ({ role, isOpen, onClose, onSubmit, loading }) => {
  const [formData, setFormData] = useState({
    name: role?.name || '',
    color: role?.color || '#99AAB5',
    permissions_bitfield: role?.permissions_bitfield || 0,
  });

  const [selectedPermissions, setSelectedPermissions] = useState<Set<keyof typeof PERMISSIONS>>(
    new Set(
      Object.entries(PERMISSIONS)
        .filter(([_, value]) => (formData.permissions_bitfield & value) !== 0)
        .map(([key]) => key as keyof typeof PERMISSIONS)
    )
  );

  React.useEffect(() => {
    if (role) {
      setFormData({
        name: role.name,
        color: role.color,
        permissions_bitfield: role.permissions_bitfield,
      });
      setSelectedPermissions(
        new Set(
          Object.entries(PERMISSIONS)
            .filter(([_, value]) => (role.permissions_bitfield & value) !== 0)
            .map(([key]) => key as keyof typeof PERMISSIONS)
        )
      );
    } else {
      setFormData({
        name: '',
        color: '#99AAB5',
        permissions_bitfield: 0,
      });
      setSelectedPermissions(new Set());
    }
  }, [role, isOpen]);

  const handlePermissionToggle = (permission: keyof typeof PERMISSIONS) => {
    const newSelected = new Set(selectedPermissions);
    if (newSelected.has(permission)) {
      newSelected.delete(permission);
    } else {
      newSelected.add(permission);
    }
    setSelectedPermissions(newSelected);

    // Calculer le bitfield
    const bitfield = Array.from(newSelected).reduce((acc, perm) => acc | PERMISSIONS[perm], 0);
    setFormData(prev => ({ ...prev, permissions_bitfield: bitfield }));
  };

  const handleSelectAll = () => {
    const allPermissions = new Set(Object.keys(PERMISSIONS) as Array<keyof typeof PERMISSIONS>);
    setSelectedPermissions(allPermissions);
    setFormData(prev => ({ ...prev, permissions_bitfield: PERMISSIONS.ADMINISTRATOR }));
  };

  const handleSelectNone = () => {
    setSelectedPermissions(new Set());
    setFormData(prev => ({ ...prev, permissions_bitfield: 0 }));
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    await onSubmit(formData);
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-gray-800 rounded-lg p-6 w-full max-w-4xl max-h-[90vh] overflow-y-auto">
        <h2 className="text-2xl font-bold text-white mb-6">
          {role ? 'Modifier le rôle' : 'Créer un nouveau rôle'}
        </h2>
        
        <form onSubmit={handleSubmit} className="space-y-6">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-2">Nom du rôle</label>
              <input
                type="text"
                value={formData.name}
                onChange={(e) => setFormData(prev => ({ ...prev, name: e.target.value }))}
                className="w-full p-3 bg-gray-700 border border-gray-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                placeholder="Nom du rôle"
                required
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-300 mb-2">Couleur</label>
              <div className="flex items-center space-x-3">
                <input
                  type="color"
                  value={formData.color}
                  onChange={(e) => setFormData(prev => ({ ...prev, color: e.target.value }))}
                  className="w-12 h-12 border border-gray-600 rounded cursor-pointer"
                />
                <input
                  type="text"
                  value={formData.color}
                  onChange={(e) => setFormData(prev => ({ ...prev, color: e.target.value }))}
                  className="flex-1 p-3 bg-gray-700 border border-gray-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                  placeholder="#99AAB5"
                />
              </div>
            </div>
          </div>

          <div>
            <div className="flex justify-between items-center mb-4">
              <label className="block text-lg font-medium text-white">Permissions</label>
              <div className="flex space-x-2">
                <button
                  type="button"
                  onClick={handleSelectAll}
                  className="px-3 py-1 bg-green-600 text-white text-sm rounded hover:bg-green-700 transition-colors"
                >
                  Tout sélectionner
                </button>
                <button
                  type="button"
                  onClick={handleSelectNone}
                  className="px-3 py-1 bg-gray-600 text-white text-sm rounded hover:bg-gray-700 transition-colors"
                >
                  Tout désélectionner
                </button>
              </div>
            </div>
            
            <div className="space-y-4 max-h-96 overflow-y-auto">
              {PERMISSION_GROUPS.map((group) => (
                <div key={group.name} className="bg-gray-700 rounded-lg p-4">
                  <h3 className="text-lg font-semibold text-white mb-3">{group.name}</h3>
                  <div className="space-y-3">
                    {group.permissions.map((permission) => (
                      <div key={permission.key} className="flex items-start space-x-3">
                        <input
                          type="checkbox"
                          id={permission.key}
                          checked={selectedPermissions.has(permission.key)}
                          onChange={() => handlePermissionToggle(permission.key)}
                          className="mt-1 w-4 h-4 text-blue-600 bg-gray-700 border-gray-600 rounded focus:ring-blue-500"
                        />
                        <div className="flex-1">
                          <label
                            htmlFor={permission.key}
                            className="block text-white font-medium cursor-pointer"
                          >
                            {permission.name}
                          </label>
                          <p className="text-gray-400 text-sm mt-1">{permission.description}</p>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          </div>

          <div className="flex justify-end space-x-3 pt-4 border-t border-gray-700">
            <button
              type="button"
              onClick={onClose}
              className="px-6 py-2 text-gray-300 hover:text-white transition-colors"
            >
              Annuler
            </button>
            <button
              type="submit"
              disabled={loading}
              className="px-6 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50 transition-colors"
            >
              {loading ? 'Enregistrement...' : (role ? 'Modifier' : 'Créer')}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

export const RoleManager: React.FC<RoleManagerProps> = ({
  roles,
  onCreateRole,
  onUpdateRole,
  onDeleteRole,
  loading = false,
}) => {
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingRole, setEditingRole] = useState<ServerRole | undefined>();
  const [deletingRole, setDeletingRole] = useState<string | null>(null);

  const handleCreateRole = () => {
    setEditingRole(undefined);
    setIsModalOpen(true);
  };

  const handleEditRole = (role: ServerRole) => {
    setEditingRole(role);
    setIsModalOpen(true);
  };

  const handleDeleteRole = async (roleId: string) => {
    if (window.confirm('Êtes-vous sûr de vouloir supprimer ce rôle ?')) {
      setDeletingRole(roleId);
      try {
        await onDeleteRole(roleId);
      } finally {
        setDeletingRole(null);
      }
    }
  };

  const handleModalSubmit = async (roleData: CreateRole | UpdateRole) => {
    if (editingRole) {
      await onUpdateRole(editingRole.id, roleData as UpdateRole);
    } else {
      await onCreateRole(roleData as CreateRole);
    }
  };

  const getPermissionNames = (permissions: number) => {
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
  };

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h3 className="text-2xl font-bold text-white">Rôles du serveur</h3>
        <button
          onClick={handleCreateRole}
          className="px-6 py-3 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors font-medium"
        >
          + Créer un rôle
        </button>
      </div>

      {roles.length === 0 ? (
        <div className="text-center py-12">
          <div className="text-gray-400 text-6xl mb-4">👥</div>
          <h4 className="text-xl font-semibold text-white mb-2">Aucun rôle créé</h4>
          <p className="text-gray-400 mb-6">
            Créez des rôles personnalisés pour organiser les permissions de vos membres
          </p>
          <button
            onClick={handleCreateRole}
            className="px-6 py-3 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
          >
            Créer le premier rôle
          </button>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {roles.map((role) => (
            <div
              key={role.id}
              className="bg-gray-800 rounded-lg p-6 hover:bg-gray-700 transition-colors"
            >
              <div className="flex items-start justify-between mb-4">
                <div className="flex items-center space-x-3">
                  <div
                    className="w-6 h-6 rounded-full border-2 border-gray-600"
                    style={{ backgroundColor: role.color }}
                  />
                  <div>
                    <h4 className="text-lg font-semibold text-white">{role.name}</h4>
                    {role.is_default && (
                      <span className="text-xs bg-blue-600 text-blue-100 px-2 py-1 rounded">
                        Par défaut
                      </span>
                    )}
                  </div>
                </div>
                
                <div className="flex space-x-1">
                  <button
                    onClick={() => handleEditRole(role)}
                    className="p-2 text-gray-400 hover:text-white hover:bg-gray-600 rounded transition-colors"
                    title="Modifier"
                  >
                    ✏️
                  </button>
                  {!role.is_default && (
                    <button
                      onClick={() => handleDeleteRole(role.id)}
                      disabled={deletingRole === role.id}
                      className="p-2 text-gray-400 hover:text-red-400 hover:bg-gray-600 rounded transition-colors disabled:opacity-50"
                      title="Supprimer"
                    >
                      {deletingRole === role.id ? '⏳' : '🗑️'}
                    </button>
                  )}
                </div>
              </div>

              <div className="space-y-3">
                <div className="flex items-center justify-between text-sm">
                  <span className="text-gray-400">Membres</span>
                  <span className="text-white font-medium">
                    {role.members_count || 0}
                  </span>
                </div>
                
                <div className="flex items-center justify-between text-sm">
                  <span className="text-gray-400">Permissions</span>
                  <span className="text-white font-medium">
                    {getPermissionNames(role.permissions_bitfield).length}
                  </span>
                </div>

                <div className="pt-3 border-t border-gray-700">
                  <div className="text-xs text-gray-400 mb-2">Permissions actives :</div>
                  <div className="flex flex-wrap gap-1">
                    {getPermissionNames(role.permissions_bitfield).slice(0, 3).map((permission) => (
                      <span
                        key={permission}
                        className="px-2 py-1 bg-gray-700 text-gray-300 text-xs rounded"
                      >
                        {permission.replace(/_/g, ' ')}
                      </span>
                    ))}
                    {getPermissionNames(role.permissions_bitfield).length > 3 && (
                      <span className="px-2 py-1 bg-gray-700 text-gray-300 text-xs rounded">
                        +{getPermissionNames(role.permissions_bitfield).length - 3} autres
                      </span>
                    )}
                  </div>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      <RoleModal
        role={editingRole}
        isOpen={isModalOpen}
        onClose={() => setIsModalOpen(false)}
        onSubmit={handleModalSubmit}
        loading={loading}
      />
    </div>
  );
};

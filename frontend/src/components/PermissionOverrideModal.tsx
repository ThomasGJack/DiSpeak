import React, { useState, useEffect } from 'react';
import { useRoles, PERMISSIONS, ServerRole } from '../hooks/useRoles';

interface PermissionOverrideModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (data: {
    roleId: string;
    allow: number;
    deny: number;
  }) => void;
  targetType: 'channel' | 'category';
  targetId: string;
  targetName: string;
  roles: ServerRole[];
  existingOverrides?: {
    roleId: string;
    allow: number;
    deny: number;
  }[];
}

interface PermissionRowProps {
  permission: {
    name: string;
    value: number;
    description: string;
  };
  rolePermissions: number;
  allow: number;
  deny: number;
  onAllowChange: (value: number, checked: boolean) => void;
  onDenyChange: (value: number, checked: boolean) => void;
}

const PermissionRow: React.FC<PermissionRowProps> = ({
  permission,
  rolePermissions,
  allow,
  deny,
  onAllowChange,
  onDenyChange,
}) => {
  const hasRolePermission = (rolePermissions & permission.value) !== 0;
  const hasAllowOverride = (allow & permission.value) !== 0;
  const hasDenyOverride = (deny & permission.value) !== 0;
  const isInherited = hasRolePermission && !hasAllowOverride && !hasDenyOverride;

  return (
    <div className="flex items-center justify-between p-3 bg-gray-700 rounded-md">
      <div className="flex-1">
        <div className="flex items-center space-x-2">
          <span className="text-white font-medium">{permission.name}</span>
          {isInherited && (
            <span className="text-xs bg-blue-600 text-blue-100 px-2 py-1 rounded">
              Hérité
            </span>
          )}
        </div>
        <p className="text-gray-400 text-sm mt-1">{permission.description}</p>
      </div>
      
      <div className="flex items-center space-x-4">
        <label className="flex items-center space-x-2">
          <input
            type="checkbox"
            checked={hasAllowOverride}
            onChange={(e) => onAllowChange(permission.value, e.target.checked)}
            className="w-4 h-4 text-green-600 bg-gray-700 border-gray-600 rounded focus:ring-green-500"
            disabled={hasDenyOverride}
          />
          <span className="text-green-400 text-sm">Autoriser</span>
        </label>
        
        <label className="flex items-center space-x-2">
          <input
            type="checkbox"
            checked={hasDenyOverride}
            onChange={(e) => onDenyChange(permission.value, e.target.checked)}
            className="w-4 h-4 text-red-600 bg-gray-700 border-gray-600 rounded focus:ring-red-500"
            disabled={hasAllowOverride}
          />
          <span className="text-red-400 text-sm">Refuser</span>
        </label>
      </div>
    </div>
  );
};

export const PermissionOverrideModal: React.FC<PermissionOverrideModalProps> = ({
  isOpen,
  onClose,
  onSubmit,
  targetType,
  targetId,
  targetName,
  roles,
  existingOverrides = [],
}) => {
  const [selectedRoleId, setSelectedRoleId] = useState('');
  const [permissions, setPermissions] = useState({
    allow: 0,
    deny: 0,
  });
  const [isEditing, setIsEditing] = useState(false);

  const selectedRole = roles.find(role => role.id === selectedRoleId);

  const permissionList = [
    { name: 'Voir les canaux', value: PERMISSIONS.VIEW_CHANNELS, description: 'Voir les canaux de cette catégorie' },
    { name: 'Envoyer des messages', value: PERMISSIONS.SEND_MESSAGES, description: 'Envoyer des messages dans les canaux texte' },
    { name: 'Gérer les messages', value: PERMISSIONS.MANAGE_MESSAGES, description: 'Supprimer et modifier les messages d\'autres utilisateurs' },
    { name: 'Gérer les canaux', value: PERMISSIONS.MANAGE_CHANNELS, description: 'Créer, modifier et supprimer les canaux' },
    { name: 'Gérer le serveur', value: PERMISSIONS.MANAGE_SERVER, description: 'Modifier les paramètres du serveur' },
    { name: 'Gérer les rôles', value: PERMISSIONS.MANAGE_ROLES, description: 'Créer, modifier et supprimer les rôles' },
    { name: 'Se connecter en vocal', value: PERMISSIONS.CONNECT_VOICE, description: 'Rejoindre les salons vocaux' },
    { name: 'Parler en vocal', value: PERMISSIONS.SPEAK_VOICE, description: 'Parler dans les salons vocaux' },
    { name: 'Couper le micro', value: PERMISSIONS.MUTE_MEMBERS, description: 'Couper le micro d\'autres membres' },
    { name: 'Couper le son', value: PERMISSIONS.DEAFEN_MEMBERS, description: 'Couper le son d\'autres membres' },
    { name: 'Déplacer les membres', value: PERMISSIONS.MOVE_MEMBERS, description: 'Déplacer les membres entre les salons' },
  ];

  useEffect(() => {
    if (selectedRoleId) {
      const existingOverride = existingOverrides.find(override => override.roleId === selectedRoleId);
      if (existingOverride) {
        setPermissions({
          allow: existingOverride.allow,
          deny: existingOverride.deny,
        });
        setIsEditing(true);
      } else {
        setPermissions({ allow: 0, deny: 0 });
        setIsEditing(false);
      }
    }
  }, [selectedRoleId, existingOverrides]);

  const handleAllowChange = (value: number, checked: boolean) => {
    setPermissions(prev => ({
      ...prev,
      allow: checked ? prev.allow | value : prev.allow & ~value,
      deny: checked ? prev.deny & ~value : prev.deny, // Décocher deny si on coche allow
    }));
  };

  const handleDenyChange = (value: number, checked: boolean) => {
    setPermissions(prev => ({
      ...prev,
      deny: checked ? prev.deny | value : prev.deny & ~value,
      allow: checked ? prev.allow & ~value : prev.allow, // Décocher allow si on coche deny
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
      setIsEditing(false);
      onClose();
    }
  };

  const handleReset = () => {
    setPermissions({ allow: 0, deny: 0 });
    setIsEditing(false);
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-gray-800 rounded-lg p-6 w-full max-w-4xl max-h-[90vh] overflow-y-auto">
        <div className="flex justify-between items-center mb-6">
          <h2 className="text-2xl font-bold text-white">
            Permissions pour {targetName}
          </h2>
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-white text-2xl"
          >
            ×
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="mb-6">
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
                  {role.name} {role.is_default && '(Par défaut)'}
                </option>
              ))}
            </select>
          </div>

          {selectedRole && (
            <div className="mb-6">
              <div className="bg-gray-700 rounded-lg p-4 mb-4">
                <h3 className="text-lg font-semibold text-white mb-2">
                  Permissions de base du rôle "{selectedRole.name}"
                </h3>
                <div className="flex flex-wrap gap-2">
                  {permissionList.map((permission) => {
                    const hasPermission = (selectedRole.permissions_bitfield & permission.value) !== 0;
                    return (
                      <span
                        key={permission.value}
                        className={`px-2 py-1 rounded text-xs ${
                          hasPermission
                            ? 'bg-green-600 text-green-100'
                            : 'bg-gray-600 text-gray-300'
                        }`}
                      >
                        {permission.name}
                      </span>
                    );
                  })}
                </div>
              </div>

              <div className="space-y-3">
                <h3 className="text-lg font-semibold text-white">
                  Overrides de permissions
                </h3>
                <p className="text-gray-400 text-sm mb-4">
                  Définissez des permissions spécifiques pour ce {targetType === 'channel' ? 'canal' : 'catégorie'}.
                  Les overrides remplacent les permissions de base du rôle.
                </p>

                {permissionList.map((permission) => (
                  <PermissionRow
                    key={permission.value}
                    permission={permission}
                    rolePermissions={selectedRole.permissions_bitfield}
                    allow={permissions.allow}
                    deny={permissions.deny}
                    onAllowChange={handleAllowChange}
                    onDenyChange={handleDenyChange}
                  />
                ))}
              </div>
            </div>
          )}

          <div className="flex justify-between items-center">
            <div className="flex space-x-3">
              {isEditing && (
                <button
                  type="button"
                  onClick={handleReset}
                  className="px-4 py-2 bg-gray-600 text-white rounded-md hover:bg-gray-700 transition-colors"
                >
                  Réinitialiser
                </button>
              )}
            </div>
            
            <div className="flex space-x-3">
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
                disabled={!selectedRoleId}
              >
                {isEditing ? 'Modifier' : 'Appliquer'}
              </button>
            </div>
          </div>
        </form>
      </div>
    </div>
  );
};

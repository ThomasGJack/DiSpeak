import React, { useState, useEffect } from 'react';
import { PERMISSIONS } from '../hooks/useRoles';

interface PermissionMatrixRole {
  id: string;
  name: string;
  color: string;
  permissions_bitfield: number;
  is_default: boolean;
}

interface PermissionMatrixProps {
  serverId: string;
  roles: PermissionMatrixRole[];
  channels: Array<{
    id: string;
    name: string;
    type: string;
    category_id?: string;
  }>;
  categories: Array<{
    id: string;
    name: string;
  }>;
  onSetChannelPermissions: (channelId: string, roleId: string, allow: number, deny: number) => Promise<void>;
  onPermissionChange?: (channelId: string, roleId: string, permissions: number) => void;
}

interface PermissionOverride {
  roleId: string;
  allow: number;
  deny: number;
}

interface ChannelOverrides {
  [channelId: string]: {
    [roleId: string]: PermissionOverride;
  };
}

interface PermissionCellProps {
  role: PermissionMatrixRole;
  channelId: string;
  basePermissions: number;
  overrides?: PermissionOverride;
  onPermissionChange: (allow: number, deny: number) => void;
}

const PermissionCell: React.FC<PermissionCellProps> = ({
  role,
  channelId,
  basePermissions,
  overrides,
  onPermissionChange,
}) => {
  const [isExpanded, setIsExpanded] = useState(false);
  const [localOverrides, setLocalOverrides] = useState({
    allow: overrides?.allow || 0,
    deny: overrides?.deny || 0,
  });

  const hasPermission = (permissions: number, permission: keyof typeof PERMISSIONS) => {
    return (permissions & PERMISSIONS[permission]) !== 0;
  };

  const calculateEffectivePermissions = () => {
    // Appliquer les overrides : (base | allow) & ~deny
    return (basePermissions | localOverrides.allow) & ~localOverrides.deny;
  };

  const effectivePermissions = calculateEffectivePermissions();

  const handlePermissionToggle = (permission: keyof typeof PERMISSIONS, type: 'allow' | 'deny') => {
    const newOverrides = { ...localOverrides };
    const permissionValue = PERMISSIONS[permission];
    
    if (type === 'allow') {
      if (hasPermission(localOverrides.allow, permission)) {
        newOverrides.allow &= ~permissionValue;
      } else {
        newOverrides.allow |= permissionValue;
        newOverrides.deny &= ~permissionValue; // Retirer de deny si on ajoute à allow
      }
    } else {
      if (hasPermission(localOverrides.deny, permission)) {
        newOverrides.deny &= ~permissionValue;
      } else {
        newOverrides.deny |= permissionValue;
        newOverrides.allow &= ~permissionValue; // Retirer de allow si on ajoute à deny
      }
    }
    
    setLocalOverrides(newOverrides);
    onPermissionChange(newOverrides.allow, newOverrides.deny);
  };

  const getPermissionIcon = (permission: keyof typeof PERMISSIONS) => {
    const icons: Record<keyof typeof PERMISSIONS, string> = {
      VIEW_CHANNELS: '👁️',
      SEND_MESSAGES: '💬',
      MANAGE_MESSAGES: '✏️',
      MANAGE_CHANNELS: '🔧',
      MANAGE_SERVER: '⚙️',
      MANAGE_ROLES: '👥',
      CONNECT_VOICE: '🎤',
      SPEAK_VOICE: '🔊',
      MUTE_MEMBERS: '🔇',
      DEAFEN_MEMBERS: '🔇',
      MOVE_MEMBERS: '↔️',
      ADMINISTRATOR: '👑',
    };
    return icons[permission];
  };

  const getPermissionDescription = (permission: keyof typeof PERMISSIONS) => {
    const descriptions: Record<keyof typeof PERMISSIONS, string> = {
      VIEW_CHANNELS: 'Voir les canaux',
      SEND_MESSAGES: 'Envoyer des messages',
      MANAGE_MESSAGES: 'Gérer les messages',
      MANAGE_CHANNELS: 'Gérer les canaux',
      MANAGE_SERVER: 'Gérer le serveur',
      MANAGE_ROLES: 'Gérer les rôles',
      CONNECT_VOICE: 'Se connecter en vocal',
      SPEAK_VOICE: 'Parler en vocal',
      MUTE_MEMBERS: 'Couper le micro',
      DEAFEN_MEMBERS: 'Couper le son',
      MOVE_MEMBERS: 'Déplacer les membres',
      ADMINISTRATOR: 'Administrateur',
    };
    return descriptions[permission];
  };

  const hasAnyPermission = Object.keys(PERMISSIONS).some(key => 
    key !== 'ADMINISTRATOR' && hasPermission(effectivePermissions, key as keyof typeof PERMISSIONS)
  );

  const hasAllPermissions = Object.keys(PERMISSIONS).every(key => 
    key === 'ADMINISTRATOR' || hasPermission(effectivePermissions, key as keyof typeof PERMISSIONS)
  );

  const hasOverrides = localOverrides.allow !== 0 || localOverrides.deny !== 0;

  return (
    <div className="relative">
      <div
        className={`w-10 h-10 rounded cursor-pointer flex items-center justify-center text-xs font-bold relative ${
          hasAllPermissions ? 'bg-green-600 text-white' :
          hasAnyPermission ? 'bg-yellow-500 text-white' :
          'bg-gray-600 text-gray-300'
        }`}
        onClick={() => setIsExpanded(!isExpanded)}
        title={`Permissions pour ${role.name}${hasOverrides ? ' (avec overrides)' : ''}`}
      >
        {hasAllPermissions ? '✓' : hasAnyPermission ? '~' : '✗'}
        {hasOverrides && (
          <div className="absolute -top-1 -right-1 w-3 h-3 bg-blue-500 rounded-full border-2 border-gray-800"></div>
        )}
      </div>

      {isExpanded && (
        <div className="absolute top-12 left-0 z-20 bg-gray-800 border border-gray-600 rounded-lg shadow-xl p-4 min-w-80">
          <div className="flex items-center justify-between mb-3">
            <div className="text-sm font-medium text-white">{role.name}</div>
            {hasOverrides && (
              <span className="text-xs bg-blue-600 text-blue-100 px-2 py-1 rounded">
                Overrides actifs
              </span>
            )}
          </div>
          
          <div className="space-y-2 max-h-64 overflow-y-auto">
            {Object.entries(PERMISSIONS).map(([key, _value]) => {
              if (key === 'ADMINISTRATOR') return null;
              const permission = key as keyof typeof PERMISSIONS;
              const hasBasePermission = hasPermission(basePermissions, permission);
              const hasAllowOverride = hasPermission(localOverrides.allow, permission);
              const hasDenyOverride = hasPermission(localOverrides.deny, permission);
              const isEffective = hasPermission(effectivePermissions, permission);
              
              return (
                <div key={key} className="flex items-center justify-between p-2 bg-gray-700 rounded">
                  <div className="flex items-center space-x-2">
                    <span className="text-sm">{getPermissionIcon(permission)}</span>
                    <span className="text-xs text-white">{getPermissionDescription(permission)}</span>
                    {hasBasePermission && !hasAllowOverride && !hasDenyOverride && (
                      <span className="text-xs text-gray-400">(hérité)</span>
                    )}
                  </div>
                  
                  <div className="flex space-x-2">
                    <label className="flex items-center">
                      <input
                        type="checkbox"
                        checked={hasAllowOverride}
                        onChange={() => handlePermissionToggle(permission, 'allow')}
                        className="w-3 h-3 text-green-600 bg-gray-700 border-gray-600 rounded"
                      />
                      <span className="ml-1 text-xs text-green-400">A</span>
                    </label>
                    <label className="flex items-center">
                      <input
                        type="checkbox"
                        checked={hasDenyOverride}
                        onChange={() => handlePermissionToggle(permission, 'deny')}
                        className="w-3 h-3 text-red-600 bg-gray-700 border-gray-600 rounded"
                      />
                      <span className="ml-1 text-xs text-red-400">D</span>
                    </label>
                    <div className={`w-3 h-3 rounded ${
                      isEffective ? 'bg-green-500' : 'bg-gray-500'
                    }`} title={isEffective ? 'Permission effective' : 'Permission refusée'}></div>
                  </div>
                </div>
              );
            })}
          </div>
          
          <div className="mt-3 pt-3 border-t border-gray-600">
            <div className="flex items-center justify-between p-2 bg-gray-700 rounded">
              <div className="flex items-center space-x-2">
                <span className="text-sm">{getPermissionIcon('ADMINISTRATOR')}</span>
                <span className="text-xs text-white">{getPermissionDescription('ADMINISTRATOR')}</span>
              </div>
              <div className="flex space-x-2">
                <label className="flex items-center">
                  <input
                    type="checkbox"
                    checked={hasPermission(localOverrides.allow, 'ADMINISTRATOR')}
                    onChange={() => handlePermissionToggle('ADMINISTRATOR', 'allow')}
                    className="w-3 h-3 text-green-600 bg-gray-700 border-gray-600 rounded"
                  />
                  <span className="ml-1 text-xs text-green-400">A</span>
                </label>
                <label className="flex items-center">
                  <input
                    type="checkbox"
                    checked={hasPermission(localOverrides.deny, 'ADMINISTRATOR')}
                    onChange={() => handlePermissionToggle('ADMINISTRATOR', 'deny')}
                    className="w-3 h-3 text-red-600 bg-gray-700 border-gray-600 rounded"
                  />
                  <span className="ml-1 text-xs text-red-400">D</span>
                </label>
                <div className={`w-3 h-3 rounded ${
                  hasPermission(effectivePermissions, 'ADMINISTRATOR') ? 'bg-green-500' : 'bg-gray-500'
                }`} title={hasPermission(effectivePermissions, 'ADMINISTRATOR') ? 'Permission effective' : 'Permission refusée'}></div>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export const PermissionMatrix: React.FC<PermissionMatrixProps> = ({
  serverId,
  roles,
  channels,
  categories,
  onSetChannelPermissions,
  onPermissionChange,
}) => {
  const [overrides, setOverrides] = useState<ChannelOverrides>({});
  const [loading, setLoading] = useState(false);

  const handlePermissionChange = async (channelId: string, roleId: string, allow: number, deny: number) => {
    try {
      setLoading(true);
      await onSetChannelPermissions(channelId, roleId, allow, deny);
      
      setOverrides(prev => ({
        ...prev,
        [channelId]: {
          ...prev[channelId],
          [roleId]: { roleId, allow, deny },
        },
      }));

      onPermissionChange?.(channelId, roleId, allow | deny);
    } catch (error) {
      console.error('Erreur lors de la mise à jour des permissions:', error);
    } finally {
      setLoading(false);
    }
  };

  const getChannelOverrides = (channelId: string, roleId: string): PermissionOverride | undefined => {
    return overrides[channelId]?.[roleId];
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center p-8">
        <div className="text-gray-400">Chargement des permissions...</div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h3 className="text-2xl font-bold text-white">Matrice des permissions</h3>
        <div className="text-sm text-gray-400">
          Cliquez sur une cellule pour gérer les permissions
        </div>
      </div>
      
      <div className="overflow-x-auto bg-gray-800 rounded-lg">
        <table className="w-full">
          <thead>
            <tr className="border-b border-gray-700">
              <th className="p-4 text-left text-white font-semibold">Canal</th>
              {roles.map(role => (
                <th key={role.id} className="p-4 text-center">
                  <div className="flex flex-col items-center space-y-2">
                    <div
                      className="w-6 h-6 rounded-full border-2 border-gray-600"
                      style={{ backgroundColor: role.color }}
                    />
                    <span className="text-sm text-white font-medium">{role.name}</span>
                    {role.is_default && (
                      <span className="text-xs bg-blue-600 text-blue-100 px-2 py-1 rounded">
                        Par défaut
                      </span>
                    )}
                  </div>
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {categories.map(category => (
              <React.Fragment key={category.id}>
                <tr className="bg-gray-700">
                  <td className="p-4 font-medium text-white" colSpan={roles.length + 1}>
                    <div className="flex items-center space-x-2">
                      <span>📁</span>
                      <span>{category.name}</span>
                    </div>
                  </td>
                </tr>
                {channels
                  .filter(channel => channel.category_id === category.id)
                  .map(channel => (
                    <tr key={channel.id} className="border-b border-gray-700 hover:bg-gray-700/50">
                      <td className="p-4">
                        <div className="flex items-center space-x-3">
                          <span className="text-lg">
                            {channel.type === 'text' ? '💬' : '🎤'}
                          </span>
                          <div>
                            <div className="text-white font-medium">{channel.name}</div>
                            <div className="text-xs text-gray-400">
                              {channel.type === 'text' ? 'Canal texte' : 'Salon vocal'}
                            </div>
                          </div>
                        </div>
                      </td>
                      {roles.map(role => (
                        <td key={role.id} className="p-4 text-center">
                          <PermissionCell
                            role={role}
                            channelId={channel.id}
                            basePermissions={role.permissions_bitfield}
                            overrides={getChannelOverrides(channel.id, role.id)}
                            onPermissionChange={(allow, deny) =>
                              handlePermissionChange(channel.id, role.id, allow, deny)
                            }
                          />
                        </td>
                      ))}
                    </tr>
                  ))}
              </React.Fragment>
            ))}
            
            {/* Canaux non catégorisés */}
            {channels.filter(channel => !channel.category_id).length > 0 && (
              <React.Fragment>
                <tr className="bg-gray-700">
                  <td className="p-4 font-medium text-white" colSpan={roles.length + 1}>
                    <div className="flex items-center space-x-2">
                      <span>📁</span>
                      <span>Sans catégorie</span>
                    </div>
                  </td>
                </tr>
                {channels
                  .filter(channel => !channel.category_id)
                  .map(channel => (
                    <tr key={channel.id} className="border-b border-gray-700 hover:bg-gray-700/50">
                      <td className="p-4">
                        <div className="flex items-center space-x-3">
                          <span className="text-lg">
                            {channel.type === 'text' ? '💬' : '🎤'}
                          </span>
                          <div>
                            <div className="text-white font-medium">{channel.name}</div>
                            <div className="text-xs text-gray-400">
                              {channel.type === 'text' ? 'Canal texte' : 'Salon vocal'}
                            </div>
                          </div>
                        </div>
                      </td>
                      {roles.map(role => (
                        <td key={role.id} className="p-4 text-center">
                          <PermissionCell
                            role={role}
                            channelId={channel.id}
                            basePermissions={role.permissions_bitfield}
                            overrides={getChannelOverrides(channel.id, role.id)}
                            onPermissionChange={(allow, deny) =>
                              handlePermissionChange(channel.id, role.id, allow, deny)
                            }
                          />
                        </td>
                      ))}
                    </tr>
                  ))}
              </React.Fragment>
            )}
          </tbody>
        </table>
      </div>

      <div className="bg-gray-800 rounded-lg p-4">
        <h4 className="text-lg font-semibold text-white mb-3">Légende</h4>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="space-y-2">
            <div className="text-sm font-medium text-gray-300">États des permissions :</div>
            <div className="flex items-center space-x-3">
              <div className="w-4 h-4 bg-green-600 rounded flex items-center justify-center text-xs text-white">✓</div>
              <span className="text-sm text-gray-400">Toutes permissions</span>
            </div>
            <div className="flex items-center space-x-3">
              <div className="w-4 h-4 bg-yellow-500 rounded flex items-center justify-center text-xs text-white">~</div>
              <span className="text-sm text-gray-400">Permissions partielles</span>
            </div>
            <div className="flex items-center space-x-3">
              <div className="w-4 h-4 bg-gray-600 rounded flex items-center justify-center text-xs text-white">✗</div>
              <span className="text-sm text-gray-400">Aucune permission</span>
            </div>
          </div>
          
          <div className="space-y-2">
            <div className="text-sm font-medium text-gray-300">Indicateurs :</div>
            <div className="flex items-center space-x-3">
              <div className="w-4 h-4 bg-blue-500 rounded-full"></div>
              <span className="text-sm text-gray-400">Overrides actifs</span>
            </div>
            <div className="flex items-center space-x-3">
              <span className="text-xs text-green-400">A</span>
              <span className="text-sm text-gray-400">Autoriser</span>
            </div>
            <div className="flex items-center space-x-3">
              <span className="text-xs text-red-400">D</span>
              <span className="text-sm text-gray-400">Refuser</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

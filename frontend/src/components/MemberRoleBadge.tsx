import React, { useState } from 'react';
import { ServerRole, PERMISSIONS } from '../hooks/useRoles';

interface MemberRoleBadgeProps {
  roles: ServerRole[];
  onRoleRemove?: (roleId: string) => void;
  showTooltip?: boolean;
  maxVisible?: number;
}

export const MemberRoleBadge: React.FC<MemberRoleBadgeProps> = ({
  roles,
  onRoleRemove,
  showTooltip = true,
  maxVisible = 3,
}) => {
  const [hoveredRole, setHoveredRole] = useState<ServerRole | null>(null);

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

  const visibleRoles = roles.slice(0, maxVisible);
  const hiddenCount = roles.length - maxVisible;

  return (
    <div className="flex flex-wrap gap-1">
      {visibleRoles.map((role) => (
        <div
          key={role.id}
          className="relative"
          onMouseEnter={() => setHoveredRole(role)}
          onMouseLeave={() => setHoveredRole(null)}
        >
          <div
            className="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium text-white"
            style={{ backgroundColor: role.color }}
          >
            {role.name}
            {onRoleRemove && !role.is_default && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onRoleRemove(role.id);
                }}
                className="ml-1 hover:bg-black hover:bg-opacity-20 rounded-full p-0.5"
              >
                ×
              </button>
            )}
          </div>

          {showTooltip && hoveredRole?.id === role.id && (
            <div className="absolute bottom-full left-1/2 transform -translate-x-1/2 mb-2 z-10">
              <div className="bg-gray-900 text-white text-xs rounded-lg p-3 shadow-lg max-w-xs">
                <div className="font-medium mb-1">{role.name}</div>
                <div className="text-gray-300 mb-2">
                  Position: {role.position}
                </div>
                <div className="mb-2">
                  <div className="font-medium mb-1">Permissions:</div>
                  <div className="space-y-1">
                    {getPermissionNames(role.permissions_bitfield).map((permission) => (
                      <div key={permission} className="text-gray-300">
                        • {permission.replace(/_/g, ' ')}
                      </div>
                    ))}
                  </div>
                </div>
                {role.is_default && (
                  <div className="text-yellow-400 text-xs">
                    Rôle par défaut
                  </div>
                )}
              </div>
              <div className="absolute top-full left-1/2 transform -translate-x-1/2">
                <div className="border-4 border-transparent border-t-gray-900"></div>
              </div>
            </div>
          )}
        </div>
      ))}

      {hiddenCount > 0 && (
        <div className="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-gray-500 text-white">
          +{hiddenCount}
        </div>
      )}
    </div>
  );
};

interface MemberRoleSelectorProps {
  availableRoles: ServerRole[];
  selectedRoleIds: string[];
  onRoleToggle: (roleId: string) => void;
  disabled?: boolean;
}

export const MemberRoleSelector: React.FC<MemberRoleSelectorProps> = ({
  availableRoles,
  selectedRoleIds,
  onRoleToggle,
  disabled = false,
}) => {
  const [isOpen, setIsOpen] = useState(false);

  return (
    <div className="relative">
      <button
        onClick={() => setIsOpen(!isOpen)}
        disabled={disabled}
        className="flex items-center space-x-2 px-3 py-2 border rounded-md hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
      >
        <span className="text-sm">
          {selectedRoleIds.length > 0 
            ? `${selectedRoleIds.length} rôle(s) sélectionné(s)`
            : 'Sélectionner des rôles'
          }
        </span>
        <span className="text-gray-400">▼</span>
      </button>

      {isOpen && (
        <div className="absolute top-full left-0 mt-1 bg-white border rounded-md shadow-lg z-10 min-w-48">
          <div className="p-2">
            <div className="text-xs text-gray-500 mb-2">Rôles disponibles:</div>
            <div className="space-y-1 max-h-48 overflow-y-auto">
              {availableRoles.map((role) => {
                const isSelected = selectedRoleIds.includes(role.id);
                return (
                  <label
                    key={role.id}
                    className="flex items-center space-x-2 p-2 hover:bg-gray-50 rounded cursor-pointer"
                  >
                    <input
                      type="checkbox"
                      checked={isSelected}
                      onChange={() => onRoleToggle(role.id)}
                      className="rounded"
                    />
                    <div
                      className="w-3 h-3 rounded-full"
                      style={{ backgroundColor: role.color }}
                    />
                    <span className="text-sm">{role.name}</span>
                    {role.is_default && (
                      <span className="text-xs text-gray-500">(défaut)</span>
                    )}
                  </label>
                );
              })}
            </div>
          </div>
        </div>
      )}
    </div>
  );
};




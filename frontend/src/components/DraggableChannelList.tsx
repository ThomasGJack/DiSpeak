import React, { useState } from 'react';
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
} from '@dnd-kit/sortable';
import {
  useSortable,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';

type Channel = {
  id: string;
  name: string;
  type: 'text' | 'voice';
  position: number;
  category_id?: string | null;
  description?: string | null;
};

type Category = {
  id: string;
  name: string;
  position: number;
};

interface DraggableChannelListProps {
  channels: Channel[];
  categories: Category[];
  onChannelMove?: (channelId: string, newPosition: number, categoryId?: string) => void | Promise<void>;
  onCategoryMove?: (categoryId: string, newPosition: number) => void | Promise<void>;
  onReorderChannels?: (categoryId: string | null, orderedChannelIds: string[]) => void | Promise<void>;
  onReorderCategories?: (orderedCategoryIds: string[]) => void | Promise<void>;
  onChannelSelect?: (channelId: string) => void;
  onChannelUpdate?: (channel: Channel) => void;
  onChannelDelete?: (channelId: string) => void;
  onCategoryUpdate?: (category: Category) => void;
  onCategoryDelete?: (categoryId: string) => void;
  canManageChannels?: boolean;
}

interface DraggableChannelItemProps {
  channel: Channel;
  onSelect?: (channelId: string) => void;
  onUpdate?: (channel: Channel) => void;
  onDelete?: (channelId: string) => void;
  canManage?: boolean;
}

interface DraggableCategoryItemProps {
  category: Category;
  channels: Channel[];
  onChannelMove?: (channelId: string, newPosition: number, categoryId?: string) => void | Promise<void>;
  onChannelSelect?: (channelId: string) => void;
  onChannelUpdate?: (channel: Channel) => void;
  onChannelDelete?: (channelId: string) => void;
  onCategoryUpdate?: (category: Category) => void;
  onCategoryDelete?: (categoryId: string) => void;
  canManage?: boolean;
}

const DraggableChannelItem: React.FC<DraggableChannelItemProps> = ({
  channel,
  onSelect,
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
  } = useSortable({ id: channel.id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={`flex items-center justify-between p-2 rounded-md hover:bg-gray-700 transition-colors cursor-pointer ${
        isDragging ? 'bg-gray-600' : 'bg-gray-800'
      }`}
      onClick={() => onSelect?.(channel.id)}
    >
      <div className="flex items-center space-x-2 flex-1">
        {canManage && (
          <div
            {...attributes}
            {...listeners}
            className="cursor-grab hover:cursor-grabbing text-gray-400 hover:text-white p-1"
          >
            ⋮⋮
          </div>
        )}
        <span className={channel.type === 'text' ? 'text-blue-400' : 'text-green-400'}>
          {channel.type === 'text' ? '💬' : '🎤'}
        </span>
        <span className="text-white font-medium">{channel.name}</span>
        {channel.description && (
          <span className="text-gray-400 text-sm truncate">{channel.description}</span>
        )}
      </div>
      {canManage && (
        <div className="flex space-x-1">
          <button
            onClick={(e) => {
              e.stopPropagation();
              onUpdate?.(channel);
            }}
            className="px-2 py-1 text-xs text-yellow-400 hover:text-yellow-300 transition-colors"
          >
            ✏️
          </button>
          <button
            onClick={(e) => {
              e.stopPropagation();
              onDelete?.(channel.id);
            }}
            className="px-2 py-1 text-xs text-red-400 hover:text-red-300 transition-colors"
          >
            🗑️
          </button>
        </div>
      )}
    </div>
  );
};

const DraggableCategoryItem: React.FC<DraggableCategoryItemProps> = ({
  category,
  channels,
  onChannelMove,
  onChannelSelect,
  onChannelUpdate,
  onChannelDelete,
  onCategoryUpdate,
  onCategoryDelete,
  canManage = false,
}) => {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: `category-${category.id}` });

  const [isExpanded, setIsExpanded] = useState(true);

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  };

  const categoryChannels = channels
    .filter(channel => channel.category_id === category.id)
    .sort((a, b) => a.position - b.position);

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={`bg-gray-900 rounded-lg p-3 mb-2 ${
        isDragging ? 'bg-gray-700' : ''
      }`}
    >
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center space-x-2">
          {canManage && (
            <div
              {...attributes}
              {...listeners}
              className="cursor-grab hover:cursor-grabbing text-gray-400 hover:text-white p-1"
            >
              ⋮⋮
            </div>
          )}
          <button
            onClick={() => setIsExpanded(!isExpanded)}
            className="text-gray-400 hover:text-white transition-colors"
          >
            {isExpanded ? '📁' : '📂'}
          </button>
          <span className="text-white font-semibold">{category.name}</span>
          <span className="text-gray-400 text-sm">({categoryChannels.length})</span>
        </div>
        {canManage && (
          <div className="flex space-x-1">
            <button
              onClick={() => onCategoryUpdate?.(category)}
              className="px-2 py-1 text-xs text-yellow-400 hover:text-yellow-300 transition-colors"
            >
              ✏️
            </button>
            <button
              onClick={() => onCategoryDelete?.(category.id)}
              className="px-2 py-1 text-xs text-red-400 hover:text-red-300 transition-colors"
            >
              🗑️
            </button>
          </div>
        )}
      </div>
      
      {isExpanded && (
        <div className="ml-6 space-y-1">
          <SortableContext items={categoryChannels.map(c => c.id)} strategy={verticalListSortingStrategy}>
            {categoryChannels.map((channel) => (
              <DraggableChannelItem
                key={channel.id}
                channel={channel}
                onSelect={onChannelSelect}
                onUpdate={onChannelUpdate}
                onDelete={onChannelDelete}
                canManage={canManage}
              />
            ))}
          </SortableContext>
        </div>
      )}
    </div>
  );
};

export const DraggableChannelList: React.FC<DraggableChannelListProps> = ({
  channels,
  categories,
  onChannelMove,
  onCategoryMove,
  onReorderChannels,
  onReorderCategories,
  onChannelSelect,
  onChannelUpdate,
  onChannelDelete,
  onCategoryUpdate,
  onCategoryDelete,
  canManageChannels = false,
}) => {
  const [activeId, setActiveId] = useState<string | null>(null);
  const [activeType, setActiveType] = useState<'channel' | 'category' | null>(null);

  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    })
  );

  const sortedCategories = [...(categories || [])].sort((a, b) => a.position - b.position);
  const uncategorizedChannels = (channels || [])
    .filter(channel => !channel.category_id)
    .sort((a, b) => a.position - b.position);

  const getOrderedChannelsInScope = (categoryId: string | null) =>
    (channels || [])
      .filter(channel => (channel.category_id ?? null) === categoryId)
      .sort((a, b) => a.position - b.position);

  const handleDragStart = (event: DragStartEvent) => {
    const { active } = event;
    setActiveId(active.id as string);
    
    if (active.id.toString().startsWith('category-')) {
      setActiveType('category');
    } else {
      setActiveType('channel');
    }
  };

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    
    if (!over) {
      setActiveId(null);
      setActiveType(null);
      return;
    }

    if (activeType === 'category') {
      const categoryId = active.id.toString().replace('category-', '');
      const overId = over.id.toString();
      
      if (overId.startsWith('category-')) {
        const overCategoryId = overId.replace('category-', '');
        const sourceIndex = sortedCategories.findIndex(c => c.id === categoryId);
        const targetIndex = sortedCategories.findIndex(c => c.id === overCategoryId);

        if (sourceIndex >= 0 && targetIndex >= 0 && sourceIndex !== targetIndex) {
          const reordered = arrayMove(sortedCategories, sourceIndex, targetIndex);
          const orderedCategoryIds = reordered.map(category => category.id);

          if (onReorderCategories) {
            void onReorderCategories(orderedCategoryIds);
          } else if (onCategoryMove) {
            const overCategory = categories.find(c => c.id === overCategoryId);
            if (overCategory) {
              void onCategoryMove(categoryId, overCategory.position);
            }
          }
        }
      }
    } else if (activeType === 'channel') {
      const channelId = active.id as string;
      const overId = over.id.toString();
      const activeChannel = channels.find(c => c.id === channelId);
      if (!activeChannel) {
        setActiveId(null);
        setActiveType(null);
        return;
      }
      
      if (overId.startsWith('category-')) {
        // Déplacer vers une catégorie
        const categoryId = overId.replace('category-', '');
        const targetScope = getOrderedChannelsInScope(categoryId);
        const newPosition = targetScope.length;
        if ((activeChannel.category_id ?? null) !== categoryId && onChannelMove) {
          void onChannelMove(channelId, newPosition, categoryId);
        }
      } else {
        // Déplacer vers un autre canal
        const targetChannel = channels.find(c => c.id === overId);
        if (targetChannel) {
          const sourceCategoryId = activeChannel.category_id ?? null;
          const targetCategoryId = targetChannel.category_id ?? null;

          if (sourceCategoryId === targetCategoryId && onReorderChannels) {
            const scopeChannels = getOrderedChannelsInScope(sourceCategoryId);
            const sourceIndex = scopeChannels.findIndex(c => c.id === channelId);
            const targetIndex = scopeChannels.findIndex(c => c.id === targetChannel.id);

            if (sourceIndex >= 0 && targetIndex >= 0 && sourceIndex !== targetIndex) {
              const reordered = arrayMove(scopeChannels, sourceIndex, targetIndex);
              void onReorderChannels(
                sourceCategoryId,
                reordered.map(channel => channel.id)
              );
            }
          } else if (onChannelMove) {
            void onChannelMove(channelId, targetChannel.position, targetChannel.category_id ?? undefined);
          }
        }
      }
    }

    setActiveId(null);
    setActiveType(null);
  };

  const renderDragOverlay = () => {
    if (!activeId || !activeType) return null;

    if (activeType === 'category') {
      const category = categories.find(c => c.id === activeId);
      if (category) {
        return (
          <div className="bg-gray-700 rounded-lg p-3 opacity-90">
            <div className="flex items-center space-x-2">
              <span>📁</span>
              <span className="text-white font-semibold">{category.name}</span>
            </div>
          </div>
        );
      }
    } else {
      const channel = channels.find(c => c.id === activeId);
      if (channel) {
        return (
          <div className="bg-gray-700 rounded-lg p-2 opacity-90">
            <div className="flex items-center space-x-2">
              <span className={channel.type === 'text' ? 'text-blue-400' : 'text-green-400'}>
                {channel.type === 'text' ? '💬' : '🎤'}
              </span>
              <span className="text-white font-medium">{channel.name}</span>
            </div>
          </div>
        );
      }
    }

    return null;
  };

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      onDragStart={handleDragStart}
      onDragEnd={handleDragEnd}
    >
      <div className="space-y-2">
        <SortableContext items={sortedCategories.map(c => `category-${c.id}`)} strategy={verticalListSortingStrategy}>
          {sortedCategories.map((category) => (
            <DraggableCategoryItem
              key={category.id}
              category={category}
              channels={channels}
              onChannelMove={onChannelMove}
              onChannelSelect={onChannelSelect}
              onChannelUpdate={onChannelUpdate}
              onChannelDelete={onChannelDelete}
              onCategoryUpdate={onCategoryUpdate}
              onCategoryDelete={onCategoryDelete}
              canManage={canManageChannels}
            />
          ))}
        </SortableContext>

        {uncategorizedChannels.length > 0 && (
          <div className="bg-gray-900 rounded-lg p-3">
            <div className="flex items-center space-x-2 mb-2">
              <span className="text-gray-400">📁</span>
              <span className="text-white font-semibold">Sans catégorie</span>
              <span className="text-gray-400 text-sm">({uncategorizedChannels.length})</span>
            </div>
            <div className="ml-6 space-y-1">
              <SortableContext items={uncategorizedChannels.map(c => c.id)} strategy={verticalListSortingStrategy}>
                {uncategorizedChannels.map((channel) => (
                  <DraggableChannelItem
                    key={channel.id}
                    channel={channel}
                    onSelect={onChannelSelect}
                    onUpdate={onChannelUpdate}
                    onDelete={onChannelDelete}
                    canManage={canManageChannels}
                  />
                ))}
              </SortableContext>
            </div>
          </div>
        )}
      </div>

      <DragOverlay>
        {renderDragOverlay()}
      </DragOverlay>
    </DndContext>
  );
};

import { useEffect, useState } from 'react';
import { useParams } from 'react-router-dom';
import { invoke, Feature } from '../lib/api';
import {
  DndContext,
  DragOverlay,
  closestCorners,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  DragStartEvent,
  DragEndEvent,
  DragOverEvent,
} from '@dnd-kit/core';
import {
  SortableContext,
  sortableKeyboardCoordinates,
  verticalListSortingStrategy,
  useSortable,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { GripVertical, Plus, Calendar, AlertTriangle, Filter, X, Sparkles, LayoutGrid } from 'lucide-react';
import { Link } from 'react-router-dom';
import FeatureCreateModal from '../components/FeatureCreateModal';
import FeatureDetailModal from '../components/FeatureDetailModal';
import SearchInput from '../components/SearchInput';
import { KanbanBoardSkeleton } from '../components/Skeleton';
import { useKanbanShortcuts } from '../hooks/useKeyboardShortcuts';

const COLUMNS = [
  { id: 'pending', label: 'To Do', color: 'border-gray-500' },
  { id: 'in_progress', label: 'In Progress', color: 'border-accent-amber' },
  { id: 'complete', label: 'Complete', color: 'border-accent-teal' },
  { id: 'blocked', label: 'Blocked', color: 'border-accent-magenta' },
];

export default function Kanban() {
  const { projectId } = useParams<{ projectId: string }>();
  const [features, setFeatures] = useState<Feature[]>([]);
  const [loading, setLoading] = useState(true);
  const [activeFeature, setActiveFeature] = useState<Feature | null>(null);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [selectedFeature, setSelectedFeature] = useState<Feature | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [priorityFilter, setPriorityFilter] = useState<number[]>([]);
  const [showFilters, setShowFilters] = useState(false);

  // Keyboard shortcuts
  useKanbanShortcuts({
    onNewFeature: () => setShowCreateModal(true),
    onSearch: () => document.querySelector<HTMLInputElement>('input[placeholder="Search features..."]')?.focus(),
  });

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 8,
      },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    })
  );

  async function loadFeatures() {
    if (!projectId) return;
    try {
      const data = await invoke<Feature[]>('get_features', { projectId });
      setFeatures(data);
    } catch (error) {
      console.error('Failed to load features:', error);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    loadFeatures();
  }, [projectId]);

  function handleFeatureCreated(feature: Feature) {
    setFeatures((prev) => [...prev, feature]);
    setShowCreateModal(false);
  }

  function handleFeatureUpdated(updated: Feature) {
    setFeatures((prev) => prev.map((f) => (f.id === updated.id ? updated : f)));
    setSelectedFeature(null);
  }

  function handleFeatureDeleted(featureId: string) {
    setFeatures((prev) => prev.filter((f) => f.id !== featureId));
    setSelectedFeature(null);
  }

  function handleFeatureClick(feature: Feature) {
    setSelectedFeature(feature);
  }

  // Apply filters to features
  const filteredFeatures = features.filter((f) => {
    // Search filter
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      const matchesName = f.name.toLowerCase().includes(query);
      const matchesDesc = f.description?.toLowerCase().includes(query);
      const matchesTags = f.tags?.some((t) => t.toLowerCase().includes(query));
      if (!matchesName && !matchesDesc && !matchesTags) {
        return false;
      }
    }
    // Priority filter
    if (priorityFilter.length > 0 && !priorityFilter.includes(f.priority)) {
      return false;
    }
    return true;
  });

  const featuresByStatus = COLUMNS.reduce((acc, column) => {
    acc[column.id] = filteredFeatures.filter((f) => f.status === column.id);
    return acc;
  }, {} as Record<string, Feature[]>);

  const hasActiveFilters = searchQuery || priorityFilter.length > 0;

  function clearFilters() {
    setSearchQuery('');
    setPriorityFilter([]);
  }

  function togglePriorityFilter(priority: number) {
    if (priorityFilter.includes(priority)) {
      setPriorityFilter(priorityFilter.filter((p) => p !== priority));
    } else {
      setPriorityFilter([...priorityFilter, priority]);
    }
  }

  function handleDragStart(event: DragStartEvent) {
    const { active } = event;
    const feature = features.find((f) => f.id === active.id);
    setActiveFeature(feature || null);
  }

  function handleDragOver(event: DragOverEvent) {
    const { active, over } = event;
    if (!over) return;

    const activeFeature = features.find((f) => f.id === active.id);
    if (!activeFeature) return;

    // Check if we're dragging over a column
    const overColumn = COLUMNS.find((c) => c.id === over.id);
    if (overColumn && activeFeature.status !== overColumn.id) {
      setFeatures((prev) =>
        prev.map((f) =>
          f.id === activeFeature.id ? { ...f, status: overColumn.id } : f
        )
      );
    }

    // Check if we're dragging over another feature
    const overFeature = features.find((f) => f.id === over.id);
    if (overFeature && activeFeature.status !== overFeature.status) {
      setFeatures((prev) =>
        prev.map((f) =>
          f.id === activeFeature.id ? { ...f, status: overFeature.status } : f
        )
      );
    }
  }

  async function handleDragEnd(event: DragEndEvent) {
    const { active } = event;
    setActiveFeature(null);

    // Update the feature status in the backend
    const feature = features.find((f) => f.id === active.id);
    if (feature) {
      try {
        await invoke('update_feature_status', {
          id: feature.id,
          status: feature.status,
        });
      } catch (error) {
        console.error('Failed to update feature status:', error);
      }
    }
  }

  if (loading) {
    return (
      <div className="p-6 h-full flex flex-col">
        <div className="flex justify-between items-center mb-6">
          <div className="h-8 w-40 bg-background-surface rounded animate-pulse" />
          <div className="h-10 w-32 bg-background-surface rounded animate-pulse" />
        </div>
        <KanbanBoardSkeleton />
      </div>
    );
  }

  return (
    <div className="p-6 h-full flex flex-col">
      <div className="flex justify-between items-center mb-4">
        <h1 className="text-2xl font-bold">Kanban Board</h1>
        <button
          onClick={() => setShowCreateModal(true)}
          className="flex items-center gap-2 px-4 py-2 bg-accent-teal text-background-deep rounded-lg font-medium hover:bg-accent-teal/90 transition-colors"
        >
          <Plus className="w-5 h-5" />
          Add Feature
        </button>
      </div>

      {/* Filter Bar */}
      <div className="flex items-center gap-3 mb-4">
        <SearchInput
          value={searchQuery}
          onChange={setSearchQuery}
          placeholder="Search features..."
          className="w-64"
        />

        <button
          onClick={() => setShowFilters(!showFilters)}
          className={`flex items-center gap-2 px-3 py-2 rounded-lg transition-colors ${
            showFilters || priorityFilter.length > 0
              ? 'bg-accent-teal/10 text-accent-teal'
              : 'bg-background-surface text-gray-400 hover:text-white'
          }`}
        >
          <Filter className="w-4 h-4" />
          Filters
          {priorityFilter.length > 0 && (
            <span className="ml-1 px-1.5 py-0.5 text-xs bg-accent-teal text-background-deep rounded-full">
              {priorityFilter.length}
            </span>
          )}
        </button>

        {hasActiveFilters && (
          <button
            onClick={clearFilters}
            className="flex items-center gap-1 px-3 py-2 text-sm text-gray-400 hover:text-white transition-colors"
          >
            <X className="w-4 h-4" />
            Clear filters
          </button>
        )}

        {hasActiveFilters && (
          <span className="text-sm text-gray-500">
            Showing {filteredFeatures.length} of {features.length} features
          </span>
        )}
      </div>

      {/* Priority Filter Pills */}
      {showFilters && (
        <div className="flex items-center gap-2 mb-4 p-3 bg-background-surface rounded-lg">
          <span className="text-sm text-gray-400 mr-2">Priority:</span>
          {[
            { value: 0, label: 'P0', color: 'border-red-400 text-red-400' },
            { value: 1, label: 'P1', color: 'border-accent-magenta text-accent-magenta' },
            { value: 2, label: 'P2', color: 'border-accent-amber text-accent-amber' },
            { value: 3, label: 'P3', color: 'border-accent-teal text-accent-teal' },
            { value: 4, label: 'P4', color: 'border-gray-400 text-gray-400' },
          ].map((p) => (
            <button
              key={p.value}
              onClick={() => togglePriorityFilter(p.value)}
              className={`px-3 py-1 text-sm border rounded-full transition-colors ${
                priorityFilter.includes(p.value)
                  ? `${p.color} bg-current/10`
                  : 'border-gray-600 text-gray-500 hover:border-gray-400 hover:text-gray-300'
              }`}
            >
              {p.label}
            </button>
          ))}
        </div>
      )}

      {/* Empty State */}
      {features.length === 0 && (
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center max-w-md">
            <LayoutGrid className="w-16 h-16 mx-auto text-gray-500 mb-4" />
            <h3 className="text-lg font-medium text-gray-300 mb-2">No features yet</h3>
            <p className="text-gray-500 mb-4">
              Create features manually or extract them automatically from your PRD.
            </p>
            <div className="flex gap-3 justify-center">
              <button
                onClick={() => setShowCreateModal(true)}
                className="inline-flex items-center gap-2 px-4 py-2 bg-accent-teal text-background-deep rounded-lg font-medium hover:bg-accent-teal/90 transition-colors"
              >
                <Plus className="w-5 h-5" />
                Add Feature
              </button>
              <Link
                to={`/projects/${projectId}`}
                className="inline-flex items-center gap-2 px-4 py-2 bg-background-surface text-gray-300 hover:text-white rounded-lg transition-colors"
              >
                <Sparkles className="w-5 h-5" />
                Extract from PRD
              </Link>
            </div>
            <p className="text-xs text-gray-600 mt-4">
              Press <kbd className="px-1.5 py-0.5 bg-background-surface rounded text-gray-400">n</kbd> to quickly add a new feature
            </p>
          </div>
        </div>
      )}

      {features.length > 0 && (
        <DndContext
        sensors={sensors}
        collisionDetection={closestCorners}
        onDragStart={handleDragStart}
        onDragOver={handleDragOver}
        onDragEnd={handleDragEnd}
      >
        <div className="flex-1 grid grid-cols-4 gap-4 overflow-hidden">
          {COLUMNS.map((column) => (
            <KanbanColumn
              key={column.id}
              column={column}
              features={featuresByStatus[column.id] || []}
              onFeatureClick={handleFeatureClick}
            />
          ))}
        </div>

        <DragOverlay>
          {activeFeature ? (
            <FeatureCard feature={activeFeature} isDragging />
          ) : null}
        </DragOverlay>
      </DndContext>
      )}

      {/* Modals */}
      {showCreateModal && projectId && (
        <FeatureCreateModal
          projectId={projectId}
          onClose={() => setShowCreateModal(false)}
          onCreated={handleFeatureCreated}
        />
      )}

      {selectedFeature && (
        <FeatureDetailModal
          feature={selectedFeature}
          onClose={() => setSelectedFeature(null)}
          onUpdated={handleFeatureUpdated}
          onDeleted={handleFeatureDeleted}
        />
      )}
    </div>
  );
}

function KanbanColumn({
  column,
  features,
  onFeatureClick,
}: {
  column: { id: string; label: string; color: string };
  features: Feature[];
  onFeatureClick: (feature: Feature) => void;
}) {
  return (
    <div
      className="flex flex-col bg-background-mid rounded-lg border border-background-surface overflow-hidden"
    >
      {/* Column Header */}
      <div className={`p-3 border-b-2 ${column.color} bg-background-surface`}>
        <div className="flex items-center justify-between">
          <h3 className="font-semibold">{column.label}</h3>
          <span className="text-sm text-gray-400">{features.length}</span>
        </div>
      </div>

      {/* Cards Container */}
      <SortableContext
        items={features.map((f) => f.id)}
        strategy={verticalListSortingStrategy}
        id={column.id}
      >
        <div className="flex-1 p-2 space-y-2 overflow-y-auto min-h-[100px]">
          {features.map((feature) => (
            <SortableFeatureCard key={feature.id} feature={feature} onClick={() => onFeatureClick(feature)} />
          ))}
          {features.length === 0 && (
            <div className="text-center text-gray-500 text-sm py-4">
              Drop items here
            </div>
          )}
        </div>
      </SortableContext>
    </div>
  );
}

function SortableFeatureCard({ feature, onClick }: { feature: Feature; onClick: () => void }) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: feature.id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  };

  return (
    <div ref={setNodeRef} style={style} {...attributes} {...listeners}>
      <FeatureCard feature={feature} onClick={onClick} />
    </div>
  );
}

function FeatureCard({
  feature,
  isDragging = false,
  onClick,
}: {
  feature: Feature;
  isDragging?: boolean;
  onClick?: () => void;
}) {
  const priorityColors: Record<number, string> = {
    0: 'border-l-red-500',
    1: 'border-l-accent-magenta',
    2: 'border-l-accent-amber',
    3: 'border-l-accent-teal',
    4: 'border-l-gray-400',
  };

  const isOverdue = feature.due_date && new Date(feature.due_date) < new Date() && feature.status !== 'complete';

  return (
    <div
      onClick={onClick}
      className={`bg-background-deep rounded-lg border border-background-surface p-3 cursor-grab border-l-4 ${
        priorityColors[feature.priority] || priorityColors[3]
      } hover:border-accent-teal/50 transition-colors group ${
        isDragging ? 'shadow-lg shadow-accent-teal/20 ring-2 ring-accent-teal' : ''
      } ${isOverdue ? 'ring-1 ring-red-500/50' : ''}`}
    >
      <div className="flex items-start gap-2">
        <GripVertical className="w-4 h-4 text-gray-500 opacity-0 group-hover:opacity-100 transition-opacity flex-shrink-0 mt-0.5" />
        <div className="flex-1 min-w-0">
          <h4 className="font-medium text-sm truncate">{feature.name}</h4>
          {feature.description && (
            <p className="text-xs text-gray-400 mt-1 line-clamp-2">
              {feature.description}
            </p>
          )}
          <div className="flex items-center gap-2 mt-2 flex-wrap">
            <span className="text-xs text-gray-500">P{feature.priority}</span>
            {feature.due_date && (
              <span className={`text-xs flex items-center gap-1 ${isOverdue ? 'text-red-400' : 'text-gray-500'}`}>
                {isOverdue && <AlertTriangle className="w-3 h-3" />}
                <Calendar className="w-3 h-3" />
                {new Date(feature.due_date).toLocaleDateString(undefined, { month: 'short', day: 'numeric' })}
              </span>
            )}
            {feature.tags && feature.tags.length > 0 && (
              <div className="flex gap-1">
                {feature.tags.slice(0, 2).map((tag) => (
                  <span key={tag} className="px-1.5 py-0.5 bg-accent-teal/10 text-accent-teal text-xs rounded">
                    {tag}
                  </span>
                ))}
                {feature.tags.length > 2 && (
                  <span className="text-xs text-gray-500">+{feature.tags.length - 2}</span>
                )}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

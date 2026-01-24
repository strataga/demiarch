import { useEffect, useState, useCallback } from 'react';
import { useParams } from 'react-router-dom';
import { invoke, Feature, Project } from '../lib/api';
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
import { useDroppable, useDraggable } from '@dnd-kit/core';
import { GripVertical, Plus, Calendar, AlertTriangle, Filter, X, Sparkles, LayoutGrid, Zap, Loader2 } from 'lucide-react';
import { Link } from 'react-router-dom';
import FeatureCreateModal from '../components/FeatureCreateModal';
import FeatureDetailModal from '../components/FeatureDetailModal';
import AutoBuildModal from '../components/AutoBuildModal';
import Toggle from '../components/Toggle';
import BuildLogPanel from '../components/BuildLogPanel';
import SearchInput from '../components/SearchInput';
import { KanbanBoardSkeleton } from '../components/Skeleton';
import { useKanbanShortcuts } from '../hooks/useKeyboardShortcuts';
import { useAutoBuild } from '../hooks/useAutoBuild';

const COLUMNS = [
  { id: 'pending', label: 'To Do', color: 'border-gray-500' },
  { id: 'in_progress', label: 'In Progress', color: 'border-accent-amber' },
  { id: 'complete', label: 'Complete', color: 'border-accent-teal' },
  { id: 'blocked', label: 'Blocked', color: 'border-accent-magenta' },
];

export default function Kanban() {
  const { projectId } = useParams<{ projectId: string }>();
  const [features, setFeatures] = useState<Feature[]>([]);
  const [project, setProject] = useState<Project | null>(null);
  const [loading, setLoading] = useState(true);
  const [activeFeature, setActiveFeature] = useState<Feature | null>(null);
  const [draggedToStatus, setDraggedToStatus] = useState<string | null>(null);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [showAutoBuildModal, setShowAutoBuildModal] = useState(false);
  const [selectedFeature, setSelectedFeature] = useState<Feature | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [priorityFilter, setPriorityFilter] = useState<number[]>([]);
  const [showFilters, setShowFilters] = useState(false);

  // Keyboard shortcuts
  useKanbanShortcuts({
    onNewFeature: () => setShowCreateModal(true),
    onSearch: () => document.querySelector<HTMLInputElement>('input[placeholder="Search features..."]')?.focus(),
  });

  // Auto Build hook callback
  const handleAutoBuildFeatureUpdated = useCallback((updated: Feature) => {
    setFeatures((prev) => prev.map((f) => (f.id === updated.id ? updated : f)));
  }, []);

  // Auto Build orchestrator
  const autoBuild = useAutoBuild(
    projectId || '',
    project?.name || '',
    project?.framework || 'react',
    features,
    handleAutoBuildFeatureUpdated
  );

  // Get current building feature for display
  const currentBuildingFeature = autoBuild.state.currentFeatureId
    ? features.find((f) => f.id === autoBuild.state.currentFeatureId)
    : null;

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 3,
      },
    }),
    useSensor(KeyboardSensor)
  );

  async function loadFeatures() {
    if (!projectId) return;
    try {
      const [featuresData, projectData] = await Promise.all([
        invoke<Feature[]>('get_features', { projectId }),
        invoke<Project>('get_project', { id: projectId }),
      ]);
      setFeatures(featuresData);
      setProject(projectData);

      // Auto-update project status based on progress
      if (projectData && projectData.status === 'discovery') {
        const hasFeatures = featuresData.length > 0;
        const hasPrd = !!projectData.prd;
        if (hasFeatures || hasPrd) {
          // Upgrade from discovery to planning/building
          const hasInProgress = featuresData.some(f => f.status === 'in_progress');
          const hasComplete = featuresData.some(f => f.status === 'complete');
          const newStatus = hasInProgress || hasComplete ? 'building' : 'planning';
          const updated = await invoke<Project>('update_project', {
            id: projectData.id,
            status: newStatus,
          });
          setProject(updated);
        }
      }
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
    // Update selectedFeature if it's the same feature being updated (don't close modal)
    setSelectedFeature((prev) => (prev?.id === updated.id ? updated : prev));
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
    console.log('[DND] Drag started:', active.id);
    const feature = features.find((f) => f.id === active.id);
    setActiveFeature(feature || null);
  }

  function handleDragOver(event: DragOverEvent) {
    const { active, over } = event;
    if (!over) return;

    const activeId = active.id as string;

    let newStatus: string | null = null;

    // Check if we're dragging over a column
    const overColumn = COLUMNS.find((c) => c.id === over.id);
    if (overColumn) {
      newStatus = overColumn.id;
    }

    // Check if we're dragging over another feature (get its column)
    if (!newStatus) {
      // Use functional update to get the current status of the over feature
      const overFeature = features.find((f) => f.id === over.id);
      if (overFeature) {
        newStatus = overFeature.status;
      }
    }

    // Get the current status of the dragged feature (use tracked status if available)
    const currentStatus = draggedToStatus || features.find((f) => f.id === activeId)?.status;

    // Update the feature's status if it changed
    if (newStatus && currentStatus !== newStatus) {
      console.log('[DND] Status changing:', currentStatus, '->', newStatus);
      setDraggedToStatus(newStatus);
      setFeatures((prev) =>
        prev.map((f) =>
          f.id === activeId ? { ...f, status: newStatus! } : f
        )
      );
    }
  }

  async function handleDragEnd(event: DragEndEvent) {
    const { active } = event;
    console.log('[DND] Drag ended:', active.id, 'newStatus:', draggedToStatus);
    setActiveFeature(null);

    // Use the tracked status from draggedToStatus, not the stale closure state
    if (draggedToStatus) {
      try {
        await invoke('update_feature_status', {
          id: active.id,
          status: draggedToStatus,
        });
        console.log('[DND] Status saved successfully');
      } catch (error) {
        console.error('Failed to update feature status:', error);
      }
    }

    // Reset the tracked status
    setDraggedToStatus(null);
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
        <div className="flex items-center gap-3">
          {/* Auto Build Toggle */}
          {features.length > 0 && (
            <div className="flex items-center gap-3">
              <div className="flex items-center gap-2">
                <span className="text-sm text-gray-400">Auto Build</span>
                <Toggle
                  checked={autoBuild.state.enabled}
                  onChange={autoBuild.toggle}
                  disabled={autoBuild.state.processing && !autoBuild.state.enabled}
                />
              </div>
              {autoBuild.state.processing && currentBuildingFeature && (
                <span className="text-sm text-accent-amber flex items-center gap-1.5">
                  <Loader2 className="w-4 h-4 animate-spin" />
                  Building: {currentBuildingFeature.name}
                </span>
              )}
              {/* Manual Build button (opens modal) */}
              <button
                onClick={() => setShowAutoBuildModal(true)}
                className="flex items-center gap-1.5 px-3 py-1.5 text-sm bg-background-surface text-gray-300 hover:text-white rounded-lg transition-colors"
                title="Manual build selection"
              >
                <Zap className="w-4 h-4" />
                Manual
              </button>
            </div>
          )}
          <button
            onClick={() => setShowCreateModal(true)}
            className="flex items-center gap-2 px-4 py-2 bg-accent-teal text-background-deep rounded-lg font-medium hover:bg-accent-teal/90 transition-colors"
          >
            <Plus className="w-5 h-5" />
            Add Feature
          </button>
        </div>
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

      {/* Build Log Panel */}
      <BuildLogPanel
        logs={autoBuild.state.logs}
        onClear={autoBuild.clearLogs}
        isProcessing={autoBuild.state.processing}
      />

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
          projectName={project?.name}
          onRetry={async (feature) => {
            // Move feature back to pending for retry
            try {
              await invoke('update_feature_status', {
                id: feature.id,
                status: 'pending',
              });
              const updated = { ...feature, status: 'pending' };
              handleFeatureUpdated(updated);
              setSelectedFeature(null);
              // If Auto Build is enabled, it will pick up the feature automatically
              // Otherwise, show a message
              if (!autoBuild.state.enabled) {
                console.log('Feature moved to pending. Enable Auto Build to retry, or use Manual build.');
              }
            } catch (error) {
              console.error('Failed to retry feature:', error);
            }
          }}
        />
      )}

      {showAutoBuildModal && project && (
        <AutoBuildModal
          features={features}
          projectName={project.name}
          framework={project.framework}
          onClose={() => setShowAutoBuildModal(false)}
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
  const { setNodeRef, isOver } = useDroppable({
    id: column.id,
  });

  return (
    <div
      ref={setNodeRef}
      className={`flex flex-col bg-background-mid rounded-lg border border-background-surface overflow-hidden transition-colors ${
        isOver ? 'border-accent-teal bg-accent-teal/5' : ''
      }`}
    >
      {/* Column Header */}
      <div className={`p-3 border-b-2 ${column.color} bg-background-surface`}>
        <div className="flex items-center justify-between">
          <h3 className="font-semibold">{column.label}</h3>
          <span className="text-sm text-gray-400">{features.length}</span>
        </div>
      </div>

      {/* Cards Container */}
      <div className="flex-1 p-2 space-y-2 overflow-y-auto min-h-[100px]">
        {features.map((feature) => (
          <DraggableFeatureCard key={feature.id} feature={feature} onClick={() => onFeatureClick(feature)} />
        ))}
        {features.length === 0 && (
          <div className="text-center text-gray-500 text-sm py-4">
            Drop items here
          </div>
        )}
      </div>
    </div>
  );
}

function DraggableFeatureCard({ feature, onClick }: { feature: Feature; onClick: () => void }) {
  const {
    attributes,
    listeners,
    setNodeRef,
    isDragging,
  } = useDraggable({ id: feature.id });

  const style: React.CSSProperties = {
    opacity: isDragging ? 0.3 : 1,
    touchAction: 'none', // Prevent scroll interference on touch devices
    cursor: isDragging ? 'grabbing' : 'grab',
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      {...attributes}
      {...listeners}
      onClick={(e) => {
        // Only trigger click if not dragging
        // The drag system uses onPointerDown, so onClick fires after drag ends
        if (!isDragging) {
          e.stopPropagation();
          onClick();
        }
      }}
    >
      <FeatureCard feature={feature} />
    </div>
  );
}

function FeatureCard({
  feature,
  isDragging = false,
}: {
  feature: Feature;
  isDragging?: boolean;
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
      className={`bg-background-deep rounded-lg border border-background-surface p-3 border-l-4 ${
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

import { useEffect, useState } from 'react';
import { useParams } from 'react-router-dom';
import { invoke } from '../lib/api';
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
import { GripVertical, Plus } from 'lucide-react';

interface FeatureSummary {
  id: string;
  name: string;
  description: string | null;
  status: string;
  priority: number;
  phase_id: string;
}

const COLUMNS = [
  { id: 'pending', label: 'To Do', color: 'border-gray-500' },
  { id: 'in_progress', label: 'In Progress', color: 'border-accent-amber' },
  { id: 'complete', label: 'Complete', color: 'border-accent-teal' },
  { id: 'blocked', label: 'Blocked', color: 'border-accent-magenta' },
];

export default function Kanban() {
  const { projectId } = useParams<{ projectId: string }>();
  const [features, setFeatures] = useState<FeatureSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [activeFeature, setActiveFeature] = useState<FeatureSummary | null>(null);

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

  useEffect(() => {
    async function loadFeatures() {
      if (!projectId) return;
      try {
        const data = await invoke<FeatureSummary[]>('get_features', { projectId });
        setFeatures(data);
      } catch (error) {
        console.error('Failed to load features:', error);
      } finally {
        setLoading(false);
      }
    }
    loadFeatures();
  }, [projectId]);

  const featuresByStatus = COLUMNS.reduce((acc, column) => {
    acc[column.id] = features.filter((f) => f.status === column.id);
    return acc;
  }, {} as Record<string, FeatureSummary[]>);

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
      <div className="flex items-center justify-center h-full">
        <div className="animate-pulse text-accent-teal">Loading features...</div>
      </div>
    );
  }

  return (
    <div className="p-6 h-full flex flex-col">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold">Kanban Board</h1>
        <button className="flex items-center gap-2 px-4 py-2 bg-accent-teal text-background-deep rounded-lg font-medium hover:bg-accent-teal/90 transition-colors">
          <Plus className="w-5 h-5" />
          Add Feature
        </button>
      </div>

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
            />
          ))}
        </div>

        <DragOverlay>
          {activeFeature ? (
            <FeatureCard feature={activeFeature} isDragging />
          ) : null}
        </DragOverlay>
      </DndContext>
    </div>
  );
}

function KanbanColumn({
  column,
  features,
}: {
  column: { id: string; label: string; color: string };
  features: FeatureSummary[];
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
            <SortableFeatureCard key={feature.id} feature={feature} />
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

function SortableFeatureCard({ feature }: { feature: FeatureSummary }) {
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
      <FeatureCard feature={feature} />
    </div>
  );
}

function FeatureCard({
  feature,
  isDragging = false,
}: {
  feature: FeatureSummary;
  isDragging?: boolean;
}) {
  const priorityColors: Record<number, string> = {
    1: 'border-l-accent-magenta',
    2: 'border-l-accent-amber',
    3: 'border-l-accent-teal',
    4: 'border-l-blue-400',
    5: 'border-l-gray-400',
  };

  return (
    <div
      className={`bg-background-deep rounded-lg border border-background-surface p-3 cursor-grab border-l-4 ${
        priorityColors[feature.priority] || priorityColors[3]
      } hover:border-accent-teal/50 transition-colors group ${
        isDragging ? 'shadow-lg shadow-accent-teal/20 ring-2 ring-accent-teal' : ''
      }`}
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
          <div className="flex items-center gap-2 mt-2">
            <span className="text-xs text-gray-500">P{feature.priority}</span>
          </div>
        </div>
      </div>
    </div>
  );
}

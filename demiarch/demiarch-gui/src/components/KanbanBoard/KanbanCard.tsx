import type { KanbanCard as KanbanCardType } from './types';

interface KanbanCardProps {
  card: KanbanCardType;
  onEdit?: (card: KanbanCardType) => void;
  onDelete?: (cardId: string) => void;
  isDragging?: boolean;
}

export function KanbanCard({ card, onEdit, onDelete, isDragging }: KanbanCardProps) {
  const priorityClass = card.priority ? `kanban-card--${card.priority}` : '';
  const draggingClass = isDragging ? 'kanban-card--dragging' : '';

  return (
    <div
      className={`kanban-card ${priorityClass} ${draggingClass}`}
      draggable
      onDragStart={(e) => {
        e.dataTransfer.setData('cardId', card.id);
        e.dataTransfer.effectAllowed = 'move';
      }}
    >
      <div className="kanban-card__header">
        <span className="kanban-card__title">{card.title}</span>
        {card.priority && (
          <span className={`kanban-card__priority kanban-card__priority--${card.priority}`}>
            {card.priority}
          </span>
        )}
      </div>

      {card.description && (
        <p className="kanban-card__description">{card.description}</p>
      )}

      {card.labels && card.labels.length > 0 && (
        <div className="kanban-card__labels">
          {card.labels.map((label) => (
            <span key={label} className="kanban-card__label">
              {label}
            </span>
          ))}
        </div>
      )}

      <div className="kanban-card__actions">
        {onEdit && (
          <button
            className="kanban-card__action"
            onClick={() => onEdit(card)}
            title="Edit card"
          >
            Edit
          </button>
        )}
        {onDelete && (
          <button
            className="kanban-card__action kanban-card__action--delete"
            onClick={() => onDelete(card.id)}
            title="Delete card"
          >
            Delete
          </button>
        )}
      </div>
    </div>
  );
}

export default KanbanCard;

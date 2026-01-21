import { useState } from 'react';
import type { KanbanCard as KanbanCardType } from './types';

interface KanbanCardProps {
  card: KanbanCardType;
  onEdit?: (card: KanbanCardType) => void;
  onDelete?: (cardId: string) => void;
  onAcceptanceCriterionToggle?: (cardId: string, criterionId: string) => void;
  isDragging?: boolean;
}

export function KanbanCard({
  card,
  onEdit,
  onDelete,
  onAcceptanceCriterionToggle,
  isDragging,
}: KanbanCardProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  const priorityClass = card.priority ? `kanban-card--${card.priority}` : '';
  const draggingClass = isDragging ? 'kanban-card--dragging' : '';
  const expandedClass = isExpanded ? 'kanban-card--expanded' : '';

  const hasDetails =
    card.description ||
    (card.acceptanceCriteria && card.acceptanceCriteria.length > 0);

  const completedCriteria =
    card.acceptanceCriteria?.filter((c) => c.completed).length ?? 0;
  const totalCriteria = card.acceptanceCriteria?.length ?? 0;

  const handleToggleExpand = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (hasDetails) {
      setIsExpanded(!isExpanded);
    }
  };

  const handleCriterionToggle = (e: React.MouseEvent, criterionId: string) => {
    e.stopPropagation();
    onAcceptanceCriterionToggle?.(card.id, criterionId);
  };

  return (
    <div className={`kanban-card ${priorityClass} ${draggingClass} ${expandedClass}`}>
      <div className="kanban-card__header" onClick={handleToggleExpand}>
        <span className="kanban-card__title">{card.title}</span>
        {card.priority && (
          <span className={`kanban-card__priority kanban-card__priority--${card.priority}`}>
            {card.priority}
          </span>
        )}
      </div>

      {!isExpanded && card.description && (
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

      {!isExpanded && totalCriteria > 0 && (
        <div className="kanban-card__criteria-summary" onClick={handleToggleExpand}>
          <span className="kanban-card__criteria-icon">☑</span>
          <span className="kanban-card__criteria-count">
            {completedCriteria}/{totalCriteria}
          </span>
          <div className="kanban-card__criteria-progress">
            <div
              className="kanban-card__criteria-progress-bar"
              style={{ width: `${totalCriteria > 0 ? (completedCriteria / totalCriteria) * 100 : 0}%` }}
            />
          </div>
        </div>
      )}

      {hasDetails && (
        <button
          className="kanban-card__expand-toggle"
          onClick={handleToggleExpand}
          title={isExpanded ? 'Collapse details' : 'Expand details'}
        >
          {isExpanded ? '▲' : '▼'}
        </button>
      )}

      {isExpanded && (
        <div className="kanban-card__details">
          {card.description && (
            <div className="kanban-card__details-section">
              <h4 className="kanban-card__details-heading">Description</h4>
              <p className="kanban-card__details-description">{card.description}</p>
            </div>
          )}

          {card.acceptanceCriteria && card.acceptanceCriteria.length > 0 && (
            <div className="kanban-card__details-section">
              <h4 className="kanban-card__details-heading">
                Acceptance Criteria
                <span className="kanban-card__criteria-badge">
                  {completedCriteria}/{totalCriteria}
                </span>
              </h4>
              <ul className="kanban-card__criteria-list">
                {card.acceptanceCriteria.map((criterion) => (
                  <li
                    key={criterion.id}
                    className={`kanban-card__criterion ${criterion.completed ? 'kanban-card__criterion--completed' : ''}`}
                    onClick={(e) => handleCriterionToggle(e, criterion.id)}
                  >
                    <span className="kanban-card__criterion-checkbox">
                      {criterion.completed ? '☑' : '☐'}
                    </span>
                    <span className="kanban-card__criterion-text">{criterion.text}</span>
                  </li>
                ))}
              </ul>
            </div>
          )}
        </div>
      )}

      <div className="kanban-card__actions">
        {onEdit && (
          <button
            className="kanban-card__action"
            onClick={(e) => {
              e.stopPropagation();
              onEdit(card);
            }}
            title="Edit card"
          >
            Edit
          </button>
        )}
        {onDelete && (
          <button
            className="kanban-card__action kanban-card__action--delete"
            onClick={(e) => {
              e.stopPropagation();
              onDelete(card.id);
            }}
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

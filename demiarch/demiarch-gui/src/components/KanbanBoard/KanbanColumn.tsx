import { useState, useRef } from 'react';
import { KanbanCard } from './KanbanCard';
import type { KanbanColumn as KanbanColumnType, KanbanCard as KanbanCardType } from './types';
import { MAX_TITLE_LENGTH } from './types';

interface KanbanColumnProps {
  column: KanbanColumnType;
  onCardMove?: (cardId: string, sourceColumnId: string, targetColumnId: string, targetIndex?: number) => void;
  onCardEdit?: (card: KanbanCardType) => void;
  onCardDelete?: (cardId: string, columnId: string) => void;
  onAddCard?: (columnId: string, title: string) => void;
  onEditColumn?: (columnId: string, title: string) => void;
  onDeleteColumn?: (columnId: string) => void;
  onAcceptanceCriterionToggle?: (cardId: string, criterionId: string) => void;
  draggingCardId?: string | null;
  onDragStart?: (cardId: string) => void;
  onDragEnd?: () => void;
}

export function KanbanColumn({
  column,
  onCardMove,
  onCardEdit,
  onCardDelete,
  onAddCard,
  onEditColumn,
  onDeleteColumn,
  onAcceptanceCriterionToggle,
  draggingCardId,
  onDragStart,
  onDragEnd,
}: KanbanColumnProps) {
  const [isAddingCard, setIsAddingCard] = useState(false);
  const [newCardTitle, setNewCardTitle] = useState('');
  const [isDragOver, setIsDragOver] = useState(false);
  const [isEditingTitle, setIsEditingTitle] = useState(false);
  const [editTitle, setEditTitle] = useState(column.title);
  const [dropTargetIndex, setDropTargetIndex] = useState<number | null>(null);
  const cardsContainerRef = useRef<HTMLDivElement>(null);

  const isOverLimit = column.limit !== undefined && column.cards.length >= column.limit;

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
    setIsDragOver(true);
  };

  const handleDragLeave = (e: React.DragEvent) => {
    // Only set drag over false if leaving the column entirely
    const relatedTarget = e.relatedTarget as Node | null;
    if (!e.currentTarget.contains(relatedTarget)) {
      setIsDragOver(false);
      setDropTargetIndex(null);
    }
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragOver(false);
    setDropTargetIndex(null);
    const cardId = e.dataTransfer.getData('cardId');
    const sourceColumnId = e.dataTransfer.getData('sourceColumnId');
    if (cardId && sourceColumnId && onCardMove) {
      onCardMove(cardId, sourceColumnId, column.id, dropTargetIndex ?? undefined);
    }
  };

  const handleCardDragOver = (e: React.DragEvent, index: number) => {
    e.preventDefault();
    e.stopPropagation();
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const midY = rect.top + rect.height / 2;
    const newIndex = e.clientY < midY ? index : index + 1;
    setDropTargetIndex(newIndex);
    setIsDragOver(true);
  };

  const handleAddCard = () => {
    if (newCardTitle.trim() && onAddCard) {
      onAddCard(column.id, newCardTitle.trim());
      setNewCardTitle('');
      setIsAddingCard(false);
    }
  };

  const handleSaveTitle = () => {
    if (editTitle.trim() && onEditColumn) {
      onEditColumn(column.id, editTitle.trim());
    }
    setIsEditingTitle(false);
  };

  return (
    <div
      className={`kanban-column ${isDragOver ? 'kanban-column--drag-over' : ''} ${isOverLimit ? 'kanban-column--over-limit' : ''}`}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      style={{ '--column-color': column.color } as React.CSSProperties}
    >
      <div className="kanban-column__header">
        {isEditingTitle ? (
          <input
            type="text"
            className="kanban-column__title-input"
            value={editTitle}
            onChange={(e) => setEditTitle(e.target.value)}
            onBlur={handleSaveTitle}
            onKeyDown={(e) => {
              if (e.key === 'Enter') handleSaveTitle();
              if (e.key === 'Escape') {
                setEditTitle(column.title);
                setIsEditingTitle(false);
              }
            }}
            maxLength={MAX_TITLE_LENGTH}
            autoFocus
          />
        ) : (
          <h3
            className="kanban-column__title"
            onDoubleClick={() => onEditColumn && setIsEditingTitle(true)}
          >
            <span
              className="kanban-column__color-dot"
              style={{ backgroundColor: column.color }}
            />
            {column.title}
            <span className="kanban-column__count">
              {column.cards.length}
              {column.limit && ` / ${column.limit}`}
            </span>
          </h3>
        )}
        {onDeleteColumn && (
          <button
            className="kanban-column__delete"
            onClick={() => onDeleteColumn(column.id)}
            title="Delete column"
          >
            ×
          </button>
        )}
      </div>

      <div className="kanban-column__cards" ref={cardsContainerRef}>
        {column.cards.map((card, index) => (
          <div key={card.id}>
            {dropTargetIndex === index && (
              <div className="kanban-column__drop-indicator" />
            )}
            <div
              className={`kanban-card-wrapper ${draggingCardId === card.id ? 'kanban-card-wrapper--dragging' : ''}`}
              draggable
              onDragStart={(e) => {
                e.dataTransfer.setData('cardId', card.id);
                e.dataTransfer.setData('sourceColumnId', column.id);
                e.dataTransfer.effectAllowed = 'move';
                onDragStart?.(card.id);
              }}
              onDragEnd={() => {
                onDragEnd?.();
              }}
              onDragOver={(e) => handleCardDragOver(e, index)}
            >
              <KanbanCard
                card={card}
                onEdit={onCardEdit}
                onDelete={onCardDelete ? (id) => onCardDelete(id, column.id) : undefined}
                onAcceptanceCriterionToggle={onAcceptanceCriterionToggle}
                isDragging={draggingCardId === card.id}
              />
            </div>
          </div>
        ))}
        {dropTargetIndex === column.cards.length && (
          <div className="kanban-column__drop-indicator" />
        )}
      </div>

      {isAddingCard ? (
        <div className="kanban-column__add-form">
          <input
            type="text"
            placeholder="Enter card title..."
            value={newCardTitle}
            onChange={(e) => setNewCardTitle(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') handleAddCard();
              if (e.key === 'Escape') {
                setNewCardTitle('');
                setIsAddingCard(false);
              }
            }}
            maxLength={MAX_TITLE_LENGTH}
            autoFocus
          />
          <div className="kanban-column__add-actions">
            <button onClick={handleAddCard} disabled={!newCardTitle.trim()}>
              Add
            </button>
            <button onClick={() => { setNewCardTitle(''); setIsAddingCard(false); }}>
              Cancel
            </button>
          </div>
        </div>
      ) : (
        <button
          className="kanban-column__add-button"
          onClick={() => setIsAddingCard(true)}
        >
          + Add Card
        </button>
      )}
    </div>
  );
}

export default KanbanColumn;

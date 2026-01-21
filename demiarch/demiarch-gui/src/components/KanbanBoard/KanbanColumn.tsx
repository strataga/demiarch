import { useState } from 'react';
import { KanbanCard } from './KanbanCard';
import type { KanbanColumn as KanbanColumnType, KanbanCard as KanbanCardType } from './types';

interface KanbanColumnProps {
  column: KanbanColumnType;
  onCardMove?: (cardId: string, sourceColumnId: string, targetColumnId: string) => void;
  onCardEdit?: (card: KanbanCardType) => void;
  onCardDelete?: (cardId: string, columnId: string) => void;
  onAddCard?: (columnId: string, title: string) => void;
  onEditColumn?: (columnId: string, title: string) => void;
  onDeleteColumn?: (columnId: string) => void;
}

export function KanbanColumn({
  column,
  onCardMove,
  onCardEdit,
  onCardDelete,
  onAddCard,
  onEditColumn,
  onDeleteColumn,
}: KanbanColumnProps) {
  const [isAddingCard, setIsAddingCard] = useState(false);
  const [newCardTitle, setNewCardTitle] = useState('');
  const [isDragOver, setIsDragOver] = useState(false);
  const [isEditingTitle, setIsEditingTitle] = useState(false);
  const [editTitle, setEditTitle] = useState(column.title);

  const isOverLimit = column.limit !== undefined && column.cards.length >= column.limit;

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
    setIsDragOver(true);
  };

  const handleDragLeave = () => {
    setIsDragOver(false);
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragOver(false);
    const cardId = e.dataTransfer.getData('cardId');
    const sourceColumnId = e.dataTransfer.getData('sourceColumnId');
    if (cardId && sourceColumnId && onCardMove) {
      onCardMove(cardId, sourceColumnId, column.id);
    }
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

      <div className="kanban-column__cards">
        {column.cards.map((card) => (
          <div
            key={card.id}
            draggable
            onDragStart={(e) => {
              e.dataTransfer.setData('cardId', card.id);
              e.dataTransfer.setData('sourceColumnId', column.id);
            }}
          >
            <KanbanCard
              card={card}
              onEdit={onCardEdit}
              onDelete={onCardDelete ? (id) => onCardDelete(id, column.id) : undefined}
            />
          </div>
        ))}
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

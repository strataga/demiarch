import { useState } from 'react';
import { KanbanColumn } from './KanbanColumn';
import type {
  KanbanBoard as KanbanBoardType,
  KanbanColumn as KanbanColumnType,
  KanbanCard as KanbanCardType,
} from './types';
import { DEFAULT_COLUMNS, createCard, createColumn } from './types';
import './KanbanBoard.css';

interface KanbanBoardProps {
  initialBoard?: KanbanBoardType;
  onBoardChange?: (board: KanbanBoardType) => void;
}

// Create default board with sample data
function createDefaultBoard(): KanbanBoardType {
  const now = new Date();
  const columns: KanbanColumnType[] = DEFAULT_COLUMNS.map((col) => ({
    ...col,
    cards: [],
  }));

  // Add sample cards for demonstration
  columns[0].cards = [
    { ...createCard('Set up project structure', 'Initialize the project with proper folder structure'), priority: 'medium' },
    { ...createCard('Design database schema', 'Create ERD and define tables'), priority: 'high' },
  ];
  columns[1].cards = [
    { ...createCard('Implement authentication', 'Add login/logout functionality'), priority: 'high', labels: ['security'] },
  ];
  columns[2].cards = [
    { ...createCard('Build Kanban UI', 'Create drag-and-drop kanban board'), priority: 'medium', labels: ['ui'] },
  ];

  return {
    id: 'default-board',
    title: 'Project Board',
    columns,
    createdAt: now,
    updatedAt: now,
  };
}

export function KanbanBoard({ initialBoard, onBoardChange }: KanbanBoardProps) {
  const [board, setBoard] = useState<KanbanBoardType>(
    initialBoard ?? createDefaultBoard()
  );
  const [isAddingColumn, setIsAddingColumn] = useState(false);
  const [newColumnTitle, setNewColumnTitle] = useState('');

  const updateBoard = (newBoard: KanbanBoardType) => {
    const updated = { ...newBoard, updatedAt: new Date() };
    setBoard(updated);
    onBoardChange?.(updated);
  };

  const handleCardMove = (cardId: string, sourceColumnId: string, targetColumnId: string) => {
    if (sourceColumnId === targetColumnId) return;

    const newColumns = board.columns.map((col) => {
      if (col.id === sourceColumnId) {
        return { ...col, cards: col.cards.filter((c) => c.id !== cardId) };
      }
      if (col.id === targetColumnId) {
        const sourceColumn = board.columns.find((c) => c.id === sourceColumnId);
        const card = sourceColumn?.cards.find((c) => c.id === cardId);
        if (card) {
          return { ...col, cards: [...col.cards, { ...card, updatedAt: new Date() }] };
        }
      }
      return col;
    });

    updateBoard({ ...board, columns: newColumns });
  };

  const handleAddCard = (columnId: string, title: string) => {
    const newColumns = board.columns.map((col) => {
      if (col.id === columnId) {
        return { ...col, cards: [...col.cards, createCard(title)] };
      }
      return col;
    });

    updateBoard({ ...board, columns: newColumns });
  };

  const handleCardDelete = (cardId: string, columnId: string) => {
    const newColumns = board.columns.map((col) => {
      if (col.id === columnId) {
        return { ...col, cards: col.cards.filter((c) => c.id !== cardId) };
      }
      return col;
    });

    updateBoard({ ...board, columns: newColumns });
  };

  const handleCardEdit = (card: KanbanCardType) => {
    // For now, just log - can be extended to open a modal
    console.log('Edit card:', card);
  };

  const handleEditColumn = (columnId: string, title: string) => {
    const newColumns = board.columns.map((col) => {
      if (col.id === columnId) {
        return { ...col, title };
      }
      return col;
    });

    updateBoard({ ...board, columns: newColumns });
  };

  const handleDeleteColumn = (columnId: string) => {
    const column = board.columns.find((c) => c.id === columnId);
    if (column && column.cards.length > 0) {
      if (!confirm(`Delete "${column.title}" column with ${column.cards.length} cards?`)) {
        return;
      }
    }

    const newColumns = board.columns.filter((col) => col.id !== columnId);
    updateBoard({ ...board, columns: newColumns });
  };

  const handleAddColumn = () => {
    if (!newColumnTitle.trim()) return;

    const colors = ['#ef4444', '#f97316', '#eab308', '#22c55e', '#06b6d4', '#3b82f6', '#8b5cf6', '#ec4899'];
    const randomColor = colors[Math.floor(Math.random() * colors.length)];

    const newColumn = createColumn(newColumnTitle.trim(), randomColor);
    updateBoard({ ...board, columns: [...board.columns, newColumn] });
    setNewColumnTitle('');
    setIsAddingColumn(false);
  };

  return (
    <div className="kanban-board">
      <div className="kanban-board__header">
        <h2 className="kanban-board__title">{board.title}</h2>
        <div className="kanban-board__stats">
          {board.columns.reduce((sum, col) => sum + col.cards.length, 0)} cards across {board.columns.length} columns
        </div>
      </div>

      <div className="kanban-board__columns">
        {board.columns.map((column) => (
          <KanbanColumn
            key={column.id}
            column={column}
            onCardMove={handleCardMove}
            onCardEdit={handleCardEdit}
            onCardDelete={handleCardDelete}
            onAddCard={handleAddCard}
            onEditColumn={handleEditColumn}
            onDeleteColumn={handleDeleteColumn}
          />
        ))}

        {isAddingColumn ? (
          <div className="kanban-board__add-column-form">
            <input
              type="text"
              placeholder="Column title..."
              value={newColumnTitle}
              onChange={(e) => setNewColumnTitle(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleAddColumn();
                if (e.key === 'Escape') {
                  setNewColumnTitle('');
                  setIsAddingColumn(false);
                }
              }}
              autoFocus
            />
            <div className="kanban-board__add-column-actions">
              <button onClick={handleAddColumn} disabled={!newColumnTitle.trim()}>
                Add Column
              </button>
              <button onClick={() => { setNewColumnTitle(''); setIsAddingColumn(false); }}>
                Cancel
              </button>
            </div>
          </div>
        ) : (
          <button
            className="kanban-board__add-column"
            onClick={() => setIsAddingColumn(true)}
          >
            + Add Column
          </button>
        )}
      </div>
    </div>
  );
}

export default KanbanBoard;

import { useState } from 'react';
import { KanbanColumn } from './KanbanColumn';
import type {
  KanbanBoard as KanbanBoardType,
  KanbanColumn as KanbanColumnType,
  KanbanCard as KanbanCardType,
} from './types';
import { DEFAULT_COLUMNS, createCard, createColumn, MAX_TITLE_LENGTH, MAX_COLUMNS } from './types';
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
    {
      ...createCard('Set up project structure', 'Initialize the project with proper folder structure'),
      priority: 'medium',
      acceptanceCriteria: [
        { id: 'ac-1', text: 'Create src directory with proper subdirectories', completed: true },
        { id: 'ac-2', text: 'Set up build configuration', completed: false },
        { id: 'ac-3', text: 'Add linting and formatting rules', completed: false },
      ],
    },
    {
      ...createCard('Design database schema', 'Create ERD and define tables'),
      priority: 'high',
      acceptanceCriteria: [
        { id: 'ac-4', text: 'Define all entity relationships', completed: false },
        { id: 'ac-5', text: 'Document table structures', completed: false },
      ],
    },
  ];
  columns[1].cards = [
    {
      ...createCard('Implement authentication', 'Add login/logout functionality'),
      priority: 'high',
      labels: ['security'],
      acceptanceCriteria: [
        { id: 'ac-6', text: 'User can register with email and password', completed: true },
        { id: 'ac-7', text: 'User can log in with credentials', completed: true },
        { id: 'ac-8', text: 'User can log out', completed: false },
        { id: 'ac-9', text: 'Session is persisted across page reloads', completed: false },
      ],
    },
  ];
  columns[2].cards = [
    {
      ...createCard('Build Kanban UI', 'Create drag-and-drop kanban board'),
      priority: 'medium',
      labels: ['ui'],
      acceptanceCriteria: [
        { id: 'ac-10', text: 'Display columns with cards', completed: true },
        { id: 'ac-11', text: 'Drag and drop cards between columns', completed: true },
        { id: 'ac-12', text: 'Add expandable card details', completed: false },
      ],
    },
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
  const [draggingCardId, setDraggingCardId] = useState<string | null>(null);

  const updateBoard = (newBoard: KanbanBoardType) => {
    const updated = { ...newBoard, updatedAt: new Date() };
    setBoard(updated);
    onBoardChange?.(updated);
  };

  const handleCardMove = (cardId: string, sourceColumnId: string, targetColumnId: string, targetIndex?: number) => {
    // Find the card being moved
    const sourceColumn = board.columns.find((c) => c.id === sourceColumnId);
    const card = sourceColumn?.cards.find((c) => c.id === cardId);
    if (!card) return;

    // Handle reordering within the same column
    if (sourceColumnId === targetColumnId) {
      const columnIndex = board.columns.findIndex((c) => c.id === sourceColumnId);
      if (columnIndex === -1) return;

      const column = board.columns[columnIndex];
      const currentIndex = column.cards.findIndex((c) => c.id === cardId);
      if (currentIndex === -1 || targetIndex === undefined) return;

      // Adjust target index if moving down within the same column
      let adjustedIndex = targetIndex;
      if (currentIndex < targetIndex) {
        adjustedIndex = targetIndex - 1;
      }

      // Don't move if it's already in the right position
      if (currentIndex === adjustedIndex) return;

      const newCards = [...column.cards];
      newCards.splice(currentIndex, 1);
      newCards.splice(adjustedIndex, 0, { ...card, updatedAt: new Date() });

      const newColumns = board.columns.map((col, idx) =>
        idx === columnIndex ? { ...col, cards: newCards } : col
      );

      updateBoard({ ...board, columns: newColumns });
      return;
    }

    // Handle moving between columns
    const newColumns = board.columns.map((col) => {
      if (col.id === sourceColumnId) {
        return { ...col, cards: col.cards.filter((c) => c.id !== cardId) };
      }
      if (col.id === targetColumnId) {
        const updatedCard = { ...card, updatedAt: new Date() };
        if (targetIndex !== undefined) {
          const newCards = [...col.cards];
          newCards.splice(targetIndex, 0, updatedCard);
          return { ...col, cards: newCards };
        }
        return { ...col, cards: [...col.cards, updatedCard] };
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

  const handleCardEdit = (_card: KanbanCardType) => {
    // TODO: Implement card edit modal
  };

  const handleAcceptanceCriterionToggle = (cardId: string, criterionId: string) => {
    const newColumns = board.columns.map((col) => ({
      ...col,
      cards: col.cards.map((card) => {
        if (card.id !== cardId || !card.acceptanceCriteria) return card;
        return {
          ...card,
          updatedAt: new Date(),
          acceptanceCriteria: card.acceptanceCriteria.map((criterion) =>
            criterion.id === criterionId
              ? { ...criterion, completed: !criterion.completed }
              : criterion
          ),
        };
      }),
    }));

    updateBoard({ ...board, columns: newColumns });
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
      // Truncate title for display in confirm dialog to prevent misleading messages
      const displayTitle = column.title.length > 50 ? `${column.title.slice(0, 50)}...` : column.title;
      if (!confirm(`Delete "${displayTitle}" column with ${column.cards.length} cards?`)) {
        return;
      }
    }

    const newColumns = board.columns.filter((col) => col.id !== columnId);
    updateBoard({ ...board, columns: newColumns });
  };

  const handleAddColumn = () => {
    if (!newColumnTitle.trim()) return;
    // Enforce max columns limit
    if (board.columns.length >= MAX_COLUMNS) return;

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
            onAcceptanceCriterionToggle={handleAcceptanceCriterionToggle}
            draggingCardId={draggingCardId}
            onDragStart={setDraggingCardId}
            onDragEnd={() => setDraggingCardId(null)}
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
              maxLength={MAX_TITLE_LENGTH}
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

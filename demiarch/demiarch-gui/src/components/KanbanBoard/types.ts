// Kanban Board Types

export interface KanbanCard {
  id: string;
  title: string;
  description?: string;
  priority?: 'low' | 'medium' | 'high';
  labels?: string[];
  createdAt: Date;
  updatedAt: Date;
}

export interface KanbanColumn {
  id: string;
  title: string;
  cards: KanbanCard[];
  color?: string;
  limit?: number; // WIP limit
}

export interface KanbanBoard {
  id: string;
  title: string;
  columns: KanbanColumn[];
  createdAt: Date;
  updatedAt: Date;
}

// Default columns for a new board
export const DEFAULT_COLUMNS: Omit<KanbanColumn, 'cards'>[] = [
  { id: 'backlog', title: 'Backlog', color: '#6b7280' },
  { id: 'todo', title: 'To Do', color: '#3b82f6' },
  { id: 'in-progress', title: 'In Progress', color: '#f59e0b', limit: 3 },
  { id: 'review', title: 'Review', color: '#8b5cf6', limit: 2 },
  { id: 'done', title: 'Done', color: '#22c55e' },
];

// Helper to create a new card
export function createCard(title: string, description?: string): KanbanCard {
  const now = new Date();
  return {
    id: `card-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
    title,
    description,
    createdAt: now,
    updatedAt: now,
  };
}

// Helper to create a new column
export function createColumn(title: string, color?: string): KanbanColumn {
  return {
    id: `col-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
    title,
    cards: [],
    color,
  };
}

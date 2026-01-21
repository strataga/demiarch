// Kanban Board Types

// Validation constants
export const MAX_TITLE_LENGTH = 100;
export const MAX_DESCRIPTION_LENGTH = 1000;
export const MAX_COLUMNS = 20;
export const MAX_CARDS_PER_COLUMN = 100;

// Hex color validation
export function isValidHexColor(color: string): boolean {
  return /^#([0-9A-Fa-f]{3}){1,2}$/.test(color);
}

export interface AcceptanceCriterion {
  id: string;
  text: string;
  completed: boolean;
}

export interface KanbanCard {
  id: string;
  title: string;
  description?: string;
  priority?: 'low' | 'medium' | 'high';
  labels?: string[];
  acceptanceCriteria?: AcceptanceCriterion[];
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
    title: title.slice(0, MAX_TITLE_LENGTH),
    description: description?.slice(0, MAX_DESCRIPTION_LENGTH),
    createdAt: now,
    updatedAt: now,
  };
}

// Helper to create a new column
export function createColumn(title: string, color?: string): KanbanColumn {
  // Validate and sanitize color
  const safeColor = color && isValidHexColor(color) ? color : undefined;

  return {
    id: `col-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
    title: title.slice(0, MAX_TITLE_LENGTH),
    cards: [],
    color: safeColor,
  };
}

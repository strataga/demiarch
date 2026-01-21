// Project Types

import type { KanbanBoard } from '../components/KanbanBoard/types';

export interface Project {
  id: string;
  name: string;
  description?: string;
  color: string;
  board: KanbanBoard;
  createdAt: Date;
  updatedAt: Date;
}

// Project colors for visual distinction
export const PROJECT_COLORS = [
  '#ef4444', // red
  '#f97316', // orange
  '#eab308', // yellow
  '#22c55e', // green
  '#06b6d4', // cyan
  '#3b82f6', // blue
  '#8b5cf6', // violet
  '#ec4899', // pink
] as const;

// Helper to create a new project
export function createProject(
  name: string,
  board: KanbanBoard,
  description?: string,
  color?: string
): Project {
  const now = new Date();
  return {
    id: `project-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
    name,
    description,
    color: color ?? PROJECT_COLORS[Math.floor(Math.random() * PROJECT_COLORS.length)],
    board,
    createdAt: now,
    updatedAt: now,
  };
}

import type { Project } from '../types/project';
import type { KanbanBoard, KanbanColumn } from '../components/KanbanBoard/types';
import { DEFAULT_COLUMNS, createCard } from '../components/KanbanBoard/types';
import { PROJECT_COLORS } from '../types/project';

function createBoard(id: string, title: string, columns: KanbanColumn[]): KanbanBoard {
  const now = new Date();
  return {
    id,
    title,
    columns,
    createdAt: now,
    updatedAt: now,
  };
}

function createProjectWithBoard(
  id: string,
  name: string,
  description: string,
  color: string,
  columns: KanbanColumn[]
): Project {
  const now = new Date();
  return {
    id,
    name,
    description,
    color,
    board: createBoard(`board-${id}`, `${name} Board`, columns),
    createdAt: now,
    updatedAt: now,
  };
}

// Demiarch project - the main project
const demiarchColumns: KanbanColumn[] = DEFAULT_COLUMNS.map((col) => ({
  ...col,
  cards: [],
}));

demiarchColumns[0].cards = [
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

demiarchColumns[1].cards = [
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

demiarchColumns[2].cards = [
  {
    ...createCard('Build Kanban UI', 'Create drag-and-drop kanban board'),
    priority: 'medium',
    labels: ['ui'],
    acceptanceCriteria: [
      { id: 'ac-10', text: 'Display columns with cards', completed: true },
      { id: 'ac-11', text: 'Drag and drop cards between columns', completed: true },
      { id: 'ac-12', text: 'Add expandable card details', completed: true },
    ],
  },
];

demiarchColumns[3].cards = [
  {
    ...createCard('Project switching UI', 'Implement project selector dropdown'),
    priority: 'medium',
    labels: ['ui'],
    acceptanceCriteria: [
      { id: 'ac-13', text: 'Show project dropdown selector', completed: false },
      { id: 'ac-14', text: 'Switch between projects', completed: false },
      { id: 'ac-15', text: 'Display project info in header', completed: false },
    ],
  },
];

// Mobile App project
const mobileColumns: KanbanColumn[] = DEFAULT_COLUMNS.map((col) => ({
  ...col,
  cards: [],
}));

mobileColumns[0].cards = [
  {
    ...createCard('Set up React Native', 'Initialize RN project with Expo'),
    priority: 'high',
    labels: ['setup'],
  },
  {
    ...createCard('Design system', 'Create reusable components'),
    priority: 'medium',
  },
];

mobileColumns[1].cards = [
  {
    ...createCard('Home screen UI', 'Build main navigation'),
    priority: 'high',
    labels: ['ui'],
  },
];

mobileColumns[2].cards = [
  {
    ...createCard('Push notifications', 'Integrate Firebase Cloud Messaging'),
    priority: 'medium',
    labels: ['backend'],
  },
];

// Website Redesign project
const websiteColumns: KanbanColumn[] = DEFAULT_COLUMNS.map((col) => ({
  ...col,
  cards: [],
}));

websiteColumns[0].cards = [
  {
    ...createCard('Gather requirements', 'Meet with stakeholders'),
    priority: 'high',
  },
  {
    ...createCard('Competitor analysis', 'Review similar websites'),
    priority: 'medium',
  },
  {
    ...createCard('Content audit', 'Review existing content'),
    priority: 'low',
  },
];

websiteColumns[1].cards = [
  {
    ...createCard('Create wireframes', 'Low-fidelity mockups'),
    priority: 'high',
    labels: ['design'],
  },
];

websiteColumns[4].cards = [
  {
    ...createCard('Update logo', 'New brand colors applied'),
    priority: 'low',
    labels: ['branding'],
  },
];

export const SAMPLE_PROJECTS: Project[] = [
  createProjectWithBoard(
    'project-demiarch',
    'Demiarch',
    'AI-powered development assistant',
    PROJECT_COLORS[5], // blue
    demiarchColumns
  ),
  createProjectWithBoard(
    'project-mobile',
    'Mobile App',
    'Cross-platform mobile application',
    PROJECT_COLORS[4], // cyan
    mobileColumns
  ),
  createProjectWithBoard(
    'project-website',
    'Website Redesign',
    'Company website refresh',
    PROJECT_COLORS[6], // violet
    websiteColumns
  ),
];

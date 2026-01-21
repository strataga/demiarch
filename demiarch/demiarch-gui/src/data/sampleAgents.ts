import type { Agent } from '../components/AgentStatus/types';

// Sample agents for demonstrating the Agent Status visualization
export const SAMPLE_AGENTS: Agent[] = [
  {
    id: 'agent-orchestrator-1',
    name: 'Orchestrator',
    type: 'orchestrator',
    status: 'working',
    currentTask: 'Coordinating task distribution',
    progress: 65,
    tasks: [
      {
        id: 'task-1',
        description: 'Initialize agent pool',
        startedAt: new Date(Date.now() - 300000),
        completedAt: new Date(Date.now() - 240000),
      },
      {
        id: 'task-2',
        description: 'Analyze project requirements',
        startedAt: new Date(Date.now() - 240000),
        completedAt: new Date(Date.now() - 120000),
      },
      {
        id: 'task-3',
        description: 'Coordinating task distribution',
        startedAt: new Date(Date.now() - 120000),
      },
    ],
    createdAt: new Date(Date.now() - 300000),
    updatedAt: new Date(),
  },
  {
    id: 'agent-coder-1',
    name: 'Coder Alpha',
    type: 'coder',
    status: 'working',
    currentTask: 'Implementing user authentication module',
    progress: 45,
    tasks: [
      {
        id: 'task-4',
        description: 'Set up project structure',
        startedAt: new Date(Date.now() - 180000),
        completedAt: new Date(Date.now() - 90000),
      },
      {
        id: 'task-5',
        description: 'Implementing user authentication module',
        startedAt: new Date(Date.now() - 90000),
      },
    ],
    parentId: 'agent-orchestrator-1',
    createdAt: new Date(Date.now() - 180000),
    updatedAt: new Date(),
  },
  {
    id: 'agent-reviewer-1',
    name: 'Code Reviewer',
    type: 'reviewer',
    status: 'waiting',
    currentTask: 'Waiting for code submission',
    tasks: [],
    parentId: 'agent-orchestrator-1',
    createdAt: new Date(Date.now() - 150000),
    updatedAt: new Date(),
  },
  {
    id: 'agent-researcher-1',
    name: 'Research Assistant',
    type: 'researcher',
    status: 'thinking',
    currentTask: 'Analyzing best practices for state management',
    tasks: [
      {
        id: 'task-6',
        description: 'Researching React patterns',
        startedAt: new Date(Date.now() - 60000),
      },
    ],
    parentId: 'agent-orchestrator-1',
    createdAt: new Date(Date.now() - 60000),
    updatedAt: new Date(),
  },
  {
    id: 'agent-planner-1',
    name: 'Sprint Planner',
    type: 'planner',
    status: 'idle',
    tasks: [
      {
        id: 'task-7',
        description: 'Created sprint backlog',
        startedAt: new Date(Date.now() - 600000),
        completedAt: new Date(Date.now() - 500000),
      },
    ],
    createdAt: new Date(Date.now() - 600000),
    updatedAt: new Date(Date.now() - 500000),
  },
];

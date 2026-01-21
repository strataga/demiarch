// Agent Status Types

export type AgentStatus = 'idle' | 'thinking' | 'working' | 'waiting' | 'error' | 'completed';

export type AgentType = 'orchestrator' | 'coder' | 'reviewer' | 'researcher' | 'planner';

export interface AgentTask {
  id: string;
  description: string;
  startedAt: Date;
  completedAt?: Date;
}

export interface Agent {
  id: string;
  name: string;
  type: AgentType;
  status: AgentStatus;
  currentTask?: string;
  progress?: number; // 0-100 for tasks with measurable progress
  tasks: AgentTask[];
  parentId?: string; // For hierarchy support
  error?: string;
  createdAt: Date;
  updatedAt: Date;
}

export interface AgentActivity {
  id: string;
  agentId: string;
  message: string;
  timestamp: Date;
  type: 'info' | 'success' | 'warning' | 'error';
}

// Status display configuration
export const STATUS_CONFIG: Record<AgentStatus, { label: string; color: string; icon: string }> = {
  idle: { label: 'Idle', color: '#6b7280', icon: '○' },
  thinking: { label: 'Thinking', color: '#f59e0b', icon: '◐' },
  working: { label: 'Working', color: '#3b82f6', icon: '●' },
  waiting: { label: 'Waiting', color: '#8b5cf6', icon: '◑' },
  error: { label: 'Error', color: '#ef4444', icon: '✕' },
  completed: { label: 'Completed', color: '#22c55e', icon: '✓' },
};

// Agent type display configuration
export const AGENT_TYPE_CONFIG: Record<AgentType, { label: string; color: string }> = {
  orchestrator: { label: 'Orchestrator', color: '#f59e0b' },
  coder: { label: 'Coder', color: '#3b82f6' },
  reviewer: { label: 'Reviewer', color: '#8b5cf6' },
  researcher: { label: 'Researcher', color: '#22c55e' },
  planner: { label: 'Planner', color: '#ec4899' },
};

// Helper to create a new agent
export function createAgent(
  name: string,
  type: AgentType,
  parentId?: string
): Agent {
  const now = new Date();
  return {
    id: `agent-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
    name,
    type,
    status: 'idle',
    tasks: [],
    parentId,
    createdAt: now,
    updatedAt: now,
  };
}

// Helper to create an agent task
export function createAgentTask(description: string): AgentTask {
  return {
    id: `task-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
    description,
    startedAt: new Date(),
  };
}

// Helper to create an activity log entry
export function createActivity(
  agentId: string,
  message: string,
  type: AgentActivity['type'] = 'info'
): AgentActivity {
  return {
    id: `activity-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
    agentId,
    message,
    timestamp: new Date(),
    type,
  };
}

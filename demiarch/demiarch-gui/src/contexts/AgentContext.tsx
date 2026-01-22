import { createContext, useContext, useState, useCallback, useEffect, type ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { Agent, AgentActivity, AgentStatus, AgentType } from '../components/AgentStatus/types';
import { createActivity } from '../components/AgentStatus/types';

// Backend event types (mirrors Tauri/Rust types)
interface BackendAgentEvent {
  timestamp: string;
  event_id: string;
  session_id: string;
  event_type: 'spawned' | 'started' | 'status_update' | 'completed' | 'failed' | 'cancelled' | 'token_update' | 'disposed';
  agent: {
    id: string;
    agent_type: string;
    name: string;
    parent_id: string | null;
    path: string;
    status: string;
    tokens: number;
    task: string | null;
    error: string | null;
  };
}

interface AgentContextValue {
  agents: Agent[];
  activities: AgentActivity[];
  addAgent: (name: string, type: AgentType, parentId?: string) => Agent;
  updateAgentStatus: (agentId: string, status: AgentStatus, currentTask?: string) => void;
  updateAgentProgress: (agentId: string, progress: number) => void;
  setAgentError: (agentId: string, error: string) => void;
  clearAgentError: (agentId: string) => void;
  removeAgent: (agentId: string) => void;
  logActivity: (agentId: string, message: string, type?: AgentActivity['type']) => void;
  clearActivities: () => void;
  clearAgents: () => void;
  isConnected: boolean;
}

const AgentContext = createContext<AgentContextValue | null>(null);

interface AgentProviderProps {
  children: ReactNode;
  initialAgents?: Agent[];
}

// Map backend agent type to frontend type
function mapAgentType(backendType: string): AgentType {
  const typeMap: Record<string, AgentType> = {
    'orchestrator': 'orchestrator',
    'planner': 'planner',
    'coder': 'coder',
    'reviewer': 'reviewer',
    'tester': 'tester',
    'researcher': 'researcher',
  };
  return typeMap[backendType.toLowerCase()] || 'coder';
}

// Map backend status to frontend status
function mapAgentStatus(backendStatus: string): AgentStatus {
  const statusMap: Record<string, AgentStatus> = {
    'spawned': 'idle',
    'started': 'thinking',
    'running': 'working',
    'working': 'working',
    'thinking': 'thinking',
    'waiting': 'waiting',
    'completed': 'completed',
    'failed': 'error',
    'cancelled': 'error',
    'disposed': 'completed',
  };
  return statusMap[backendStatus.toLowerCase()] || 'idle';
}

export function AgentProvider({ children, initialAgents = [] }: AgentProviderProps) {
  const [agents, setAgents] = useState<Agent[]>(initialAgents);
  const [activities, setActivities] = useState<AgentActivity[]>([]);
  const [isConnected, setIsConnected] = useState(false);

  // Initialize Tauri event listeners
  useEffect(() => {
    let unlistenEvent: UnlistenFn | null = null;
    let unlistenSession: UnlistenFn | null = null;

    async function initializeEventListener() {
      try {
        // Start the agent watcher on the backend
        await invoke('start_agent_watcher');
        setIsConnected(true);

        // Load recent events first
        try {
          const recentEvents = await invoke<BackendAgentEvent[]>('get_recent_agent_events', { count: 100 });
          processInitialEvents(recentEvents);
        } catch (e) {
          console.warn('No recent events to load:', e);
        }

        // Listen for new agent events
        unlistenEvent = await listen<BackendAgentEvent>('agent-event', (event) => {
          processAgentEvent(event.payload);
        });

        // Listen for session changes (new demiarch run started)
        unlistenSession = await listen<string>('agent-session-change', () => {
          // Clear agents and activities when a new session starts
          setAgents([]);
          setActivities([]);
        });

      } catch (error) {
        console.error('Failed to initialize agent event listener:', error);
        setIsConnected(false);
      }
    }

    function processInitialEvents(events: BackendAgentEvent[]) {
      const agentMap = new Map<string, Agent>();
      const activityList: AgentActivity[] = [];

      for (const event of events) {
        const agentId = event.agent.id;

        if (event.event_type === 'spawned') {
          // Create new agent
          agentMap.set(agentId, {
            id: agentId,
            name: event.agent.name,
            type: mapAgentType(event.agent.agent_type),
            status: 'idle',
            currentTask: event.agent.task || undefined,
            tasks: [],
            parentId: event.agent.parent_id || undefined,
            createdAt: new Date(event.timestamp),
            updatedAt: new Date(event.timestamp),
          });

          activityList.push(createActivity(agentId, `Agent "${event.agent.name}" spawned`, 'info'));
        } else {
          // Update existing agent
          const existing = agentMap.get(agentId);
          if (existing) {
            existing.status = mapAgentStatus(event.agent.status || event.event_type);
            existing.updatedAt = new Date(event.timestamp);

            if (event.event_type === 'completed') {
              existing.status = 'completed';
              activityList.push(createActivity(agentId, `Agent "${existing.name}" completed`, 'success'));
            } else if (event.event_type === 'failed') {
              existing.status = 'error';
              existing.error = event.agent.error || 'Unknown error';
              activityList.push(createActivity(agentId, `Agent "${existing.name}" failed: ${event.agent.error}`, 'error'));
            } else if (event.event_type === 'started') {
              existing.status = 'working';
              activityList.push(createActivity(agentId, `Agent "${existing.name}" started working`, 'info'));
            }
          }
        }
      }

      setAgents(Array.from(agentMap.values()));
      setActivities(activityList.slice(-50)); // Keep last 50 activities
    }

    function processAgentEvent(event: BackendAgentEvent) {
      const agentId = event.agent.id;

      switch (event.event_type) {
        case 'spawned': {
          const newAgent: Agent = {
            id: agentId,
            name: event.agent.name,
            type: mapAgentType(event.agent.agent_type),
            status: 'idle',
            currentTask: event.agent.task || undefined,
            tasks: [],
            parentId: event.agent.parent_id || undefined,
            createdAt: new Date(event.timestamp),
            updatedAt: new Date(event.timestamp),
          };
          setAgents((prev) => {
            // Don't add duplicate agents
            if (prev.find((a) => a.id === agentId)) {
              return prev;
            }
            return [...prev, newAgent];
          });
          setActivities((prev) => [...prev.slice(-49), createActivity(agentId, `Agent "${event.agent.name}" spawned`, 'info')]);
          break;
        }

        case 'started': {
          setAgents((prev) =>
            prev.map((agent) =>
              agent.id === agentId
                ? { ...agent, status: 'working' as AgentStatus, updatedAt: new Date(event.timestamp) }
                : agent
            )
          );
          setActivities((prev) => {
            const agent = agents.find((a) => a.id === agentId);
            return [...prev.slice(-49), createActivity(agentId, `Agent "${agent?.name || agentId}" started working`, 'info')];
          });
          break;
        }

        case 'status_update': {
          setAgents((prev) =>
            prev.map((agent) =>
              agent.id === agentId
                ? {
                    ...agent,
                    status: mapAgentStatus(event.agent.status),
                    currentTask: event.agent.task || agent.currentTask,
                    updatedAt: new Date(event.timestamp),
                  }
                : agent
            )
          );
          break;
        }

        case 'completed': {
          setAgents((prev) =>
            prev.map((agent) =>
              agent.id === agentId
                ? { ...agent, status: 'completed' as AgentStatus, updatedAt: new Date(event.timestamp) }
                : agent
            )
          );
          setActivities((prev) => {
            const agent = agents.find((a) => a.id === agentId);
            return [...prev.slice(-49), createActivity(agentId, `Agent "${agent?.name || agentId}" completed`, 'success')];
          });
          break;
        }

        case 'failed': {
          setAgents((prev) =>
            prev.map((agent) =>
              agent.id === agentId
                ? {
                    ...agent,
                    status: 'error' as AgentStatus,
                    error: event.agent.error || 'Unknown error',
                    updatedAt: new Date(event.timestamp),
                  }
                : agent
            )
          );
          setActivities((prev) => {
            const agent = agents.find((a) => a.id === agentId);
            return [...prev.slice(-49), createActivity(agentId, `Agent "${agent?.name || agentId}" failed: ${event.agent.error}`, 'error')];
          });
          break;
        }

        case 'cancelled': {
          setAgents((prev) =>
            prev.map((agent) =>
              agent.id === agentId
                ? { ...agent, status: 'error' as AgentStatus, error: 'Cancelled', updatedAt: new Date(event.timestamp) }
                : agent
            )
          );
          break;
        }

        case 'token_update': {
          // Could track token usage if needed
          break;
        }

        case 'disposed': {
          // Keep the agent visible but marked as completed
          setAgents((prev) =>
            prev.map((agent) =>
              agent.id === agentId
                ? { ...agent, status: 'completed' as AgentStatus, updatedAt: new Date(event.timestamp) }
                : agent
            )
          );
          break;
        }
      }
    }

    initializeEventListener();

    return () => {
      if (unlistenEvent) unlistenEvent();
      if (unlistenSession) unlistenSession();
    };
  }, []);

  const addAgent = useCallback(
    (name: string, type: AgentType, parentId?: string): Agent => {
      const now = new Date();
      const newAgent: Agent = {
        id: `agent-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
        name,
        type,
        status: 'idle',
        tasks: [],
        parentId,
        createdAt: now,
        updatedAt: now,
      };
      setAgents((prev) => [...prev, newAgent]);

      const activity = createActivity(newAgent.id, `Agent "${name}" (${type}) initialized`, 'info');
      setActivities((prev) => [...prev, activity]);

      return newAgent;
    },
    []
  );

  const updateAgentStatus = useCallback(
    (agentId: string, status: AgentStatus, currentTask?: string) => {
      setAgents((prev) =>
        prev.map((agent) =>
          agent.id === agentId
            ? {
                ...agent,
                status,
                currentTask,
                error: status !== 'error' ? undefined : agent.error,
                updatedAt: new Date(),
              }
            : agent
        )
      );
    },
    []
  );

  const updateAgentProgress = useCallback((agentId: string, progress: number) => {
    setAgents((prev) =>
      prev.map((agent) =>
        agent.id === agentId
          ? {
              ...agent,
              progress: Math.min(100, Math.max(0, progress)),
              updatedAt: new Date(),
            }
          : agent
      )
    );
  }, []);

  const setAgentError = useCallback((agentId: string, error: string) => {
    setAgents((prev) =>
      prev.map((agent) =>
        agent.id === agentId
          ? {
              ...agent,
              status: 'error' as AgentStatus,
              error,
              updatedAt: new Date(),
            }
          : agent
      )
    );

    const activity = createActivity(agentId, error, 'error');
    setActivities((prev) => [...prev, activity]);
  }, []);

  const clearAgentError = useCallback((agentId: string) => {
    setAgents((prev) =>
      prev.map((agent) =>
        agent.id === agentId
          ? {
              ...agent,
              status: 'idle' as AgentStatus,
              error: undefined,
              updatedAt: new Date(),
            }
          : agent
      )
    );
  }, []);

  const removeAgent = useCallback((agentId: string) => {
    setAgents((prev) => prev.filter((agent) => agent.id !== agentId));
  }, []);

  const logActivity = useCallback(
    (agentId: string, message: string, type: AgentActivity['type'] = 'info') => {
      const activity = createActivity(agentId, message, type);
      setActivities((prev) => [...prev, activity]);
    },
    []
  );

  const clearActivities = useCallback(() => {
    setActivities([]);
  }, []);

  const clearAgents = useCallback(() => {
    setAgents([]);
    setActivities([]);
  }, []);

  return (
    <AgentContext.Provider
      value={{
        agents,
        activities,
        addAgent,
        updateAgentStatus,
        updateAgentProgress,
        setAgentError,
        clearAgentError,
        removeAgent,
        logActivity,
        clearActivities,
        clearAgents,
        isConnected,
      }}
    >
      {children}
    </AgentContext.Provider>
  );
}

export function useAgents() {
  const context = useContext(AgentContext);
  if (!context) {
    throw new Error('useAgents must be used within an AgentProvider');
  }
  return context;
}

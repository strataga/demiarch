import { createContext, useContext, useState, useCallback, type ReactNode } from 'react';
import type { Agent, AgentActivity, AgentStatus, AgentType } from '../components/AgentStatus/types';
import { createAgent as createAgentHelper, createActivity } from '../components/AgentStatus/types';

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
}

const AgentContext = createContext<AgentContextValue | null>(null);

interface AgentProviderProps {
  children: ReactNode;
  initialAgents?: Agent[];
}

export function AgentProvider({ children, initialAgents = [] }: AgentProviderProps) {
  const [agents, setAgents] = useState<Agent[]>(initialAgents);
  const [activities, setActivities] = useState<AgentActivity[]>([]);

  const addAgent = useCallback(
    (name: string, type: AgentType, parentId?: string): Agent => {
      const newAgent = createAgentHelper(name, type, parentId);
      setAgents((prev) => [...prev, newAgent]);

      // Log the agent creation
      const activity = createActivity(
        newAgent.id,
        `Agent "${name}" (${type}) initialized`,
        'info'
      );
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

    // Log the error
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

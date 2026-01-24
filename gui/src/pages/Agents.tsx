import { useEffect, useState } from 'react';
import { AgentStatus, invoke } from '../lib/api';
import { Bot, Cpu, Code2, Search, TestTube2, Package, Settings, FileCode, ChevronDown, ChevronRight, Trash2 } from 'lucide-react';

const AGENT_ICONS: Record<string, React.ComponentType<{ className?: string }>> = {
  orchestrator: Bot,
  planner: Cpu,
  coder: Code2,
  reviewer: Search,
  tester: TestTube2,
};

const AGENT_COLORS: Record<string, string> = {
  orchestrator: 'text-accent-teal bg-accent-teal/10',
  planner: 'text-accent-magenta bg-accent-magenta/10',
  coder: 'text-accent-amber bg-accent-amber/10',
  reviewer: 'text-accent-amber bg-accent-amber/10',
  tester: 'text-accent-amber bg-accent-amber/10',
};

const STATUS_COLORS: Record<string, string> = {
  running: 'text-accent-teal',
  pending: 'text-gray-400',
  success: 'text-green-400',
  failed: 'text-accent-magenta',
  cancelled: 'text-gray-500',
};

export default function Agents() {
  const [agents, setAgents] = useState<AgentStatus[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function loadAgents() {
      try {
        const data = await invoke<AgentStatus[]>('get_agents');
        setAgents(data);
      } catch (error) {
        console.error('Failed to load agents:', error);
      } finally {
        setLoading(false);
      }
    }
    loadAgents();

    // Poll for updates
    const interval = setInterval(loadAgents, 2000);
    return () => clearInterval(interval);
  }, []);

  const clearCompletedAgents = async () => {
    try {
      await invoke('clear_completed_agents');
      const data = await invoke<AgentStatus[]>('get_agents');
      setAgents(data);
    } catch (error) {
      console.error('Failed to clear agents:', error);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="animate-pulse text-accent-teal">Loading agents...</div>
      </div>
    );
  }

  // Build tree structure
  const rootAgents = agents.filter((a) => !a.parent_id);
  const childAgents = agents.filter((a) => a.parent_id);

  return (
    <div className="p-6 space-y-6">
      <div className="flex justify-between items-center">
        <h1 className="text-2xl font-bold">Agent Hierarchy</h1>
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-4 text-sm text-gray-400">
            <span>Total: {agents.length}</span>
            <span>Running: {agents.filter((a) => a.status === 'running').length}</span>
            <span>Tokens: {agents.reduce((sum, a) => sum + a.tokens_used, 0)}</span>
          </div>
          {agents.some((a) => a.status === 'success' || a.status === 'failed') && (
            <button
              onClick={clearCompletedAgents}
              className="flex items-center gap-1 px-3 py-1.5 text-sm bg-background-surface text-gray-300 rounded-lg hover:bg-background-surface/80 transition-colors"
            >
              <Trash2 className="w-4 h-4" />
              Clear Completed
            </button>
          )}
        </div>
      </div>

      {/* Agent Legend */}
      <div className="flex gap-4 text-sm">
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 rounded-full bg-accent-teal" />
          <span className="text-gray-400">Orchestrator (L1)</span>
        </div>
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 rounded-full bg-accent-magenta" />
          <span className="text-gray-400">Planner (L2)</span>
        </div>
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 rounded-full bg-accent-amber" />
          <span className="text-gray-400">Workers (L3)</span>
        </div>
      </div>

      {/* Agent Tree */}
      <div className="bg-background-mid rounded-lg border border-background-surface p-6">
        {rootAgents.length === 0 ? (
          <div className="text-center text-gray-400 py-8">
            <Bot className="w-12 h-12 mx-auto mb-3 opacity-50" />
            <p>No active agents</p>
            <p className="text-sm">Agents will appear here during code generation</p>
          </div>
        ) : (
          <div className="space-y-4">
            {rootAgents.map((agent) => (
              <AgentNode
                key={agent.id}
                agent={agent}
                children={childAgents.filter((c) => c.parent_id === agent.id)}
                allAgents={agents}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function AgentNode({
  agent,
  children,
  allAgents,
  depth = 0,
}: {
  agent: AgentStatus;
  children: AgentStatus[];
  allAgents: AgentStatus[];
  depth?: number;
}) {
  const [expanded, setExpanded] = useState(false);
  const Icon = AGENT_ICONS[agent.agent_type] || Bot;
  const colorClass = AGENT_COLORS[agent.agent_type] || AGENT_COLORS.orchestrator;
  const statusColor = STATUS_COLORS[agent.status] || STATUS_COLORS.pending;

  const hasDependencies = agent.dependencies && agent.dependencies.length > 0;
  const hasSetup = agent.setup_requirements && agent.setup_requirements.length > 0;
  const hasFiles = agent.generated_files && agent.generated_files.length > 0;
  const hasDetails = hasDependencies || hasSetup || hasFiles;

  return (
    <div className="space-y-3">
      <div
        className="rounded-lg bg-background-surface"
        style={{ marginLeft: depth * 24 }}
      >
        <div className="flex items-center gap-3 p-3">
          {hasDetails && (
            <button
              onClick={() => setExpanded(!expanded)}
              className="text-gray-400 hover:text-gray-200"
            >
              {expanded ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
            </button>
          )}
          <div className={`w-10 h-10 rounded-lg ${colorClass} flex items-center justify-center`}>
            <Icon className="w-5 h-5" />
          </div>
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2">
              <span className="font-medium capitalize">{agent.agent_type}</span>
              <span className={`text-xs ${statusColor}`}>● {agent.status}</span>
              {agent.feature_name && (
                <span className="text-xs text-gray-500">• {agent.feature_name}</span>
              )}
            </div>
            {agent.task && (
              <p className="text-sm text-gray-400 truncate">{agent.task}</p>
            )}
          </div>
          <div className="text-right flex items-center gap-4">
            {hasFiles && (
              <span className="text-xs text-gray-500 flex items-center gap-1">
                <FileCode className="w-3 h-3" />
                {agent.generated_files?.length}
              </span>
            )}
            {hasDependencies && (
              <span className="text-xs text-gray-500 flex items-center gap-1">
                <Package className="w-3 h-3" />
                {agent.dependencies?.length}
              </span>
            )}
            {hasSetup && (
              <span className="text-xs text-gray-500 flex items-center gap-1">
                <Settings className="w-3 h-3" />
                {agent.setup_requirements?.length}
              </span>
            )}
            <div>
              <span className="text-sm font-mono text-gray-400">{agent.tokens_used} tok</span>
              <p className="text-xs text-gray-500">{agent.id.slice(0, 8)}</p>
            </div>
          </div>
        </div>

        {/* Expanded details */}
        {expanded && hasDetails && (
          <div className="px-4 pb-3 space-y-3 border-t border-background-deep">
            {/* Generated Files */}
            {hasFiles && (
              <div className="pt-3">
                <h4 className="text-xs font-medium text-gray-400 mb-2 flex items-center gap-1">
                  <FileCode className="w-3 h-3" />
                  Generated Files ({agent.generated_files?.length})
                </h4>
                <div className="space-y-1">
                  {agent.generated_files?.map((file, i) => (
                    <div key={i} className="text-xs font-mono text-gray-500 bg-background-deep px-2 py-1 rounded">
                      {file}
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Dependencies */}
            {hasDependencies && (
              <div className="pt-3">
                <h4 className="text-xs font-medium text-gray-400 mb-2 flex items-center gap-1">
                  <Package className="w-3 h-3" />
                  Dependencies ({agent.dependencies?.length})
                </h4>
                <div className="space-y-1">
                  {agent.dependencies?.map((dep, i) => (
                    <div key={i} className="text-xs bg-background-deep px-2 py-1 rounded flex justify-between">
                      <span className="font-mono text-accent-teal">{dep.name}{dep.version && `@${dep.version}`}</span>
                      <span className="text-gray-500">{dep.reason}</span>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Setup Requirements */}
            {hasSetup && (
              <div className="pt-3">
                <h4 className="text-xs font-medium text-gray-400 mb-2 flex items-center gap-1">
                  <Settings className="w-3 h-3" />
                  Setup Requirements ({agent.setup_requirements?.length})
                </h4>
                <div className="space-y-2">
                  {agent.setup_requirements?.map((req, i) => (
                    <div key={i} className="text-xs bg-background-deep px-2 py-1.5 rounded">
                      <div className="flex items-center gap-2">
                        <span className="text-accent-amber font-medium">{req.step}</span>
                        <span className="text-gray-600 text-[10px] uppercase">{req.type}</span>
                      </div>
                      <p className="text-gray-400 mt-0.5">{req.description}</p>
                      {req.command && (
                        <code className="block mt-1 text-gray-500 font-mono bg-background-mid px-1.5 py-0.5 rounded">
                          {req.command}
                        </code>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        )}
      </div>

      {children.length > 0 && (
        <div className="border-l-2 border-background-surface ml-5 pl-4 space-y-3">
          {children.map((child) => (
            <AgentNode
              key={child.id}
              agent={child}
              children={allAgents.filter((a) => a.parent_id === child.id)}
              allAgents={allAgents}
              depth={depth + 1}
            />
          ))}
        </div>
      )}
    </div>
  );
}

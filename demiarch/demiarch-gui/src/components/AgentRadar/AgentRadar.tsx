import { useState, useMemo } from 'react';
import { useAgents } from '../../contexts/AgentContext';
import type { Agent } from '../AgentStatus/types';

interface AgentRadarProps {
  onClose: () => void;
}

// Agent role classification for the neural hierarchy
type AgentRole = 'orchestrator' | 'planner' | 'worker';

function getAgentRole(agent: Agent): AgentRole {
  if (agent.type === 'orchestrator') return 'orchestrator';
  if (agent.type === 'planner') return 'planner';
  return 'worker'; // coder, reviewer, researcher are workers
}

// Colors for each role
const ROLE_COLORS: Record<AgentRole, string> = {
  orchestrator: '#14b8a6', // teal/cyan
  planner: '#ec4899', // pink/magenta
  worker: '#eab308', // yellow/gold
};

// Calculate position on a circle
function getCirclePosition(index: number, total: number, radius: number, centerX: number, centerY: number) {
  const angle = (index / total) * 2 * Math.PI - Math.PI / 2; // Start from top
  return {
    x: centerX + radius * Math.cos(angle),
    y: centerY + radius * Math.sin(angle),
  };
}

export function AgentRadar({ onClose }: AgentRadarProps) {
  const { agents, activities } = useAgents();
  const [selectedAgent, setSelectedAgent] = useState<Agent | null>(null);

  // Group agents by role
  const agentsByRole = useMemo(() => {
    const orchestrators: Agent[] = [];
    const planners: Agent[] = [];
    const workers: Agent[] = [];

    agents.forEach((agent) => {
      const role = getAgentRole(agent);
      if (role === 'orchestrator') orchestrators.push(agent);
      else if (role === 'planner') planners.push(agent);
      else workers.push(agent);
    });

    return { orchestrators, planners, workers };
  }, [agents]);

  // Get recent activities
  const recentActivities = useMemo(() => {
    return activities.slice(-10).reverse();
  }, [activities]);

  // Calculate stats
  const stats = useMemo(() => {
    const working = agents.filter((a) => a.status === 'working' || a.status === 'thinking').length;
    const tasksDispatched = activities.filter((a) => a.type === 'info').length;
    const completed = activities.filter((a) => a.type === 'success').length;
    return {
      orchestratorCount: agentsByRole.orchestrators.length,
      plannerCount: agentsByRole.planners.length,
      workerCount: agentsByRole.workers.length,
      workingCount: working,
      tasksDispatched,
      completed,
    };
  }, [agents, activities, agentsByRole]);

  const formatTime = (date: Date) => {
    return date.toLocaleTimeString('en-US', {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  };

  // SVG dimensions
  const svgWidth = 600;
  const svgHeight = 500;
  const centerX = svgWidth / 2;
  const centerY = svgHeight / 2;

  // Radius for each ring
  const plannerRadius = 100;
  const workerRadius = 180;

  return (
    <div className="agent-radar">
      <div className="agent-radar__header">
        <h2 className="agent-radar__title">AGENT MONITOR</h2>
        <div className="agent-radar__actions">
          <button className="agent-radar__btn">Pause All</button>
          <button className="agent-radar__btn">View Logs</button>
          <button className="agent-radar__btn agent-radar__btn--primary">Deploy Agent</button>
          <button className="agent-radar__close" onClick={onClose}>×</button>
        </div>
      </div>

      <div className="agent-radar__content">
        {/* Neural Hierarchy Visualization */}
        <div className="agent-radar__visualization">
          <svg
            width={svgWidth}
            height={svgHeight}
            viewBox={`0 0 ${svgWidth} ${svgHeight}`}
            className="agent-radar__svg"
          >
            {/* Background circles for rings */}
            <circle
              cx={centerX}
              cy={centerY}
              r={workerRadius + 40}
              fill="none"
              stroke="rgba(255,255,255,0.05)"
              strokeWidth="1"
              strokeDasharray="4 4"
            />
            <circle
              cx={centerX}
              cy={centerY}
              r={plannerRadius + 30}
              fill="none"
              stroke="rgba(255,255,255,0.08)"
              strokeWidth="1"
              strokeDasharray="4 4"
            />

            {/* Ring labels */}
            <text x={centerX} y={30} textAnchor="middle" fill="#666" fontSize="12" className="agent-radar__ring-label">
              WORKERS
            </text>
            <text x={centerX} y={centerY - plannerRadius - 40} textAnchor="middle" fill="#666" fontSize="12" className="agent-radar__ring-label">
              PLANNER
            </text>
            <text x={centerX} y={centerY + 70} textAnchor="middle" fill="#666" fontSize="12" className="agent-radar__ring-label">
              ORCHESTRATOR
            </text>

            {/* Connection lines from orchestrator to planners */}
            {agentsByRole.orchestrators.length > 0 && agentsByRole.planners.map((planner, i) => {
              const pos = getCirclePosition(i, agentsByRole.planners.length, plannerRadius, centerX, centerY);
              return (
                <line
                  key={`orch-plan-${planner.id}`}
                  x1={centerX}
                  y1={centerY}
                  x2={pos.x}
                  y2={pos.y}
                  stroke={ROLE_COLORS.orchestrator}
                  strokeWidth="2"
                  strokeDasharray="5 5"
                  opacity="0.5"
                />
              );
            })}

            {/* Connection lines from planners to workers */}
            {agentsByRole.planners.map((planner, plannerIdx) => {
              const plannerPos = getCirclePosition(plannerIdx, agentsByRole.planners.length, plannerRadius, centerX, centerY);
              // Connect each planner to nearby workers
              const workersPerPlanner = Math.ceil(agentsByRole.workers.length / Math.max(1, agentsByRole.planners.length));
              const startWorker = plannerIdx * workersPerPlanner;
              const endWorker = Math.min(startWorker + workersPerPlanner, agentsByRole.workers.length);

              return Array.from({ length: endWorker - startWorker }).map((_, wi) => {
                const workerIdx = startWorker + wi;
                if (workerIdx >= agentsByRole.workers.length) return null;
                const workerPos = getCirclePosition(workerIdx, agentsByRole.workers.length, workerRadius, centerX, centerY);
                return (
                  <line
                    key={`plan-work-${planner.id}-${workerIdx}`}
                    x1={plannerPos.x}
                    y1={plannerPos.y}
                    x2={workerPos.x}
                    y2={workerPos.y}
                    stroke={ROLE_COLORS.planner}
                    strokeWidth="1"
                    strokeDasharray="3 3"
                    opacity="0.3"
                  />
                );
              });
            })}

            {/* Worker nodes (outer ring) */}
            {agentsByRole.workers.map((agent, i) => {
              const pos = getCirclePosition(i, agentsByRole.workers.length, workerRadius, centerX, centerY);
              const isActive = agent.status === 'working' || agent.status === 'thinking';
              const isSelected = selectedAgent?.id === agent.id;
              return (
                <g key={agent.id} onClick={() => setSelectedAgent(agent)} style={{ cursor: 'pointer' }}>
                  <circle
                    cx={pos.x}
                    cy={pos.y}
                    r={isSelected ? 28 : 24}
                    fill={ROLE_COLORS.worker}
                    opacity={isActive ? 1 : 0.6}
                    stroke={isSelected ? '#fff' : 'none'}
                    strokeWidth="2"
                  />
                  {isActive && (
                    <circle
                      cx={pos.x}
                      cy={pos.y}
                      r={30}
                      fill="none"
                      stroke={ROLE_COLORS.worker}
                      strokeWidth="2"
                      opacity="0.3"
                      className="agent-radar__pulse"
                    />
                  )}
                </g>
              );
            })}

            {/* Planner nodes (middle ring) */}
            {agentsByRole.planners.map((agent, i) => {
              const pos = getCirclePosition(i, agentsByRole.planners.length, plannerRadius, centerX, centerY);
              const isActive = agent.status === 'working' || agent.status === 'thinking';
              const isSelected = selectedAgent?.id === agent.id;
              return (
                <g key={agent.id} onClick={() => setSelectedAgent(agent)} style={{ cursor: 'pointer' }}>
                  <circle
                    cx={pos.x}
                    cy={pos.y}
                    r={isSelected ? 32 : 28}
                    fill={ROLE_COLORS.planner}
                    opacity={isActive ? 1 : 0.6}
                    stroke={isSelected ? '#fff' : 'none'}
                    strokeWidth="2"
                  />
                  <text
                    x={pos.x}
                    y={pos.y + 5}
                    textAnchor="middle"
                    fill="#fff"
                    fontSize="14"
                    fontWeight="600"
                  >
                    P{i + 1}
                  </text>
                  {isActive && (
                    <circle
                      cx={pos.x}
                      cy={pos.y}
                      r={36}
                      fill="none"
                      stroke={ROLE_COLORS.planner}
                      strokeWidth="2"
                      opacity="0.3"
                      className="agent-radar__pulse"
                    />
                  )}
                </g>
              );
            })}

            {/* Orchestrator node (center) */}
            {agentsByRole.orchestrators.map((agent) => {
              const isActive = agent.status === 'working' || agent.status === 'thinking';
              const isSelected = selectedAgent?.id === agent.id;
              return (
                <g key={agent.id} onClick={() => setSelectedAgent(agent)} style={{ cursor: 'pointer' }}>
                  <circle
                    cx={centerX}
                    cy={centerY}
                    r={isSelected ? 38 : 34}
                    fill={ROLE_COLORS.orchestrator}
                    stroke={isSelected ? '#fff' : 'none'}
                    strokeWidth="2"
                  />
                  <text
                    x={centerX}
                    y={centerY + 6}
                    textAnchor="middle"
                    fill="#fff"
                    fontSize="18"
                    fontWeight="700"
                  >
                    O
                  </text>
                  {isActive && (
                    <circle
                      cx={centerX}
                      cy={centerY}
                      r={42}
                      fill="none"
                      stroke={ROLE_COLORS.orchestrator}
                      strokeWidth="2"
                      opacity="0.3"
                      className="agent-radar__pulse"
                    />
                  )}
                </g>
              );
            })}
          </svg>
        </div>

        {/* Right Panel - Stats & Activity */}
        <div className="agent-radar__panel">
          <div className="agent-radar__section">
            <h3 className="agent-radar__section-title">NEURAL HIERARCHY</h3>
            {selectedAgent ? (
              <p className="agent-radar__section-subtitle">{selectedAgent.name} - {selectedAgent.currentTask || 'Idle'}</p>
            ) : (
              <p className="agent-radar__section-subtitle">Click an agent to view details</p>
            )}
          </div>

          {/* Agent counts */}
          <div className="agent-radar__counts">
            <div className="agent-radar__count">
              <span className="agent-radar__count-value">{stats.orchestratorCount}</span>
              <span className="agent-radar__count-label">ORCHESTRATOR</span>
            </div>
            <div className="agent-radar__count">
              <span className="agent-radar__count-value">{stats.plannerCount}</span>
              <span className="agent-radar__count-label">PLANNERS</span>
            </div>
            <div className="agent-radar__count">
              <span className="agent-radar__count-value">{stats.workerCount}</span>
              <span className="agent-radar__count-label">WORKERS</span>
            </div>
          </div>

          {/* Activity Stream */}
          <div className="agent-radar__activity">
            <div className="agent-radar__activity-header">
              <h3 className="agent-radar__section-title">ACTIVITY STREAM</h3>
              <span className="agent-radar__live-badge">● LIVE</span>
            </div>
            <div className="agent-radar__activity-list">
              {recentActivities.length === 0 ? (
                <div className="agent-radar__no-activity">No recent activity</div>
              ) : (
                recentActivities.map((activity) => {
                  const agent = agents.find((a) => a.id === activity.agentId);
                  return (
                    <div key={activity.id} className={`agent-radar__activity-item agent-radar__activity-item--${activity.type}`}>
                      <span className="agent-radar__activity-time">{formatTime(activity.timestamp)}</span>
                      {agent && (
                        <span className="agent-radar__activity-agent">{agent.name}</span>
                      )}
                      <span className="agent-radar__activity-message">{activity.message}</span>
                    </div>
                  );
                })
              )}
            </div>
          </div>

          {/* Selected Agent Details or Orchestrator Stats */}
          <div className="agent-radar__details">
            {selectedAgent ? (
              <>
                <div className="agent-radar__detail-header">
                  <span
                    className="agent-radar__detail-dot"
                    style={{ background: ROLE_COLORS[getAgentRole(selectedAgent)] }}
                  />
                  <span className="agent-radar__detail-name">{selectedAgent.name}</span>
                  <span className="agent-radar__detail-type">{selectedAgent.type}</span>
                </div>
                <div className="agent-radar__detail-stats">
                  <div className="agent-radar__stat">
                    <span className="agent-radar__stat-value">{selectedAgent.tasks.length}</span>
                    <span className="agent-radar__stat-label">TASKS</span>
                  </div>
                  <div className="agent-radar__stat">
                    <span className="agent-radar__stat-value">{selectedAgent.status}</span>
                    <span className="agent-radar__stat-label">STATUS</span>
                  </div>
                </div>
              </>
            ) : (
              <>
                <div className="agent-radar__detail-header">
                  <span className="agent-radar__detail-dot" style={{ background: ROLE_COLORS.orchestrator }} />
                  <span className="agent-radar__detail-name">Orchestrator</span>
                  <span className="agent-radar__detail-type">Primary Controller</span>
                </div>
                <div className="agent-radar__detail-stats">
                  <div className="agent-radar__stat">
                    <span className="agent-radar__stat-value">{stats.tasksDispatched}</span>
                    <span className="agent-radar__stat-label">TASKS DISPATCHED</span>
                  </div>
                  <div className="agent-radar__stat">
                    <span className="agent-radar__stat-value">{stats.completed}</span>
                    <span className="agent-radar__stat-label">COMPLETED</span>
                  </div>
                </div>
              </>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

export default AgentRadar;

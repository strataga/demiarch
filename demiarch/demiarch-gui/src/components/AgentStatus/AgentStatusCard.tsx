import type { Agent } from './types';
import { STATUS_CONFIG, AGENT_TYPE_CONFIG } from './types';

interface AgentStatusCardProps {
  agent: Agent;
  isCompact?: boolean;
}

export function AgentStatusCard({ agent, isCompact = false }: AgentStatusCardProps) {
  const statusConfig = STATUS_CONFIG[agent.status];
  const typeConfig = AGENT_TYPE_CONFIG[agent.type];

  const statusClass = `agent-card--${agent.status}`;
  const compactClass = isCompact ? 'agent-card--compact' : '';

  return (
    <div className={`agent-card ${statusClass} ${compactClass}`}>
      <div className="agent-card__header">
        <div className="agent-card__identity">
          <span
            className="agent-card__status-indicator"
            style={{ color: statusConfig.color }}
            title={statusConfig.label}
          >
            {statusConfig.icon}
          </span>
          <span className="agent-card__name">{agent.name}</span>
        </div>
        <span
          className="agent-card__type-badge"
          style={{ backgroundColor: typeConfig.color }}
        >
          {typeConfig.label}
        </span>
      </div>

      {!isCompact && (
        <>
          <div className="agent-card__status-row">
            <span className="agent-card__status-label">Status:</span>
            <span
              className="agent-card__status-value"
              style={{ color: statusConfig.color }}
            >
              {statusConfig.label}
            </span>
          </div>

          {agent.currentTask && (
            <div className="agent-card__task">
              <span className="agent-card__task-label">Current Task:</span>
              <span className="agent-card__task-value">{agent.currentTask}</span>
            </div>
          )}

          {agent.progress !== undefined && agent.status === 'working' && (
            <div className="agent-card__progress">
              <div className="agent-card__progress-bar">
                <div
                  className="agent-card__progress-fill"
                  style={{
                    width: `${agent.progress}%`,
                    backgroundColor: statusConfig.color,
                  }}
                />
              </div>
              <span className="agent-card__progress-text">{agent.progress}%</span>
            </div>
          )}

          {agent.error && (
            <div className="agent-card__error">
              <span className="agent-card__error-icon">!</span>
              <span className="agent-card__error-text">{agent.error}</span>
            </div>
          )}

          {agent.tasks.length > 0 && (
            <div className="agent-card__tasks-summary">
              <span className="agent-card__tasks-count">
                {agent.tasks.filter((t) => t.completedAt).length}/{agent.tasks.length} tasks
              </span>
            </div>
          )}
        </>
      )}

      {isCompact && agent.currentTask && (
        <div className="agent-card__task-compact">
          {agent.currentTask}
        </div>
      )}
    </div>
  );
}

export default AgentStatusCard;

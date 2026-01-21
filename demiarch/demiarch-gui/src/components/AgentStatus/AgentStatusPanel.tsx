import { useState } from 'react';
import { AgentStatusCard } from './AgentStatusCard';
import { useAgents } from '../../contexts/AgentContext';
import type { AgentActivity } from './types';

interface AgentStatusPanelProps {
  showActivityLog?: boolean;
  maxActivities?: number;
}

export function AgentStatusPanel({
  showActivityLog = true,
  maxActivities = 50,
}: AgentStatusPanelProps) {
  const { agents, activities, clearActivities } = useAgents();
  const [isCollapsed, setIsCollapsed] = useState(false);
  const [activeTab, setActiveTab] = useState<'agents' | 'activity'>('agents');

  const activeAgents = agents.filter((a) => a.status !== 'idle' && a.status !== 'completed');
  const idleAgents = agents.filter((a) => a.status === 'idle' || a.status === 'completed');

  const recentActivities = activities.slice(-maxActivities).reverse();

  const formatTime = (date: Date) => {
    return date.toLocaleTimeString('en-US', {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  };

  const getActivityTypeClass = (type: AgentActivity['type']) => {
    return `activity-item--${type}`;
  };

  if (agents.length === 0) {
    return (
      <div className="agent-panel agent-panel--empty">
        <div className="agent-panel__header">
          <h3 className="agent-panel__title">Agents</h3>
        </div>
        <div className="agent-panel__empty-state">
          <span className="agent-panel__empty-icon">○</span>
          <p className="agent-panel__empty-text">No agents active</p>
        </div>
      </div>
    );
  }

  return (
    <div className={`agent-panel ${isCollapsed ? 'agent-panel--collapsed' : ''}`}>
      <div className="agent-panel__header">
        <h3 className="agent-panel__title">
          Agents
          {activeAgents.length > 0 && (
            <span className="agent-panel__active-count">{activeAgents.length} active</span>
          )}
        </h3>
        <button
          className="agent-panel__toggle"
          onClick={() => setIsCollapsed(!isCollapsed)}
          title={isCollapsed ? 'Expand panel' : 'Collapse panel'}
        >
          {isCollapsed ? '▶' : '▼'}
        </button>
      </div>

      {!isCollapsed && (
        <>
          {showActivityLog && (
            <div className="agent-panel__tabs">
              <button
                className={`agent-panel__tab ${activeTab === 'agents' ? 'agent-panel__tab--active' : ''}`}
                onClick={() => setActiveTab('agents')}
              >
                Agents ({agents.length})
              </button>
              <button
                className={`agent-panel__tab ${activeTab === 'activity' ? 'agent-panel__tab--active' : ''}`}
                onClick={() => setActiveTab('activity')}
              >
                Activity ({activities.length})
              </button>
            </div>
          )}

          {activeTab === 'agents' && (
            <div className="agent-panel__content">
              {activeAgents.length > 0 && (
                <div className="agent-panel__section">
                  <h4 className="agent-panel__section-title">Active</h4>
                  <div className="agent-panel__agents">
                    {activeAgents.map((agent) => (
                      <AgentStatusCard key={agent.id} agent={agent} />
                    ))}
                  </div>
                </div>
              )}

              {idleAgents.length > 0 && (
                <div className="agent-panel__section">
                  <h4 className="agent-panel__section-title">Idle</h4>
                  <div className="agent-panel__agents">
                    {idleAgents.map((agent) => (
                      <AgentStatusCard key={agent.id} agent={agent} isCompact />
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}

          {activeTab === 'activity' && (
            <div className="agent-panel__content">
              <div className="agent-panel__activity-header">
                <span className="agent-panel__activity-title">Recent Activity</span>
                {activities.length > 0 && (
                  <button
                    className="agent-panel__clear-btn"
                    onClick={clearActivities}
                    title="Clear activity log"
                  >
                    Clear
                  </button>
                )}
              </div>
              <div className="agent-panel__activity-log">
                {recentActivities.length === 0 ? (
                  <div className="agent-panel__no-activity">No activity recorded</div>
                ) : (
                  recentActivities.map((activity) => {
                    const agent = agents.find((a) => a.id === activity.agentId);
                    return (
                      <div
                        key={activity.id}
                        className={`activity-item ${getActivityTypeClass(activity.type)}`}
                      >
                        <span className="activity-item__time">
                          {formatTime(activity.timestamp)}
                        </span>
                        {agent && (
                          <span className="activity-item__agent">{agent.name}</span>
                        )}
                        <span className="activity-item__message">{activity.message}</span>
                      </div>
                    );
                  })
                )}
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}

export default AgentStatusPanel;

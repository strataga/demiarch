import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { KanbanBoard } from "./components/KanbanBoard";
import { ProjectProvider, useProjects } from "./contexts/ProjectContext";
import { AgentProvider, useAgents } from "./contexts/AgentContext";
import { ConflictProvider, useConflicts } from "./contexts/ConflictContext";
import { ProjectSelector } from "./components/ProjectSelector";
import { ProjectProgress } from "./components/ProjectProgress";
import { AgentRadar } from "./components/AgentRadar";
import { AgentStatusPanel } from "./components/AgentStatus";
import { ConflictPanel } from "./components/ConflictPanel";
import "./App.css";
import "./components/AgentStatus/AgentStatus.css";
import "./components/ConflictPanel/ConflictPanel.css";

interface AppInfo {
  name: string;
  version: string;
  description: string;
}

interface HealthStatus {
  status: string;
  message: string;
}

function AppContent() {
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [healthStatus, setHealthStatus] = useState<HealthStatus | null>(null);
  const [isAppLoading, setIsAppLoading] = useState(true);
  const [showAgentRadar, setShowAgentRadar] = useState(false);
  const [showAgentPanel, setShowAgentPanel] = useState(false);
  const { currentProject, updateProjectBoard, isLoading: isProjectsLoading, error: projectsError, refreshProjects } = useProjects();
  const { checkForConflicts, isPanelVisible, setPanelVisible, summary } = useConflicts();
  const { agents } = useAgents();

  // Calculate agent stats for HUD
  const activeAgents = agents.filter((a) => a.status === 'working' || a.status === 'thinking');

  useEffect(() => {
    async function initialize() {
      try {
        const [info, health] = await Promise.all([
          invoke<AppInfo>("get_app_info"),
          invoke<HealthStatus>("health_check"),
        ]);
        setAppInfo(info);
        setHealthStatus(health);
      } catch (error) {
        console.error("Failed to initialize:", error);
      } finally {
        setIsAppLoading(false);
      }
    }
    initialize();
  }, []);

  // Calculate conflict count
  const conflictCount = summary ? summary.modifiedFiles.length + summary.deletedFiles.length : 0;

  if (isAppLoading) {
    return (
      <div className="app-loading">
        <div className="spinner" />
        <p>Loading Demiarch...</p>
      </div>
    );
  }

  return (
    <div className="app-layout">
      {/* Compact Header */}
      <header className="app-header">
        <div className="app-header__left">
          <h1 className="app-title">{appInfo?.name || "Demiarch"}</h1>
          <span className="app-version">v{appInfo?.version}</span>
          <ProjectSelector />
        </div>
        <div className="app-header__right">
          {/* Agent HUD Button */}
          <button
            className={`header-btn header-btn--agents ${showAgentRadar ? 'header-btn--active' : ''}`}
            onClick={() => setShowAgentRadar(!showAgentRadar)}
            title="Agent Monitor"
          >
            <span className="agent-dots">
              {agents.slice(0, 3).map((agent) => (
                <span
                  key={agent.id}
                  className={`agent-dot agent-dot--${agent.status}`}
                />
              ))}
              {agents.length === 0 && (
                <span className="agent-dot agent-dot--idle" />
              )}
            </span>
            {activeAgents.length > 0 && (
              <span className="header-btn__count">{activeAgents.length}</span>
            )}
          </button>

          {/* Conflict Badge */}
          {conflictCount > 0 && (
            <button
              className="header-btn header-btn--warning"
              onClick={() => {
                if (isPanelVisible) {
                  setPanelVisible(false);
                } else {
                  checkForConflicts('current-project');
                }
              }}
              title="Conflicts detected"
            >
              <span className="warning-icon">⚠</span>
              <span className="conflict-count">{conflictCount}</span>
            </button>
          )}

          {/* Health Status */}
          <div className={`status-dot ${healthStatus?.status}`} title={healthStatus?.message} />
        </div>
      </header>

      {/* Main Content - Full Screen Kanban */}
      <main className="app-main">
        {isProjectsLoading ? (
          <div className="app-main__empty">
            <div className="spinner" />
            <p>Loading projects...</p>
          </div>
        ) : projectsError ? (
          <div className="app-main__empty">
            <p className="error-text">Failed to load projects</p>
            <p className="hint-text">{projectsError}</p>
            <button className="retry-btn" onClick={() => refreshProjects()}>Retry</button>
          </div>
        ) : currentProject ? (
          <>
            {/* Project Progress Bar */}
            <div className="project-progress-strip">
              <ProjectProgress />
            </div>
            <KanbanBoard
              key={currentProject.id}
              initialBoard={currentProject.board}
              onBoardChange={updateProjectBoard}
            />
          </>
        ) : (
          <div className="app-main__empty">
            <p>No project selected</p>
            <p className="hint-text">Select a project from the dropdown above</p>
          </div>
        )}
      </main>

      {/* Agent Radar Overlay */}
      {showAgentRadar && (
        <div className="overlay-backdrop" onClick={() => setShowAgentRadar(false)}>
          <div className="overlay-panel overlay-panel--radar" onClick={(e) => e.stopPropagation()}>
            <AgentRadar onClose={() => setShowAgentRadar(false)} />
          </div>
        </div>
      )}

      {/* Slide-out Agent Detail Panel */}
      {showAgentPanel && (
        <div className="slide-panel slide-panel--right">
          <div className="slide-panel__header">
            <h3 className="slide-panel__title">Agents</h3>
            <button className="slide-panel__close" onClick={() => setShowAgentPanel(false)}>×</button>
          </div>
          <div className="slide-panel__content">
            <AgentStatusPanel showActivityLog={true} />
          </div>
        </div>
      )}

      {/* Slide-out Conflict Panel */}
      {isPanelVisible && (
        <div className="slide-panel slide-panel--right">
          <div className="slide-panel__header">
            <h3 className="slide-panel__title">Conflicts</h3>
            <button className="slide-panel__close" onClick={() => setPanelVisible(false)}>×</button>
          </div>
          <div className="slide-panel__content">
            <ConflictPanel />
          </div>
        </div>
      )}

      {/* Keyboard Hints */}
      <div className="keyboard-hints">
        <div className="key-hint"><span className="key">A</span> Agents</div>
        <div className="key-hint"><span className="key">C</span> Conflicts</div>
        <div className="key-hint"><span className="key">N</span> New Card</div>
        <div className="key-hint"><span className="key">?</span> Help</div>
      </div>
    </div>
  );
}

function App() {
  return (
    <ProjectProvider>
      <AgentProvider>
        <ConflictProvider>
          <AppContent />
        </ConflictProvider>
      </AgentProvider>
    </ProjectProvider>
  );
}

export default App;

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { KanbanBoard } from "./components/KanbanBoard";
import { ProjectProvider, useProjects } from "./contexts/ProjectContext";
import { AgentProvider } from "./contexts/AgentContext";
import { ProjectSelector } from "./components/ProjectSelector";
import { AgentStatusPanel } from "./components/AgentStatus";
import { SAMPLE_PROJECTS } from "./data/sampleProjects";
import { SAMPLE_AGENTS } from "./data/sampleAgents";
import "./App.css";
import "./components/AgentStatus/AgentStatus.css";

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
  const [greeting, setGreeting] = useState<string>("");
  const [name, setName] = useState<string>("");
  const [isLoading, setIsLoading] = useState(true);
  const { currentProject, updateProjectBoard } = useProjects();

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
        setIsLoading(false);
      }
    }
    initialize();
  }, []);

  async function handleGreet() {
    if (!name.trim()) return;
    try {
      const result = await invoke<string>("greet", { name: name.trim() });
      setGreeting(result);
    } catch (error) {
      console.error("Greet failed:", error);
    }
  }

  if (isLoading) {
    return (
      <div className="container loading">
        <div className="spinner" />
        <p>Loading Demiarch...</p>
      </div>
    );
  }

  return (
    <main className="container">
      <header className="header">
        <div className="header__top">
          <div className="header__branding">
            <h1>{appInfo?.name || "Demiarch"}</h1>
            <span className="version">v{appInfo?.version}</span>
          </div>
          <ProjectSelector />
        </div>
        <p className="tagline">{appInfo?.description}</p>
      </header>

      <section className="status-section">
        <div className={`status-indicator ${healthStatus?.status}`}>
          <span className="status-dot" />
          <span>{healthStatus?.message}</span>
        </div>
      </section>

      <section className="interaction-section">
        <h2>Test IPC Communication</h2>
        <div className="input-group">
          <input
            type="text"
            placeholder="Enter your name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleGreet()}
          />
          <button onClick={handleGreet} disabled={!name.trim()}>
            Greet
          </button>
        </div>
        {greeting && <p className="greeting-result">{greeting}</p>}
      </section>

      <section className="agent-section">
        <AgentStatusPanel showActivityLog={true} />
      </section>

      <section className="kanban-section">
        {currentProject ? (
          <KanbanBoard
            key={currentProject.id}
            initialBoard={currentProject.board}
            onBoardChange={updateProjectBoard}
          />
        ) : (
          <div className="kanban-section__empty">
            <p>No project selected</p>
            <p className="kanban-section__empty-hint">Select a project from the dropdown above to view its board</p>
          </div>
        )}
      </section>

      <footer className="footer">
        <p>
          Built with{" "}
          <a href="https://tauri.app" target="_blank" rel="noreferrer">
            Tauri
          </a>{" "}
          +{" "}
          <a href="https://react.dev" target="_blank" rel="noreferrer">
            React
          </a>
        </p>
      </footer>
    </main>
  );
}

function App() {
  return (
    <ProjectProvider initialProjects={SAMPLE_PROJECTS}>
      <AgentProvider initialAgents={SAMPLE_AGENTS}>
        <AppContent />
      </AgentProvider>
    </ProjectProvider>
  );
}

export default App;

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

interface AppInfo {
  name: string;
  version: string;
  description: string;
}

interface HealthStatus {
  status: string;
  message: string;
}

function App() {
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [healthStatus, setHealthStatus] = useState<HealthStatus | null>(null);
  const [greeting, setGreeting] = useState<string>("");
  const [name, setName] = useState<string>("");
  const [isLoading, setIsLoading] = useState(true);

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
        <h1>{appInfo?.name || "Demiarch"}</h1>
        <p className="tagline">{appInfo?.description}</p>
        <span className="version">v{appInfo?.version}</span>
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

      <section className="features-section">
        <h2>Features</h2>
        <div className="features-grid">
          <div className="feature-card">
            <div className="feature-icon">🏠</div>
            <h3>Local-First</h3>
            <p>All data stored locally in SQLite</p>
          </div>
          <div className="feature-card">
            <div className="feature-icon">🤖</div>
            <h3>AI-Powered</h3>
            <p>Generate code through conversation</p>
          </div>
          <div className="feature-card">
            <div className="feature-icon">🔒</div>
            <h3>Secure</h3>
            <p>Encrypted API keys, no telemetry</p>
          </div>
          <div className="feature-card">
            <div className="feature-icon">📦</div>
            <h3>You Own It</h3>
            <p>Generated code is yours to keep</p>
          </div>
        </div>
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

export default App;
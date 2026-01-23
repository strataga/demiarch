//! Tauri command handlers
//!
//! These commands bridge the React frontend to demiarch-core functionality.
//! Each command is exposed to the frontend via Tauri's invoke system.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Project summary for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub framework: String,
    pub status: String,
    pub feature_count: usize,
}

/// Feature summary for Kanban board
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: i32,
    pub phase_id: String,
}

/// Agent status for visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    pub id: String,
    pub agent_type: String,
    pub status: String,
    pub parent_id: Option<String>,
    pub task: Option<String>,
    pub tokens_used: u64,
}

/// Cost summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSummary {
    pub today_usd: f64,
    pub daily_limit_usd: f64,
    pub remaining_usd: f64,
    pub alert_threshold: f64,
}

/// Session summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub status: String,
    pub current_project_id: Option<String>,
    pub started_at: String,
}

/// Doctor check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorResult {
    pub config_ok: bool,
    pub api_key_ok: bool,
    pub database_ok: bool,
    pub schema_version: i32,
    pub project_count: usize,
}

/// Conflict hunk for resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictHunk {
    pub id: String,
    pub start_line: u32,
    pub end_line: u32,
    pub user_content: String,
    pub ai_content: String,
    pub resolved: bool,
    pub resolution: Option<String>,
    pub custom_content: Option<String>,
}

/// Conflict for a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub id: String,
    pub file_path: String,
    pub hunks: Vec<ConflictHunk>,
    pub created_at: String,
}

// ============================================================
// Project Commands
// ============================================================

#[tauri::command]
pub async fn get_projects() -> Result<Vec<ProjectSummary>, String> {
    // TODO: Connect to demiarch-core
    // For now, return mock data to verify the bridge works
    Ok(vec![
        ProjectSummary {
            id: "f78b82f0".to_string(),
            name: "analytics-dashboard".to_string(),
            framework: "nextjs".to_string(),
            status: "building".to_string(),
            feature_count: 5,
        },
        ProjectSummary {
            id: "58f7a856".to_string(),
            name: "task-tracker".to_string(),
            framework: "nextjs".to_string(),
            status: "discovery".to_string(),
            feature_count: 3,
        },
    ])
}

#[tauri::command]
pub async fn get_project(id: String) -> Result<ProjectSummary, String> {
    // TODO: Connect to demiarch-core
    Ok(ProjectSummary {
        id,
        name: "analytics-dashboard".to_string(),
        framework: "nextjs".to_string(),
        status: "building".to_string(),
        feature_count: 5,
    })
}

#[tauri::command]
pub async fn create_project(name: String, framework: String) -> Result<ProjectSummary, String> {
    // TODO: Connect to demiarch-core
    let id = Uuid::new_v4().to_string()[..8].to_string();
    Ok(ProjectSummary {
        id,
        name,
        framework,
        status: "discovery".to_string(),
        feature_count: 0,
    })
}

// ============================================================
// Feature Commands
// ============================================================

#[tauri::command]
pub async fn get_features(project_id: String) -> Result<Vec<FeatureSummary>, String> {
    // TODO: Connect to demiarch-core
    let _ = project_id;
    Ok(vec![
        FeatureSummary {
            id: "feat-001".to_string(),
            name: "User Authentication".to_string(),
            description: Some("OAuth2 login with Google and GitHub".to_string()),
            status: "complete".to_string(),
            priority: 1,
            phase_id: "phase-1".to_string(),
        },
        FeatureSummary {
            id: "feat-002".to_string(),
            name: "Dashboard Layout".to_string(),
            description: Some("Main dashboard with sidebar navigation".to_string()),
            status: "in_progress".to_string(),
            priority: 1,
            phase_id: "phase-1".to_string(),
        },
        FeatureSummary {
            id: "feat-003".to_string(),
            name: "Analytics Charts".to_string(),
            description: Some("Interactive charts with Chart.js".to_string()),
            status: "pending".to_string(),
            priority: 2,
            phase_id: "phase-2".to_string(),
        },
    ])
}

#[tauri::command]
pub async fn get_feature(id: String) -> Result<FeatureSummary, String> {
    // TODO: Connect to demiarch-core
    Ok(FeatureSummary {
        id,
        name: "User Authentication".to_string(),
        description: Some("OAuth2 login with Google and GitHub".to_string()),
        status: "complete".to_string(),
        priority: 1,
        phase_id: "phase-1".to_string(),
    })
}

#[tauri::command]
pub async fn update_feature_status(id: String, status: String) -> Result<FeatureSummary, String> {
    // TODO: Connect to demiarch-core
    Ok(FeatureSummary {
        id,
        name: "Feature".to_string(),
        description: None,
        status,
        priority: 2,
        phase_id: "phase-1".to_string(),
    })
}

// ============================================================
// Session Commands
// ============================================================

#[tauri::command]
pub async fn get_sessions() -> Result<Vec<SessionSummary>, String> {
    // TODO: Connect to demiarch-core
    Ok(vec![SessionSummary {
        id: "session-001".to_string(),
        status: "active".to_string(),
        current_project_id: Some("f78b82f0".to_string()),
        started_at: "2026-01-22T10:00:00Z".to_string(),
    }])
}

// ============================================================
// Cost Commands
// ============================================================

#[tauri::command]
pub async fn get_costs() -> Result<CostSummary, String> {
    // TODO: Connect to demiarch-core
    Ok(CostSummary {
        today_usd: 0.0,
        daily_limit_usd: 10.0,
        remaining_usd: 10.0,
        alert_threshold: 0.8,
    })
}

// ============================================================
// Agent Commands
// ============================================================

#[tauri::command]
pub async fn get_agents() -> Result<Vec<AgentStatus>, String> {
    // TODO: Connect to demiarch-core
    Ok(vec![
        AgentStatus {
            id: "agent-001".to_string(),
            agent_type: "orchestrator".to_string(),
            status: "running".to_string(),
            parent_id: None,
            task: Some("Coordinating feature generation".to_string()),
            tokens_used: 150,
        },
        AgentStatus {
            id: "agent-002".to_string(),
            agent_type: "planner".to_string(),
            status: "running".to_string(),
            parent_id: Some("agent-001".to_string()),
            task: Some("Decomposing tasks".to_string()),
            tokens_used: 320,
        },
    ])
}

// ============================================================
// System Commands
// ============================================================

#[tauri::command]
pub async fn doctor() -> Result<DoctorResult, String> {
    // TODO: Connect to demiarch-core
    Ok(DoctorResult {
        config_ok: true,
        api_key_ok: true,
        database_ok: true,
        schema_version: 11,
        project_count: 3,
    })
}

// ============================================================
// Conflict Resolution Commands
// ============================================================

#[tauri::command]
pub async fn get_conflicts(project_id: String) -> Result<Vec<Conflict>, String> {
    // TODO: Connect to demiarch-core
    let _ = project_id;
    // Return mock conflicts for UI development
    Ok(vec![
        Conflict {
            id: "conflict-001".to_string(),
            file_path: "src/components/Dashboard.tsx".to_string(),
            hunks: vec![
                ConflictHunk {
                    id: "hunk-001".to_string(),
                    start_line: 15,
                    end_line: 28,
                    user_content: r#"function Dashboard() {
  const [data, setData] = useState([]);

  useEffect(() => {
    // User's custom fetch logic
    fetch('/api/data')
      .then(res => res.json())
      .then(setData);
  }, []);

  return <div>{data.map(renderItem)}</div>;
}"#.to_string(),
                    ai_content: r#"function Dashboard() {
  const { data, isLoading, error } = useQuery({
    queryKey: ['dashboard'],
    queryFn: () => fetchDashboardData(),
  });

  if (isLoading) return <Skeleton />;
  if (error) return <ErrorMessage error={error} />;

  return <DashboardGrid data={data} />;
}"#.to_string(),
                    resolved: false,
                    resolution: None,
                    custom_content: None,
                },
                ConflictHunk {
                    id: "hunk-002".to_string(),
                    start_line: 45,
                    end_line: 52,
                    user_content: r#"const styles = {
  container: 'flex flex-col gap-4',
  header: 'text-xl font-bold',
};"#.to_string(),
                    ai_content: r#"const styles = {
  container: 'grid grid-cols-3 gap-6 p-4',
  header: 'text-2xl font-semibold tracking-tight',
  subheader: 'text-gray-500 text-sm',
};"#.to_string(),
                    resolved: false,
                    resolution: None,
                    custom_content: None,
                },
            ],
            created_at: "2026-01-22T14:30:00Z".to_string(),
        },
    ])
}

#[tauri::command]
pub async fn resolve_conflict_hunk(
    conflict_id: String,
    hunk_id: String,
    resolution: String,
    custom_content: Option<String>,
) -> Result<(), String> {
    // TODO: Connect to demiarch-core
    let _ = (conflict_id, hunk_id, resolution, custom_content);
    Ok(())
}

#[tauri::command]
pub async fn apply_conflict_resolutions(conflict_id: String) -> Result<(), String> {
    // TODO: Connect to demiarch-core
    let _ = conflict_id;
    Ok(())
}

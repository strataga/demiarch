import { useEffect, useState } from 'react';
import { invoke } from '../lib/api';
import { Link } from 'react-router-dom';
import {
  FolderOpen,
  Activity,
  DollarSign,
  CheckCircle2,
  ArrowRight
} from 'lucide-react';

interface ProjectSummary {
  id: string;
  name: string;
  framework: string;
  status: string;
  feature_count: number;
}

interface CostSummary {
  today_usd: number;
  daily_limit_usd: number;
  remaining_usd: number;
  alert_threshold: number;
}

interface DoctorResult {
  config_ok: boolean;
  api_key_ok: boolean;
  database_ok: boolean;
  schema_version: number;
  project_count: number;
}

export default function Dashboard() {
  const [projects, setProjects] = useState<ProjectSummary[]>([]);
  const [costs, setCosts] = useState<CostSummary | null>(null);
  const [health, setHealth] = useState<DoctorResult | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function loadData() {
      try {
        const [projectsData, costsData, healthData] = await Promise.all([
          invoke<ProjectSummary[]>('get_projects'),
          invoke<CostSummary>('get_costs'),
          invoke<DoctorResult>('doctor'),
        ]);
        setProjects(projectsData);
        setCosts(costsData);
        setHealth(healthData);
      } catch (error) {
        console.error('Failed to load dashboard data:', error);
      } finally {
        setLoading(false);
      }
    }
    loadData();
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="animate-pulse text-accent-teal">Loading...</div>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      <h1 className="text-2xl font-bold">Dashboard</h1>

      {/* Stats Grid */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <StatCard
          icon={FolderOpen}
          label="Projects"
          value={health?.project_count || 0}
          color="teal"
        />
        <StatCard
          icon={Activity}
          label="Schema Version"
          value={`v${health?.schema_version || 0}`}
          color="magenta"
        />
        <StatCard
          icon={DollarSign}
          label="Today's Cost"
          value={`$${costs?.today_usd.toFixed(2) || '0.00'}`}
          color="amber"
        />
        <StatCard
          icon={CheckCircle2}
          label="Health"
          value={health?.config_ok && health?.database_ok ? 'OK' : 'Issues'}
          color={health?.config_ok && health?.database_ok ? 'teal' : 'magenta'}
        />
      </div>

      {/* Projects List */}
      <div className="bg-background-mid rounded-lg border border-background-surface">
        <div className="p-4 border-b border-background-surface flex justify-between items-center">
          <h2 className="text-lg font-semibold">Recent Projects</h2>
          <Link
            to="/projects"
            className="text-accent-teal text-sm hover:underline flex items-center gap-1"
          >
            View all <ArrowRight className="w-4 h-4" />
          </Link>
        </div>
        <div className="divide-y divide-background-surface">
          {projects.map((project) => (
            <Link
              key={project.id}
              to={`/projects/${project.id}`}
              className="flex items-center justify-between p-4 hover:bg-background-surface transition-colors"
            >
              <div>
                <h3 className="font-medium">{project.name}</h3>
                <p className="text-sm text-gray-400">
                  {project.framework} · {project.feature_count} features
                </p>
              </div>
              <StatusBadge status={project.status} />
            </Link>
          ))}
        </div>
      </div>
    </div>
  );
}

function StatCard({
  icon: Icon,
  label,
  value,
  color
}: {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  value: string | number;
  color: 'teal' | 'magenta' | 'amber';
}) {
  const colorClasses = {
    teal: 'text-accent-teal bg-accent-teal/10',
    magenta: 'text-accent-magenta bg-accent-magenta/10',
    amber: 'text-accent-amber bg-accent-amber/10',
  };

  return (
    <div className="bg-background-mid rounded-lg border border-background-surface p-4">
      <div className={`w-10 h-10 rounded-lg ${colorClasses[color]} flex items-center justify-center mb-3`}>
        <Icon className="w-5 h-5" />
      </div>
      <p className="text-sm text-gray-400">{label}</p>
      <p className="text-2xl font-bold">{value}</p>
    </div>
  );
}

function StatusBadge({ status }: { status: string }) {
  const colors: Record<string, string> = {
    discovery: 'bg-blue-500/20 text-blue-400',
    planning: 'bg-purple-500/20 text-purple-400',
    building: 'bg-accent-amber/20 text-accent-amber',
    complete: 'bg-accent-teal/20 text-accent-teal',
    archived: 'bg-gray-500/20 text-gray-400',
  };

  return (
    <span className={`px-2 py-1 rounded text-xs font-medium ${colors[status] || colors.discovery}`}>
      {status}
    </span>
  );
}

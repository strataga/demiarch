import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Link } from 'react-router-dom';
import { Plus, FolderOpen, ArrowRight } from 'lucide-react';

interface ProjectSummary {
  id: string;
  name: string;
  framework: string;
  status: string;
  feature_count: number;
}

export default function Projects() {
  const [projects, setProjects] = useState<ProjectSummary[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function loadProjects() {
      try {
        const data = await invoke<ProjectSummary[]>('get_projects');
        setProjects(data);
      } catch (error) {
        console.error('Failed to load projects:', error);
      } finally {
        setLoading(false);
      }
    }
    loadProjects();
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="animate-pulse text-accent-teal">Loading projects...</div>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      <div className="flex justify-between items-center">
        <h1 className="text-2xl font-bold">Projects</h1>
        <button className="flex items-center gap-2 px-4 py-2 bg-accent-teal text-background-deep rounded-lg font-medium hover:bg-accent-teal/90 transition-colors">
          <Plus className="w-5 h-5" />
          New Project
        </button>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {projects.map((project) => (
          <Link
            key={project.id}
            to={`/projects/${project.id}`}
            className="bg-background-mid rounded-lg border border-background-surface p-4 card-hover group"
          >
            <div className="flex items-start justify-between mb-3">
              <div className="w-12 h-12 rounded-lg bg-accent-teal/10 flex items-center justify-center">
                <FolderOpen className="w-6 h-6 text-accent-teal" />
              </div>
              <StatusBadge status={project.status} />
            </div>
            <h3 className="font-semibold text-lg mb-1">{project.name}</h3>
            <p className="text-sm text-gray-400 mb-3">{project.framework}</p>
            <div className="flex justify-between items-center text-sm">
              <span className="text-gray-500">{project.feature_count} features</span>
              <span className="text-accent-teal opacity-0 group-hover:opacity-100 transition-opacity flex items-center gap-1">
                Open <ArrowRight className="w-4 h-4" />
              </span>
            </div>
          </Link>
        ))}
      </div>
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

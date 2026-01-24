import { useEffect, useState } from 'react';
import { useParams, Link } from 'react-router-dom';
import { invoke } from '../lib/api';
import { ArrowLeft, FileText, LayoutGrid, Clock } from 'lucide-react';
import ReactMarkdown from 'react-markdown';

interface Project {
  id: string;
  name: string;
  framework: string;
  status: string;
  feature_count: number;
  prd?: string;
  description?: string;
  created_at?: string;
}

export default function ProjectDetail() {
  const { projectId } = useParams<{ projectId: string }>();
  const [project, setProject] = useState<Project | null>(null);
  const [loading, setLoading] = useState(true);
  const [activeTab, setActiveTab] = useState<'prd' | 'kanban'>('prd');

  useEffect(() => {
    async function loadProject() {
      if (!projectId) return;
      try {
        const data = await invoke<Project>('get_project', { id: projectId });
        setProject(data);
      } catch (error) {
        console.error('Failed to load project:', error);
      } finally {
        setLoading(false);
      }
    }
    loadProject();
  }, [projectId]);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="animate-pulse text-accent-teal">Loading project...</div>
      </div>
    );
  }

  if (!project) {
    return (
      <div className="p-6">
        <p className="text-gray-400">Project not found</p>
        <Link to="/projects" className="text-accent-teal hover:underline">
          Back to projects
        </Link>
      </div>
    );
  }

  return (
    <div className="p-6 h-full flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-4">
          <Link
            to="/projects"
            className="p-2 rounded-lg hover:bg-background-surface transition-colors"
          >
            <ArrowLeft className="w-5 h-5" />
          </Link>
          <div>
            <h1 className="text-2xl font-bold">{project.name}</h1>
            <div className="flex items-center gap-3 text-sm text-gray-400">
              <span className="capitalize">{project.status}</span>
              {project.created_at && (
                <>
                  <span>·</span>
                  <span className="flex items-center gap-1">
                    <Clock className="w-4 h-4" />
                    {new Date(project.created_at).toLocaleDateString()}
                  </span>
                </>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-2 mb-4">
        <button
          onClick={() => setActiveTab('prd')}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg font-medium transition-colors ${
            activeTab === 'prd'
              ? 'bg-accent-teal text-background-deep'
              : 'bg-background-surface text-gray-400 hover:text-white'
          }`}
        >
          <FileText className="w-4 h-4" />
          PRD
        </button>
        <Link
          to={`/projects/${projectId}/kanban`}
          className="flex items-center gap-2 px-4 py-2 rounded-lg font-medium bg-background-surface text-gray-400 hover:text-white transition-colors"
        >
          <LayoutGrid className="w-4 h-4" />
          Kanban Board
        </Link>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto bg-background-mid rounded-lg border border-background-surface p-6">
        {project.prd ? (
          <div className="prose prose-invert max-w-none">
            <ReactMarkdown>{project.prd}</ReactMarkdown>
          </div>
        ) : (
          <div className="text-center py-12 text-gray-400">
            <FileText className="w-12 h-12 mx-auto mb-3 opacity-50" />
            <p>No PRD generated yet</p>
            <p className="text-sm mt-2">
              This project was created without a PRD. You can add one by editing the project.
            </p>
          </div>
        )}
      </div>
    </div>
  );
}

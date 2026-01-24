import { useEffect, useState } from 'react';
import { useParams, Link, useNavigate } from 'react-router-dom';
import { invoke, Feature } from '../lib/api';
import { ArrowLeft, FileText, LayoutGrid, Clock, Edit3, Save, X, Sparkles } from 'lucide-react';
import ReactMarkdown from 'react-markdown';
import ExtractFeaturesModal from '../components/ExtractFeaturesModal';

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
  const navigate = useNavigate();
  const [project, setProject] = useState<Project | null>(null);
  const [loading, setLoading] = useState(true);
  const [activeTab, setActiveTab] = useState<'prd' | 'kanban'>('prd');
  const [isEditing, setIsEditing] = useState(false);
  const [editedPrd, setEditedPrd] = useState('');
  const [saving, setSaving] = useState(false);
  const [showExtractModal, setShowExtractModal] = useState(false);

  useEffect(() => {
    async function loadProject() {
      if (!projectId) return;
      try {
        const data = await invoke<Project>('get_project', { id: projectId });
        setProject(data);
        setEditedPrd(data?.prd || '');
      } catch (error) {
        console.error('Failed to load project:', error);
      } finally {
        setLoading(false);
      }
    }
    loadProject();
  }, [projectId]);

  function handleStartEdit() {
    setEditedPrd(project?.prd || '');
    setIsEditing(true);
  }

  function handleCancelEdit() {
    setEditedPrd(project?.prd || '');
    setIsEditing(false);
  }

  async function handleSavePrd() {
    if (!project) return;
    setSaving(true);
    try {
      const updated = await invoke<Project>('update_project', {
        id: project.id,
        prd: editedPrd,
        status: editedPrd ? 'planning' : 'discovery',
      });
      setProject(updated);
      setIsEditing(false);
    } catch (error) {
      console.error('Failed to save PRD:', error);
    } finally {
      setSaving(false);
    }
  }

  function handleFeaturesCreated(features: Feature[]) {
    setShowExtractModal(false);
    // Update project feature count
    if (project) {
      setProject({
        ...project,
        feature_count: project.feature_count + features.length,
        status: 'building',
      });
    }
    // Navigate to kanban to see the new features
    navigate(`/projects/${projectId}/kanban`);
  }

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
        {/* Edit/Save buttons */}
        {!isEditing ? (
          <div className="flex gap-2">
            {project.prd && (
              <button
                onClick={() => setShowExtractModal(true)}
                className="flex items-center gap-2 px-4 py-2 bg-accent-teal/10 text-accent-teal hover:bg-accent-teal/20 rounded-lg transition-colors"
              >
                <Sparkles className="w-4 h-4" />
                Extract Features
              </button>
            )}
            <button
              onClick={handleStartEdit}
              className="flex items-center gap-2 px-4 py-2 bg-background-surface text-gray-300 hover:text-white rounded-lg transition-colors"
            >
              <Edit3 className="w-4 h-4" />
              Edit PRD
            </button>
          </div>
        ) : (
          <div className="flex gap-2">
            <button
              onClick={handleCancelEdit}
              className="flex items-center gap-2 px-4 py-2 bg-background-surface text-gray-300 hover:text-white rounded-lg transition-colors"
            >
              <X className="w-4 h-4" />
              Cancel
            </button>
            <button
              onClick={handleSavePrd}
              disabled={saving}
              className="flex items-center gap-2 px-4 py-2 bg-accent-teal text-background-deep rounded-lg font-medium hover:bg-accent-teal/90 transition-colors disabled:opacity-50"
            >
              <Save className="w-4 h-4" />
              {saving ? 'Saving...' : 'Save'}
            </button>
          </div>
        )}
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
        {isEditing ? (
          <textarea
            value={editedPrd}
            onChange={(e) => setEditedPrd(e.target.value)}
            placeholder="# Product Requirements Document

## Executive Summary
What are we building and why?

## Problem Statement
- **The Problem**:
- **Who Has It**:
- **Current Alternatives**:

## Core Features (MVP)
..."
            className="w-full h-full min-h-[500px] bg-background-deep border border-background-surface rounded-lg p-4 text-white font-mono text-sm resize-none focus:outline-none focus:ring-2 focus:ring-accent-teal"
          />
        ) : project.prd ? (
          <div className="prose prose-invert max-w-none">
            <ReactMarkdown>{project.prd}</ReactMarkdown>
          </div>
        ) : (
          <div className="text-center py-12 text-gray-400">
            <FileText className="w-12 h-12 mx-auto mb-3 opacity-50" />
            <p>No PRD generated yet</p>
            <p className="text-sm mt-2">
              Click "Edit PRD" to add a Product Requirements Document.
            </p>
            <button
              onClick={handleStartEdit}
              className="mt-4 inline-flex items-center gap-2 px-4 py-2 bg-accent-teal text-background-deep rounded-lg font-medium hover:bg-accent-teal/90 transition-colors"
            >
              <Edit3 className="w-4 h-4" />
              Add PRD
            </button>
          </div>
        )}
      </div>

      {/* Extract Features Modal */}
      {showExtractModal && project?.prd && projectId && (
        <ExtractFeaturesModal
          prd={project.prd}
          projectId={projectId}
          onClose={() => setShowExtractModal(false)}
          onFeaturesCreated={handleFeaturesCreated}
        />
      )}
    </div>
  );
}

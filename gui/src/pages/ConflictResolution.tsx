import { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import {
  GitMerge,
  ChevronLeft,
  ChevronRight,
  Check,
  Edit3,
  AlertTriangle,
} from 'lucide-react';

interface ConflictHunk {
  id: string;
  start_line: number;
  end_line: number;
  user_content: string;
  ai_content: string;
  resolved: boolean;
  resolution: 'user' | 'ai' | 'custom' | null;
  custom_content: string | null;
}

interface Conflict {
  id: string;
  file_path: string;
  hunks: ConflictHunk[];
  created_at: string;
}

export default function ConflictResolution() {
  const { projectId, conflictId } = useParams<{ projectId: string; conflictId: string }>();
  const navigate = useNavigate();
  const [conflict, setConflict] = useState<Conflict | null>(null);
  const [conflicts, setConflicts] = useState<Conflict[]>([]);
  const [currentHunkIndex, setCurrentHunkIndex] = useState(0);
  const [editMode, setEditMode] = useState(false);
  const [customContent, setCustomContent] = useState('');
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function loadConflicts() {
      if (!projectId) return;
      try {
        const data = await invoke<Conflict[]>('get_conflicts', { projectId });
        setConflicts(data);

        if (conflictId) {
          const found = data.find(c => c.id === conflictId);
          setConflict(found || null);
        } else if (data.length > 0) {
          setConflict(data[0]);
        }
      } catch (error) {
        console.error('Failed to load conflicts:', error);
      } finally {
        setLoading(false);
      }
    }
    loadConflicts();
  }, [projectId, conflictId]);

  const currentHunk = conflict?.hunks[currentHunkIndex];
  const unresolvedCount = conflict?.hunks.filter(h => !h.resolved).length || 0;
  const totalHunks = conflict?.hunks.length || 0;

  async function resolveHunk(resolution: 'user' | 'ai' | 'custom') {
    if (!conflict || !currentHunk) return;

    const content = resolution === 'custom' ? customContent : null;

    try {
      await invoke('resolve_conflict_hunk', {
        conflictId: conflict.id,
        hunkId: currentHunk.id,
        resolution,
        customContent: content,
      });

      setConflict(prev => {
        if (!prev) return null;
        return {
          ...prev,
          hunks: prev.hunks.map(h =>
            h.id === currentHunk.id
              ? { ...h, resolved: true, resolution, custom_content: content }
              : h
          ),
        };
      });

      setEditMode(false);
      setCustomContent('');

      // Move to next unresolved hunk
      const nextUnresolved = conflict.hunks.findIndex(
        (h, i) => i > currentHunkIndex && !h.resolved
      );
      if (nextUnresolved !== -1) {
        setCurrentHunkIndex(nextUnresolved);
      }
    } catch (error) {
      console.error('Failed to resolve hunk:', error);
    }
  }

  async function applyResolutions() {
    if (!conflict) return;

    try {
      await invoke('apply_conflict_resolutions', { conflictId: conflict.id });
      navigate(`/projects/${projectId}`);
    } catch (error) {
      console.error('Failed to apply resolutions:', error);
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="animate-pulse text-accent-teal">Loading conflicts...</div>
      </div>
    );
  }

  if (!conflict) {
    return (
      <div className="flex flex-col items-center justify-center h-full gap-4">
        <GitMerge className="w-16 h-16 text-accent-teal opacity-50" />
        <p className="text-gray-400">No conflicts to resolve</p>
        <button
          onClick={() => navigate(`/projects/${projectId}`)}
          className="px-4 py-2 bg-accent-teal text-background-deep rounded-lg font-medium hover:bg-accent-teal/90 transition-colors"
        >
          Back to Project
        </button>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="p-4 border-b border-background-surface bg-background-mid">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <GitMerge className="w-6 h-6 text-accent-magenta" />
            <div>
              <h1 className="text-lg font-bold">Conflict Resolution</h1>
              <p className="text-sm text-gray-400 font-mono">{conflict.file_path}</p>
            </div>
          </div>
          <div className="flex items-center gap-4">
            <div className="text-sm">
              <span className="text-accent-amber">{unresolvedCount}</span>
              <span className="text-gray-400"> of {totalHunks} unresolved</span>
            </div>
            <button
              onClick={applyResolutions}
              disabled={unresolvedCount > 0}
              className={`px-4 py-2 rounded-lg font-medium transition-colors ${
                unresolvedCount > 0
                  ? 'bg-background-surface text-gray-500 cursor-not-allowed'
                  : 'bg-accent-teal text-background-deep hover:bg-accent-teal/90'
              }`}
            >
              Apply All
            </button>
          </div>
        </div>

        {/* Hunk Navigation */}
        <div className="flex items-center gap-2 mt-4">
          <button
            onClick={() => setCurrentHunkIndex(i => Math.max(0, i - 1))}
            disabled={currentHunkIndex === 0}
            className="p-1 rounded hover:bg-background-surface disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <ChevronLeft className="w-5 h-5" />
          </button>
          <div className="flex gap-1">
            {conflict.hunks.map((hunk, i) => (
              <button
                key={hunk.id}
                onClick={() => setCurrentHunkIndex(i)}
                className={`w-8 h-8 rounded text-sm font-medium transition-colors ${
                  i === currentHunkIndex
                    ? 'bg-accent-teal text-background-deep'
                    : hunk.resolved
                    ? 'bg-accent-teal/20 text-accent-teal'
                    : 'bg-background-surface text-gray-400 hover:bg-background-surface/80'
                }`}
              >
                {i + 1}
              </button>
            ))}
          </div>
          <button
            onClick={() => setCurrentHunkIndex(i => Math.min(totalHunks - 1, i + 1))}
            disabled={currentHunkIndex === totalHunks - 1}
            className="p-1 rounded hover:bg-background-surface disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <ChevronRight className="w-5 h-5" />
          </button>
        </div>
      </div>

      {/* Diff View */}
      {currentHunk && (
        <div className="flex-1 flex overflow-hidden">
          {editMode ? (
            /* Custom Edit Mode */
            <div className="flex-1 flex flex-col p-4">
              <div className="flex items-center justify-between mb-2">
                <span className="text-sm text-gray-400">Custom Merge</span>
                <div className="flex gap-2">
                  <button
                    onClick={() => {
                      setEditMode(false);
                      setCustomContent('');
                    }}
                    className="px-3 py-1 text-sm rounded bg-background-surface hover:bg-background-surface/80 transition-colors"
                  >
                    Cancel
                  </button>
                  <button
                    onClick={() => resolveHunk('custom')}
                    className="px-3 py-1 text-sm rounded bg-accent-teal text-background-deep hover:bg-accent-teal/90 transition-colors"
                  >
                    Apply Custom
                  </button>
                </div>
              </div>
              <textarea
                value={customContent}
                onChange={e => setCustomContent(e.target.value)}
                className="flex-1 bg-background-deep border border-background-surface rounded-lg p-4 font-mono text-sm resize-none focus:outline-none focus:border-accent-teal"
                placeholder="Enter your custom merged code..."
              />
            </div>
          ) : (
            /* Side-by-Side Diff */
            <>
              {/* User Version */}
              <div className="flex-1 flex flex-col border-r border-background-surface">
                <div className="p-3 bg-background-surface flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium text-accent-amber">Your Changes</span>
                    <span className="text-xs text-gray-500">
                      Lines {currentHunk.start_line}-{currentHunk.end_line}
                    </span>
                  </div>
                  <button
                    onClick={() => resolveHunk('user')}
                    disabled={currentHunk.resolved}
                    className={`flex items-center gap-1 px-3 py-1 text-sm rounded transition-colors ${
                      currentHunk.resolved && currentHunk.resolution === 'user'
                        ? 'bg-accent-teal/20 text-accent-teal'
                        : currentHunk.resolved
                        ? 'bg-background-mid text-gray-500 cursor-not-allowed'
                        : 'bg-accent-amber/20 text-accent-amber hover:bg-accent-amber/30'
                    }`}
                  >
                    <Check className="w-4 h-4" />
                    Accept
                  </button>
                </div>
                <div className="flex-1 overflow-auto">
                  <pre className="p-4 text-sm font-mono whitespace-pre-wrap text-gray-300">
                    {currentHunk.user_content}
                  </pre>
                </div>
              </div>

              {/* AI Version */}
              <div className="flex-1 flex flex-col">
                <div className="p-3 bg-background-surface flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium text-accent-magenta">AI Generated</span>
                    <span className="text-xs text-gray-500">
                      Lines {currentHunk.start_line}-{currentHunk.end_line}
                    </span>
                  </div>
                  <button
                    onClick={() => resolveHunk('ai')}
                    disabled={currentHunk.resolved}
                    className={`flex items-center gap-1 px-3 py-1 text-sm rounded transition-colors ${
                      currentHunk.resolved && currentHunk.resolution === 'ai'
                        ? 'bg-accent-teal/20 text-accent-teal'
                        : currentHunk.resolved
                        ? 'bg-background-mid text-gray-500 cursor-not-allowed'
                        : 'bg-accent-magenta/20 text-accent-magenta hover:bg-accent-magenta/30'
                    }`}
                  >
                    <Check className="w-4 h-4" />
                    Accept
                  </button>
                </div>
                <div className="flex-1 overflow-auto">
                  <pre className="p-4 text-sm font-mono whitespace-pre-wrap text-gray-300">
                    {currentHunk.ai_content}
                  </pre>
                </div>
              </div>
            </>
          )}
        </div>
      )}

      {/* Action Bar */}
      {currentHunk && !editMode && (
        <div className="p-4 border-t border-background-surface bg-background-mid flex items-center justify-between">
          <div className="flex items-center gap-2 text-sm text-gray-400">
            {currentHunk.resolved ? (
              <>
                <Check className="w-4 h-4 text-accent-teal" />
                Resolved with {currentHunk.resolution} version
              </>
            ) : (
              <>
                <AlertTriangle className="w-4 h-4 text-accent-amber" />
                Unresolved conflict
              </>
            )}
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={() => {
                setEditMode(true);
                setCustomContent(currentHunk.user_content);
              }}
              disabled={currentHunk.resolved}
              className={`flex items-center gap-2 px-4 py-2 rounded-lg transition-colors ${
                currentHunk.resolved
                  ? 'bg-background-surface text-gray-500 cursor-not-allowed'
                  : 'bg-background-surface hover:bg-background-surface/80'
              }`}
            >
              <Edit3 className="w-4 h-4" />
              Custom Merge
            </button>
            <button
              onClick={() => resolveHunk('user')}
              disabled={currentHunk.resolved}
              className={`flex items-center gap-2 px-4 py-2 rounded-lg transition-colors ${
                currentHunk.resolved
                  ? 'bg-background-surface text-gray-500 cursor-not-allowed'
                  : 'bg-accent-amber/20 text-accent-amber hover:bg-accent-amber/30'
              }`}
            >
              Accept Yours
            </button>
            <button
              onClick={() => resolveHunk('ai')}
              disabled={currentHunk.resolved}
              className={`flex items-center gap-2 px-4 py-2 rounded-lg transition-colors ${
                currentHunk.resolved
                  ? 'bg-background-surface text-gray-500 cursor-not-allowed'
                  : 'bg-accent-magenta/20 text-accent-magenta hover:bg-accent-magenta/30'
              }`}
            >
              Accept AI
            </button>
          </div>
        </div>
      )}

      {/* Conflict List Sidebar (if multiple conflicts) */}
      {conflicts.length > 1 && (
        <div className="absolute top-20 right-4 w-64 bg-background-mid border border-background-surface rounded-lg shadow-lg">
          <div className="p-3 border-b border-background-surface">
            <h3 className="text-sm font-medium">All Conflicts ({conflicts.length})</h3>
          </div>
          <div className="max-h-48 overflow-y-auto">
            {conflicts.map(c => (
              <button
                key={c.id}
                onClick={() => {
                  setConflict(c);
                  setCurrentHunkIndex(0);
                }}
                className={`w-full text-left p-3 hover:bg-background-surface transition-colors ${
                  c.id === conflict?.id ? 'bg-background-surface' : ''
                }`}
              >
                <p className="text-sm font-mono truncate">{c.file_path}</p>
                <p className="text-xs text-gray-400">
                  {c.hunks.filter(h => !h.resolved).length} unresolved
                </p>
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

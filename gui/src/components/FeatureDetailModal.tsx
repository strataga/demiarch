import { useState, useEffect, useCallback } from 'react';
import { X, Edit3, Save, Trash2, Calendar, Tag, AlertTriangle, Package, Settings, FileCode, Terminal, RefreshCw, FolderOpen, History, Copy, Check, GitBranch, CheckCircle2, Clock, XCircle, Loader2 } from 'lucide-react';
import { invoke, Feature } from '../lib/api';
import { useModalShortcuts } from '../hooks/useKeyboardShortcuts';
import { openFolder, copyToClipboard, getGitHistory, GitCommit } from '../lib/shell';

interface FeatureDetailModalProps {
  feature: Feature;
  onClose: () => void;
  onUpdated: (feature: Feature) => void;
  onDeleted: (featureId: string) => void;
  onRetry?: (feature: Feature) => void;
  projectPath?: string;
  projectName?: string;
}

const PRIORITY_OPTIONS = [
  { value: 0, label: 'P0 - Critical', color: 'text-red-400' },
  { value: 1, label: 'P1 - High', color: 'text-accent-magenta' },
  { value: 2, label: 'P2 - Medium', color: 'text-accent-amber' },
  { value: 3, label: 'P3 - Low', color: 'text-accent-teal' },
  { value: 4, label: 'P4 - Backlog', color: 'text-gray-400' },
];

const STATUS_OPTIONS = [
  { value: 'pending', label: 'To Do', color: 'bg-gray-500' },
  { value: 'in_progress', label: 'In Progress', color: 'bg-accent-amber' },
  { value: 'complete', label: 'Complete', color: 'bg-accent-teal' },
  { value: 'blocked', label: 'Blocked', color: 'bg-accent-magenta' },
];

export default function FeatureDetailModal({
  feature,
  onClose,
  onUpdated,
  onDeleted,
  onRetry,
  projectPath,
  projectName,
}: FeatureDetailModalProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showCommitHistory, setShowCommitHistory] = useState(false);
  const [commitHistory, setCommitHistory] = useState<GitCommit[]>([]);
  const [loadingHistory, setLoadingHistory] = useState(false);
  const [copiedPath, setCopiedPath] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<'overview' | 'code' | 'setup'>('overview');

  // Edit form state
  const [name, setName] = useState(feature.name);
  const [description, setDescription] = useState(feature.description || '');
  const [priority, setPriority] = useState(feature.priority);
  const [status, setStatus] = useState(feature.status);
  const [dueDate, setDueDate] = useState(feature.due_date || '');
  const [tagInput, setTagInput] = useState('');
  const [tags, setTags] = useState<string[]>(feature.tags || []);

  // Close handler
  const handleClose = useCallback(() => {
    onClose();
  }, [onClose]);

  // Keyboard shortcuts
  useModalShortcuts(handleClose);

  // Reset form when feature changes
  useEffect(() => {
    setName(feature.name);
    setDescription(feature.description || '');
    setPriority(feature.priority);
    setStatus(feature.status);
    setDueDate(feature.due_date || '');
    setTags(feature.tags || []);
  }, [feature]);

  function handleAddTag() {
    const tag = tagInput.trim().toLowerCase();
    if (tag && !tags.includes(tag)) {
      setTags([...tags, tag]);
      setTagInput('');
    }
  }

  function handleRemoveTag(tag: string) {
    setTags(tags.filter((t) => t !== tag));
  }

  function handleTagKeyDown(e: React.KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleAddTag();
    }
  }

  function handleCancelEdit() {
    setName(feature.name);
    setDescription(feature.description || '');
    setPriority(feature.priority);
    setStatus(feature.status);
    setDueDate(feature.due_date || '');
    setTags(feature.tags || []);
    setIsEditing(false);
    setError(null);
  }

  async function handleSave() {
    if (!name.trim()) {
      setError('Feature name is required');
      return;
    }

    setSaving(true);
    setError(null);

    try {
      const updated = await invoke<Feature>('update_feature', {
        id: feature.id,
        name: name.trim(),
        description: description.trim() || null,
        priority,
        status,
        due_date: dueDate || null,
        tags,
      });
      onUpdated(updated);
      setIsEditing(false);
    } catch (err) {
      console.error('Failed to update feature:', err);
      setError(String(err));
    } finally {
      setSaving(false);
    }
  }

  async function handleDelete() {
    setDeleting(true);
    try {
      await invoke('delete_feature', { id: feature.id });
      onDeleted(feature.id);
    } catch (err) {
      console.error('Failed to delete feature:', err);
      setError(String(err));
      setDeleting(false);
    }
  }

  const priorityInfo = PRIORITY_OPTIONS.find((p) => p.value === feature.priority) || PRIORITY_OPTIONS[2];
  const statusInfo = STATUS_OPTIONS.find((s) => s.value === feature.status) || STATUS_OPTIONS[0];

  const isOverdue = feature.due_date && new Date(feature.due_date) < new Date() && feature.status !== 'complete';

  async function handleOpenFolder() {
    if (!projectPath || !feature.generated_code?.[0]) return;

    // Get the folder from the first generated file
    const firstFile = feature.generated_code[0].path;
    const folderPath = firstFile.substring(0, firstFile.lastIndexOf('/'));
    const fullPath = `${projectPath}/${folderPath}`;

    await openFolder(fullPath);
  }

  async function handleCopyPath(path: string) {
    const result = await copyToClipboard(projectPath ? `${projectPath}/${path}` : path);
    if (result.success) {
      setCopiedPath(path);
      setTimeout(() => setCopiedPath(null), 2000);
    }
  }

  async function handleViewHistory() {
    if (!projectPath || !feature.generated_code?.length) return;

    setLoadingHistory(true);
    setShowCommitHistory(true);

    const files = feature.generated_code.map(f => f.path);
    const result = await getGitHistory(projectPath, files);

    if (result.success && result.commits) {
      setCommitHistory(result.commits);
    }
    setLoadingHistory(false);
  }

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
      <div className="bg-background-mid border border-background-surface rounded-lg w-full max-w-lg max-h-[90vh] flex flex-col">
        {/* Header */}
        <div className="flex justify-between items-center p-4 border-b border-background-surface">
          <div className="flex items-center gap-3">
            <div className={`w-2 h-8 rounded-full ${statusInfo.color}`} />
            <div>
              <h2 className="font-semibold">Feature Details</h2>
              <span className={`text-xs ${priorityInfo.color}`}>{priorityInfo.label}</span>
            </div>
          </div>
          <div className="flex items-center gap-2">
            {!isEditing && (
              <button
                onClick={() => setIsEditing(true)}
                className="p-2 text-gray-400 hover:text-white rounded-lg hover:bg-background-surface transition-colors"
                title="Edit feature"
              >
                <Edit3 className="w-4 h-4" />
              </button>
            )}
            <button
              onClick={handleClose}
              className="text-gray-400 hover:text-white transition-colors"
              title="Close"
            >
              <X className="w-5 h-5" />
            </button>
          </div>
        </div>

        {/* Content */}
        <div className="p-4 space-y-4 overflow-y-auto">
          {error && (
            <div className="bg-red-500/10 border border-red-500/30 text-red-400 px-3 py-2 rounded-lg text-sm">
              {error}
            </div>
          )}

          {isEditing ? (
            // Edit Mode
            <>
              <div>
                <label className="block text-sm text-gray-400 mb-1">Feature Name *</label>
                <input
                  type="text"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  className="w-full bg-background-surface border border-background-surface rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-accent-teal"
                />
              </div>

              <div>
                <label className="block text-sm text-gray-400 mb-1">Description</label>
                <textarea
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  rows={3}
                  className="w-full bg-background-surface border border-background-surface rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-accent-teal resize-none"
                />
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Priority</label>
                  <select
                    value={priority}
                    onChange={(e) => setPriority(Number(e.target.value))}
                    className="w-full bg-background-surface border border-background-surface rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-accent-teal"
                  >
                    {PRIORITY_OPTIONS.map((opt) => (
                      <option key={opt.value} value={opt.value} className="bg-background-surface text-white">
                        {opt.label}
                      </option>
                    ))}
                  </select>
                </div>
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Status</label>
                  <select
                    value={status}
                    onChange={(e) => setStatus(e.target.value)}
                    className="w-full bg-background-surface border border-background-surface rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-accent-teal"
                  >
                    {STATUS_OPTIONS.map((opt) => (
                      <option key={opt.value} value={opt.value} className="bg-background-surface text-white">
                        {opt.label}
                      </option>
                    ))}
                  </select>
                </div>
              </div>

              <div>
                <label className="block text-sm text-gray-400 mb-1 flex items-center gap-1">
                  <Calendar className="w-4 h-4" />
                  Due Date
                </label>
                <input
                  type="date"
                  value={dueDate}
                  onChange={(e) => setDueDate(e.target.value)}
                  className="w-full bg-background-surface border border-background-surface rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-accent-teal"
                />
              </div>

              <div>
                <label className="block text-sm text-gray-400 mb-1 flex items-center gap-1">
                  <Tag className="w-4 h-4" />
                  Tags
                </label>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={tagInput}
                    onChange={(e) => setTagInput(e.target.value)}
                    onKeyDown={handleTagKeyDown}
                    placeholder="Add a tag..."
                    className="flex-1 bg-background-surface border border-background-surface rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-accent-teal"
                  />
                  <button
                    type="button"
                    onClick={handleAddTag}
                    className="px-3 py-2 bg-background-surface text-gray-400 rounded-lg hover:text-white transition-colors"
                  >
                    Add
                  </button>
                </div>
                {tags.length > 0 && (
                  <div className="flex flex-wrap gap-2 mt-2">
                    {tags.map((tag) => (
                      <span
                        key={tag}
                        className="inline-flex items-center gap-1 px-2 py-1 bg-accent-teal/10 text-accent-teal text-xs rounded"
                      >
                        {tag}
                        <button
                          type="button"
                          onClick={() => handleRemoveTag(tag)}
                          className="hover:text-white"
                        >
                          <X className="w-3 h-3" />
                        </button>
                      </span>
                    ))}
                  </div>
                )}
              </div>

              <div className="flex gap-3 pt-2">
                <button
                  onClick={handleCancelEdit}
                  className="flex-1 px-4 py-2 bg-background-surface text-gray-300 rounded-lg font-medium hover:text-white transition-colors"
                >
                  Cancel
                </button>
                <button
                  onClick={handleSave}
                  disabled={saving || !name.trim()}
                  className="flex-1 px-4 py-2 bg-accent-teal text-background-deep rounded-lg font-medium hover:bg-accent-teal/90 transition-colors disabled:opacity-50 flex items-center justify-center gap-2"
                >
                  <Save className="w-4 h-4" />
                  {saving ? 'Saving...' : 'Save Changes'}
                </button>
              </div>
            </>
          ) : (
            // View Mode
            <>
              {/* Feature Header with Status */}
              <div className="flex items-start justify-between">
                <div className="flex-1">
                  <h3 className="text-xl font-semibold">{feature.name}</h3>
                  <div className="flex items-center gap-2 mt-2">
                    <span
                      className={`inline-flex items-center gap-1 px-2 py-1 rounded text-xs font-medium ${statusInfo.color} text-white`}
                    >
                      {feature.status === 'complete' && <CheckCircle2 className="w-3 h-3" />}
                      {feature.status === 'in_progress' && <Loader2 className="w-3 h-3 animate-spin" />}
                      {feature.status === 'blocked' && <XCircle className="w-3 h-3" />}
                      {feature.status === 'pending' && <Clock className="w-3 h-3" />}
                      {statusInfo.label}
                    </span>
                    <span className={`text-xs ${priorityInfo.color}`}>{priorityInfo.label}</span>
                  </div>
                </div>
              </div>

              {/* Build Summary Card for Completed/Built Features */}
              {(feature.status === 'complete' || feature.generated_code?.length) && (
                <div className="bg-gradient-to-r from-accent-teal/10 to-accent-teal/5 border border-accent-teal/20 rounded-lg p-3">
                  <h4 className="text-sm font-medium text-accent-teal mb-2 flex items-center gap-2">
                    <CheckCircle2 className="w-4 h-4" />
                    Build Summary
                  </h4>
                  <div className="grid grid-cols-3 gap-3 text-center">
                    <div className="bg-background-surface/50 rounded p-2">
                      <div className="text-lg font-bold text-white">
                        {feature.generated_code?.length || 0}
                      </div>
                      <div className="text-xs text-gray-400">Files</div>
                    </div>
                    <div className="bg-background-surface/50 rounded p-2">
                      <div className="text-lg font-bold text-white">
                        {feature.dependencies?.length || 0}
                      </div>
                      <div className="text-xs text-gray-400">Dependencies</div>
                    </div>
                    <div className="bg-background-surface/50 rounded p-2">
                      <div className="text-lg font-bold text-white">
                        {feature.setup_requirements?.length || 0}
                      </div>
                      <div className="text-xs text-gray-400">Setup Steps</div>
                    </div>
                  </div>
                  {!feature.generated_code?.length && feature.status === 'complete' && (
                    <div className="mt-2 text-xs text-gray-400 text-center">
                      No code generated yet. Use Auto Build to generate implementation.
                    </div>
                  )}
                </div>
              )}

              {/* Git Branch Info */}
              <div className="flex items-center gap-2 text-sm bg-background-surface rounded-lg px-3 py-2">
                <GitBranch className="w-4 h-4 text-accent-amber" />
                <span className="text-gray-400">Branch:</span>
                <span className="font-mono text-white">main</span>
                <span className="text-gray-500 ml-auto text-xs">
                  {feature.updated_at !== feature.created_at
                    ? `Updated ${new Date(feature.updated_at).toLocaleDateString()}`
                    : `Created ${new Date(feature.created_at).toLocaleDateString()}`
                  }
                </span>
              </div>

              {/* Tabs for Details */}
              {/* Tabs for Details */}
              {(
                <>
                  <div className="flex border-b border-background-surface">
                    <button
                      onClick={() => setActiveTab('overview')}
                      className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors ${
                        activeTab === 'overview'
                          ? 'border-accent-teal text-accent-teal'
                          : 'border-transparent text-gray-400 hover:text-white'
                      }`}
                    >
                      Overview
                    </button>
                    <button
                      onClick={() => setActiveTab('code')}
                      className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors flex items-center gap-1 ${
                        activeTab === 'code'
                          ? 'border-accent-teal text-accent-teal'
                          : 'border-transparent text-gray-400 hover:text-white'
                      }`}
                    >
                      <FileCode className="w-3 h-3" />
                      Code ({feature.generated_code?.length || 0})
                    </button>
                    <button
                      onClick={() => setActiveTab('setup')}
                      className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors flex items-center gap-1 ${
                        activeTab === 'setup'
                          ? 'border-accent-teal text-accent-teal'
                          : 'border-transparent text-gray-400 hover:text-white'
                      }`}
                    >
                      <Settings className="w-3 h-3" />
                      Setup ({(feature.dependencies?.length || 0) + (feature.setup_requirements?.length || 0)})
                    </button>
                  </div>

                  {/* Tab Content */}
                  <div className="min-h-[100px]">
                    {activeTab === 'overview' && (
                      <div className="space-y-4">
                        {feature.description && (
                          <div>
                            <h4 className="text-sm text-gray-400 mb-1">Description</h4>
                            <p className="text-sm">{feature.description}</p>
                          </div>
                        )}

                        <div className="grid grid-cols-2 gap-4">
                          <div>
                            <h4 className="text-sm text-gray-400 mb-1">Priority</h4>
                            <span className={`text-sm font-medium ${priorityInfo.color}`}>
                              {priorityInfo.label}
                            </span>
                          </div>
                          <div>
                            <h4 className="text-sm text-gray-400 mb-1 flex items-center gap-1">
                              <Calendar className="w-4 h-4" />
                              Due Date
                            </h4>
                            {feature.due_date ? (
                              <span className={`text-sm ${isOverdue ? 'text-red-400 font-medium' : ''}`}>
                                {isOverdue && <AlertTriangle className="w-4 h-4 inline mr-1" />}
                                {new Date(feature.due_date).toLocaleDateString()}
                              </span>
                            ) : (
                              <span className="text-sm text-gray-500">Not set</span>
                            )}
                          </div>
                        </div>

                        {feature.tags && feature.tags.length > 0 && (
                          <div>
                            <h4 className="text-sm text-gray-400 mb-1 flex items-center gap-1">
                              <Tag className="w-4 h-4" />
                              Tags
                            </h4>
                            <div className="flex flex-wrap gap-2">
                              {feature.tags.map((tag) => (
                                <span
                                  key={tag}
                                  className="px-2 py-1 bg-accent-teal/10 text-accent-teal text-xs rounded"
                                >
                                  {tag}
                                </span>
                              ))}
                            </div>
                          </div>
                        )}
                      </div>
                    )}

                    {activeTab === 'code' && (
                      <div className="space-y-3">
                        {feature.generated_code && feature.generated_code.length > 0 ? (
                          <>
                            <div className="flex items-center justify-end gap-2">
                              <button
                                onClick={handleOpenFolder}
                                className="flex items-center gap-1 px-2 py-1 text-xs bg-background-surface text-gray-400 rounded hover:text-white transition-colors"
                                title="Open folder in file explorer"
                              >
                                <FolderOpen className="w-3 h-3" />
                                Open Folder
                              </button>
                              <button
                                onClick={handleViewHistory}
                                className="flex items-center gap-1 px-2 py-1 text-xs bg-background-surface text-gray-400 rounded hover:text-white transition-colors"
                                title="View git commit history"
                              >
                                <History className="w-3 h-3" />
                                History
                              </button>
                            </div>

                            <div className="space-y-1">
                              {feature.generated_code.map((file, idx) => (
                                <div
                                  key={idx}
                                  className="flex items-center gap-2 p-2 bg-background-surface rounded text-sm font-mono group"
                                >
                                  <FileCode className="w-3 h-3 text-gray-500" />
                                  <span className="text-gray-300 flex-1 truncate">{file.path}</span>
                                  <button
                                    onClick={() => handleCopyPath(file.path)}
                                    className="opacity-0 group-hover:opacity-100 text-gray-500 hover:text-white transition-all"
                                    title="Copy path"
                                  >
                                    {copiedPath === file.path ? (
                                      <Check className="w-3 h-3 text-accent-teal" />
                                    ) : (
                                      <Copy className="w-3 h-3" />
                                    )}
                                  </button>
                                  <span className="text-xs text-gray-500">{file.language}</span>
                                </div>
                              ))}
                            </div>

                            {showCommitHistory && (
                              <div className="p-3 bg-background-deep rounded-lg border border-background-surface">
                                <div className="flex items-center justify-between mb-2">
                                  <h5 className="text-sm font-medium flex items-center gap-1">
                                    <History className="w-3 h-3" />
                                    Commit History
                                  </h5>
                                  <button
                                    onClick={() => setShowCommitHistory(false)}
                                    className="text-gray-500 hover:text-white"
                                  >
                                    <X className="w-4 h-4" />
                                  </button>
                                </div>
                                {loadingHistory ? (
                                  <div className="text-sm text-gray-500 py-2">Loading...</div>
                                ) : commitHistory.length > 0 ? (
                                  <div className="space-y-2">
                                    {commitHistory.map((commit, idx) => (
                                      <div key={idx} className="text-xs border-l-2 border-accent-teal pl-2">
                                        <div className="flex items-center gap-2">
                                          <span className="font-mono text-accent-teal">{commit.shortHash}</span>
                                          <span className="text-gray-400">{commit.author}</span>
                                        </div>
                                        <p className="text-gray-300 mt-0.5">{commit.message}</p>
                                        <span className="text-gray-500">{new Date(commit.date).toLocaleString()}</span>
                                      </div>
                                    ))}
                                  </div>
                                ) : (
                                  <div className="text-sm text-gray-500 py-2">
                                    No commits found for these files
                                  </div>
                                )}
                              </div>
                            )}
                          </>
                        ) : (
                          <div className="text-center py-8 text-gray-500">
                            <FileCode className="w-8 h-8 mx-auto mb-2 opacity-50" />
                            <p className="text-sm">No code generated yet</p>
                            <p className="text-xs mt-1">Enable Auto Build to generate implementation</p>
                          </div>
                        )}
                      </div>
                    )}

                    {activeTab === 'setup' && (
                      <div className="space-y-4">
                        {/* Dependencies */}
                        {feature.dependencies && feature.dependencies.length > 0 && (
                          <div>
                            <h4 className="text-sm text-gray-400 mb-2 flex items-center gap-1">
                              <Package className="w-4 h-4" />
                              Dependencies ({feature.dependencies.length})
                            </h4>
                            <div className="space-y-1">
                              {feature.dependencies.map((dep, idx) => (
                                <div
                                  key={idx}
                                  className="flex items-center gap-2 p-2 bg-background-surface rounded text-sm"
                                >
                                  <span className="font-mono text-accent-amber">
                                    {dep.name}
                                    {dep.version && <span className="text-gray-500">@{dep.version}</span>}
                                  </span>
                                  {dep.dev && (
                                    <span className="px-1 py-0.5 bg-gray-700 text-gray-400 text-xs rounded">
                                      dev
                                    </span>
                                  )}
                                  <span className="text-gray-400 text-xs ml-auto truncate max-w-[150px]">{dep.reason}</span>
                                </div>
                              ))}
                            </div>
                          </div>
                        )}

                        {/* Setup Requirements */}
                        {feature.setup_requirements && feature.setup_requirements.length > 0 && (
                          <div>
                            <h4 className="text-sm text-gray-400 mb-2 flex items-center gap-1">
                              <Settings className="w-4 h-4" />
                              Setup Steps ({feature.setup_requirements.length})
                            </h4>
                            <div className="space-y-2">
                              {feature.setup_requirements.map((setup, idx) => (
                                <div
                                  key={idx}
                                  className="p-2 bg-background-surface rounded text-sm"
                                >
                                  <div className="flex items-center gap-2 mb-1">
                                    <span className="font-medium">{idx + 1}. {setup.step}</span>
                                    <span className={`px-1 py-0.5 text-xs rounded ${
                                      setup.type === 'install' ? 'bg-accent-teal/20 text-accent-teal' :
                                      setup.type === 'config' ? 'bg-accent-amber/20 text-accent-amber' :
                                      setup.type === 'env' ? 'bg-accent-magenta/20 text-accent-magenta' :
                                      'bg-gray-700 text-gray-400'
                                    }`}>
                                      {setup.type}
                                    </span>
                                  </div>
                                  <p className="text-gray-400 text-xs">{setup.description}</p>
                                  {setup.command && (
                                    <div className="mt-1 flex items-center gap-1">
                                      <Terminal className="w-3 h-3 text-gray-500" />
                                      <code className="font-mono text-xs text-gray-400 bg-background-deep px-2 py-1 rounded">
                                        {setup.command}
                                      </code>
                                    </div>
                                  )}
                                </div>
                              ))}
                            </div>
                          </div>
                        )}

                        {!feature.dependencies?.length && !feature.setup_requirements?.length && (
                          <div className="text-center py-8 text-gray-500">
                            <Package className="w-8 h-8 mx-auto mb-2 opacity-50" />
                            <p className="text-sm">No dependencies or setup required</p>
                          </div>
                        )}
                      </div>
                    )}
                  </div>
                </>
              )}

              {/* Retry Button for Blocked Features */}
              {feature.status === 'blocked' && onRetry && (
                <div className="pt-4 border-t border-background-surface">
                  <div className="flex items-center gap-2 mb-2 text-accent-magenta">
                    <AlertTriangle className="w-4 h-4" />
                    <span className="text-sm font-medium">This feature is blocked</span>
                  </div>
                  <p className="text-xs text-gray-400 mb-3">
                    The build failed. Check that your API key is configured in Settings, then retry.
                  </p>
                  <button
                    onClick={() => onRetry(feature)}
                    className="flex items-center gap-2 px-4 py-2 bg-accent-amber text-background-deep rounded-lg font-medium hover:bg-accent-amber/90 transition-colors"
                  >
                    <RefreshCw className="w-4 h-4" />
                    Retry Build
                  </button>
                </div>
              )}

              {/* Delete Button */}
              <div className="pt-4 border-t border-background-surface">
                {!showDeleteConfirm ? (
                  <button
                    onClick={() => setShowDeleteConfirm(true)}
                    className="flex items-center gap-2 px-4 py-2 text-red-400 hover:bg-red-500/10 rounded-lg transition-colors"
                  >
                    <Trash2 className="w-4 h-4" />
                    Delete Feature
                  </button>
                ) : (
                  <div className="space-y-3">
                    <div className="flex items-center gap-2 text-red-400">
                      <AlertTriangle className="w-5 h-5" />
                      <span className="text-sm font-medium">Are you sure you want to delete this feature?</span>
                    </div>
                    <div className="flex gap-3">
                      <button
                        onClick={() => setShowDeleteConfirm(false)}
                        className="flex-1 px-4 py-2 bg-background-surface text-gray-300 rounded-lg font-medium hover:text-white transition-colors"
                      >
                        Cancel
                      </button>
                      <button
                        onClick={handleDelete}
                        disabled={deleting}
                        className="flex-1 px-4 py-2 bg-red-500 text-white rounded-lg font-medium hover:bg-red-600 transition-colors disabled:opacity-50"
                      >
                        {deleting ? 'Deleting...' : 'Yes, Delete'}
                      </button>
                    </div>
                  </div>
                )}
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}

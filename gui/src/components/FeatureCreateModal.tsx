import { useState } from 'react';
import { X, Plus, Calendar, Tag } from 'lucide-react';
import { invoke, Feature } from '../lib/api';
import { useModalShortcuts } from '../hooks/useKeyboardShortcuts';

interface FeatureCreateModalProps {
  projectId: string;
  onClose: () => void;
  onCreated: (feature: Feature) => void;
}

const PRIORITY_OPTIONS = [
  { value: 0, label: 'P0 - Critical', color: 'text-red-400' },
  { value: 1, label: 'P1 - High', color: 'text-accent-magenta' },
  { value: 2, label: 'P2 - Medium', color: 'text-accent-amber' },
  { value: 3, label: 'P3 - Low', color: 'text-accent-teal' },
  { value: 4, label: 'P4 - Backlog', color: 'text-gray-400' },
];

const STATUS_OPTIONS = [
  { value: 'pending', label: 'To Do' },
  { value: 'in_progress', label: 'In Progress' },
  { value: 'complete', label: 'Complete' },
  { value: 'blocked', label: 'Blocked' },
];

export default function FeatureCreateModal({ projectId, onClose, onCreated }: FeatureCreateModalProps) {
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [priority, setPriority] = useState(2);
  const [status, setStatus] = useState('pending');
  const [dueDate, setDueDate] = useState('');
  const [tagInput, setTagInput] = useState('');
  const [tags, setTags] = useState<string[]>([]);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Keyboard shortcuts
  useModalShortcuts(onClose);

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

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!name.trim()) {
      setError('Feature name is required');
      return;
    }

    setSaving(true);
    setError(null);

    try {
      const feature = await invoke<Feature>('create_feature', {
        name: name.trim(),
        description: description.trim() || null,
        priority,
        status,
        project_id: projectId,
        due_date: dueDate || null,
        tags,
      });
      onCreated(feature);
    } catch (err) {
      console.error('Failed to create feature:', err);
      setError(String(err));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
      <div className="bg-background-mid border border-background-surface rounded-lg w-full max-w-lg">
        {/* Header */}
        <div className="flex justify-between items-center p-4 border-b border-background-surface">
          <h2 className="text-lg font-semibold flex items-center gap-2">
            <Plus className="w-5 h-5 text-accent-teal" />
            New Feature
          </h2>
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-white transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Form */}
        <form onSubmit={handleSubmit} className="p-4 space-y-4">
          {error && (
            <div className="bg-red-500/10 border border-red-500/30 text-red-400 px-3 py-2 rounded-lg text-sm">
              {error}
            </div>
          )}

          {/* Name */}
          <div>
            <label className="block text-sm text-gray-400 mb-1">Feature Name *</label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g., User authentication"
              className="w-full bg-background-surface border border-background-surface rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-accent-teal"
              autoFocus
            />
          </div>

          {/* Description */}
          <div>
            <label className="block text-sm text-gray-400 mb-1">Description</label>
            <textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Describe what this feature does..."
              rows={3}
              className="w-full bg-background-surface border border-background-surface rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-accent-teal resize-none"
            />
          </div>

          {/* Priority & Status */}
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

          {/* Due Date */}
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

          {/* Tags */}
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

          {/* Actions */}
          <div className="flex gap-3 pt-2">
            <button
              type="button"
              onClick={onClose}
              className="flex-1 px-4 py-2 bg-background-surface text-gray-300 rounded-lg font-medium hover:text-white transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={saving || !name.trim()}
              className="flex-1 px-4 py-2 bg-accent-teal text-background-deep rounded-lg font-medium hover:bg-accent-teal/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {saving ? 'Creating...' : 'Create Feature'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

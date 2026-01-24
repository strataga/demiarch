import { useState, useEffect } from 'react';
import { X, Sparkles, Check, Loader2, AlertTriangle } from 'lucide-react';
import { extractFeaturesFromPRD, extractFeaturesFromPRDLocal, ExtractedFeature } from '../lib/ai';
import { invoke, Feature } from '../lib/api';

interface ExtractFeaturesModalProps {
  prd: string;
  projectId: string;
  onClose: () => void;
  onFeaturesCreated: (features: Feature[]) => void;
}

const PRIORITY_LABELS: Record<number, { label: string; color: string }> = {
  0: { label: 'P0', color: 'text-red-400 bg-red-500/10' },
  1: { label: 'P1', color: 'text-accent-magenta bg-accent-magenta/10' },
  2: { label: 'P2', color: 'text-accent-amber bg-accent-amber/10' },
  3: { label: 'P3', color: 'text-accent-teal bg-accent-teal/10' },
  4: { label: 'P4', color: 'text-gray-400 bg-gray-500/10' },
};

export default function ExtractFeaturesModal({
  prd,
  projectId,
  onClose,
  onFeaturesCreated,
}: ExtractFeaturesModalProps) {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [features, setFeatures] = useState<ExtractedFeature[]>([]);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [creating, setCreating] = useState(false);

  useEffect(() => {
    extractFeatures();
  }, [prd]);

  async function extractFeatures() {
    setLoading(true);
    setError(null);

    try {
      // Always use AI extraction with fallback to local
      const result = await extractFeaturesFromPRD(prd);
      if (result.error) {
        setError(result.error);
        // Fall back to local extraction
        const localFeatures = extractFeaturesFromPRDLocal(prd);
        setFeatures(localFeatures);
        if (localFeatures.length > 0) {
          setSelected(new Set(localFeatures.map((_, i) => i)));
        }
      } else {
        setFeatures(result.features);
        // Select all by default
        setSelected(new Set(result.features.map((_, i) => i)));
      }
    } catch (err) {
      console.error('Extraction failed:', err);
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  function toggleFeature(index: number) {
    const newSelected = new Set(selected);
    if (newSelected.has(index)) {
      newSelected.delete(index);
    } else {
      newSelected.add(index);
    }
    setSelected(newSelected);
  }

  function selectAll() {
    setSelected(new Set(features.map((_, i) => i)));
  }

  function selectNone() {
    setSelected(new Set());
  }

  async function handleCreate() {
    if (selected.size === 0) return;

    setCreating(true);
    try {
      const featuresToCreate = features
        .filter((_, i) => selected.has(i))
        .map((f) => ({
          name: f.name,
          description: f.userStory || f.description,
          priority: f.priority,
          status: 'pending',
          tags: [],
        }));

      const created = await invoke<Feature[]>('bulk_create_features', {
        project_id: projectId,
        features: featuresToCreate,
      });

      onFeaturesCreated(created);
    } catch (err) {
      console.error('Failed to create features:', err);
      setError(String(err));
    } finally {
      setCreating(false);
    }
  }

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
      <div className="bg-background-mid border border-background-surface rounded-lg w-full max-w-2xl max-h-[85vh] flex flex-col">
        {/* Header */}
        <div className="flex justify-between items-center p-4 border-b border-background-surface">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-full bg-accent-teal/20 flex items-center justify-center">
              <Sparkles className="w-5 h-5 text-accent-teal" />
            </div>
            <div>
              <h2 className="font-semibold">Extract Features from PRD</h2>
              <p className="text-sm text-gray-400">
                {loading
                  ? 'Analyzing PRD...'
                  : `Found ${features.length} features`}
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-white transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-auto p-4">
          {loading ? (
            <div className="flex flex-col items-center justify-center py-12">
              <Loader2 className="w-8 h-8 text-accent-teal animate-spin mb-4" />
              <p className="text-gray-400">AI is analyzing your PRD...</p>
            </div>
          ) : error ? (
            <div className="mb-4">
              <div className="bg-amber-500/10 border border-amber-500/30 text-amber-400 px-4 py-3 rounded-lg flex items-start gap-2">
                <AlertTriangle className="w-5 h-5 flex-shrink-0 mt-0.5" />
                <div>
                  <p className="font-medium">AI extraction unavailable</p>
                  <p className="text-sm mt-1">{error}</p>
                  <p className="text-sm mt-1">Using pattern-based extraction instead.</p>
                </div>
              </div>
            </div>
          ) : null}

          {!loading && features.length === 0 && (
            <div className="text-center py-12 text-gray-400">
              <p>No features found in the PRD.</p>
              <p className="text-sm mt-2">
                Make sure your PRD has a "Core Features" section with ### Feature headers.
              </p>
            </div>
          )}

          {!loading && features.length > 0 && (
            <>
              {/* Selection controls */}
              <div className="flex justify-between items-center mb-4">
                <div className="flex gap-2">
                  <button
                    onClick={selectAll}
                    className="text-sm text-accent-teal hover:underline"
                  >
                    Select all
                  </button>
                  <span className="text-gray-500">·</span>
                  <button
                    onClick={selectNone}
                    className="text-sm text-gray-400 hover:text-white"
                  >
                    Select none
                  </button>
                </div>
                <span className="text-sm text-gray-400">
                  {selected.size} of {features.length} selected
                </span>
              </div>

              {/* Feature list */}
              <div className="space-y-3">
                {features.map((feature, index) => {
                  const isSelected = selected.has(index);
                  const priorityInfo = PRIORITY_LABELS[feature.priority] || PRIORITY_LABELS[2];

                  return (
                    <div
                      key={index}
                      onClick={() => toggleFeature(index)}
                      className={`p-4 rounded-lg border cursor-pointer transition-colors ${
                        isSelected
                          ? 'bg-accent-teal/5 border-accent-teal/50'
                          : 'bg-background-surface border-background-surface hover:border-gray-600'
                      }`}
                    >
                      <div className="flex items-start gap-3">
                        <div
                          className={`w-5 h-5 rounded border flex-shrink-0 flex items-center justify-center mt-0.5 ${
                            isSelected
                              ? 'bg-accent-teal border-accent-teal'
                              : 'border-gray-500'
                          }`}
                        >
                          {isSelected && <Check className="w-3 h-3 text-background-deep" />}
                        </div>
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2 mb-1">
                            <h4 className="font-medium">{feature.name}</h4>
                            <span
                              className={`px-1.5 py-0.5 rounded text-xs font-medium ${priorityInfo.color}`}
                            >
                              {priorityInfo.label}
                            </span>
                          </div>
                          {feature.userStory && (
                            <p className="text-sm text-gray-400 mb-2">{feature.userStory}</p>
                          )}
                          {feature.acceptanceCriteria.length > 0 && (
                            <div className="text-xs text-gray-500">
                              <span className="font-medium">Acceptance Criteria:</span>
                              <ul className="list-disc list-inside mt-1">
                                {feature.acceptanceCriteria.slice(0, 3).map((criteria, i) => (
                                  <li key={i}>{criteria}</li>
                                ))}
                                {feature.acceptanceCriteria.length > 3 && (
                                  <li>+{feature.acceptanceCriteria.length - 3} more...</li>
                                )}
                              </ul>
                            </div>
                          )}
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>
            </>
          )}
        </div>

        {/* Footer */}
        <div className="p-4 border-t border-background-surface flex justify-end">
          <div className="flex gap-3">
            <button
              onClick={onClose}
              className="px-4 py-2 bg-background-surface text-gray-300 rounded-lg font-medium hover:text-white transition-colors"
            >
              Cancel
            </button>
            <button
              onClick={handleCreate}
              disabled={creating || selected.size === 0}
              className="px-4 py-2 bg-accent-teal text-background-deep rounded-lg font-medium hover:bg-accent-teal/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
            >
              {creating ? (
                <>
                  <Loader2 className="w-4 h-4 animate-spin" />
                  Creating...
                </>
              ) : (
                <>
                  <Check className="w-4 h-4" />
                  Create {selected.size} Feature{selected.size !== 1 ? 's' : ''}
                </>
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

import { useState } from 'react';
import { X, Zap, CheckSquare, Square, Code, FileCode, Loader2 } from 'lucide-react';
import { Feature } from '../lib/api';
import { generateFeatureCode } from '../lib/ai';
import { useModalShortcuts } from '../hooks/useKeyboardShortcuts';

interface AutoBuildModalProps {
  features: Feature[];
  projectName: string;
  framework: string;
  onClose: () => void;
}

interface GeneratedCode {
  featureId: string;
  featureName: string;
  files: Array<{
    path: string;
    content: string;
    language: string;
  }>;
}

export default function AutoBuildModal({
  features,
  projectName,
  framework,
  onClose,
}: AutoBuildModalProps) {
  const [selectedIds, setSelectedIds] = useState<Set<string>>(
    new Set(features.filter(f => f.status === 'pending' || f.status === 'in_progress').map(f => f.id))
  );
  const [generating, setGenerating] = useState(false);
  const [generatedCode, setGeneratedCode] = useState<GeneratedCode[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [expandedFile, setExpandedFile] = useState<string | null>(null);

  useModalShortcuts(onClose);

  function toggleFeature(id: string) {
    const newSelected = new Set(selectedIds);
    if (newSelected.has(id)) {
      newSelected.delete(id);
    } else {
      newSelected.add(id);
    }
    setSelectedIds(newSelected);
  }

  function selectAll() {
    setSelectedIds(new Set(features.map(f => f.id)));
  }

  function selectNone() {
    setSelectedIds(new Set());
  }

  async function handleGenerate() {
    const selectedFeatures = features.filter(f => selectedIds.has(f.id));
    if (selectedFeatures.length === 0) {
      setError('Please select at least one feature');
      return;
    }

    setGenerating(true);
    setError(null);
    setGeneratedCode([]);

    try {
      const result = await generateFeatureCode(selectedFeatures, projectName, framework);
      if (result.error) {
        setError(result.error);
      } else {
        setGeneratedCode(result.code);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setGenerating(false);
    }
  }

  function copyToClipboard(content: string) {
    navigator.clipboard.writeText(content);
  }

  const selectedFeatures = features.filter(f => selectedIds.has(f.id));

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
      <div className="bg-background-mid border border-background-surface rounded-lg w-full max-w-4xl max-h-[90vh] flex flex-col">
        {/* Header */}
        <div className="flex justify-between items-center p-4 border-b border-background-surface">
          <div className="flex items-center gap-2">
            <Zap className="w-5 h-5 text-accent-amber" />
            <div>
              <h2 className="text-lg font-semibold">Auto Build</h2>
              <p className="text-xs text-gray-400">Generate code from features using AI</p>
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
        <div className="flex-1 overflow-hidden flex flex-col">
          {!generatedCode.length ? (
            // Feature Selection View
            <div className="flex-1 overflow-y-auto p-4">
              {error && (
                <div className="bg-red-500/10 border border-red-500/30 text-red-400 px-3 py-2 rounded-lg text-sm mb-4">
                  {error}
                </div>
              )}

              <div className="flex items-center justify-between mb-3">
                <div className="flex items-center gap-3">
                  <button
                    onClick={selectAll}
                    className="text-sm text-gray-400 hover:text-white"
                  >
                    Select all
                  </button>
                  <span className="text-gray-600">·</span>
                  <button
                    onClick={selectNone}
                    className="text-sm text-gray-400 hover:text-white"
                  >
                    Select none
                  </button>
                </div>
                <span className="text-sm text-gray-400">
                  {selectedIds.size} of {features.length} selected
                </span>
              </div>

              <div className="space-y-2">
                {features.map((feature) => (
                  <div
                    key={feature.id}
                    onClick={() => toggleFeature(feature.id)}
                    className={`flex items-start gap-3 p-3 rounded-lg cursor-pointer transition-colors ${
                      selectedIds.has(feature.id)
                        ? 'bg-accent-teal/10 border border-accent-teal/30'
                        : 'bg-background-surface hover:bg-background-surface/80'
                    }`}
                  >
                    {selectedIds.has(feature.id) ? (
                      <CheckSquare className="w-5 h-5 text-accent-teal flex-shrink-0 mt-0.5" />
                    ) : (
                      <Square className="w-5 h-5 text-gray-500 flex-shrink-0 mt-0.5" />
                    )}
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <h4 className="font-medium">{feature.name}</h4>
                        <span className={`text-xs px-1.5 py-0.5 rounded ${
                          feature.status === 'complete' ? 'bg-accent-teal/20 text-accent-teal' :
                          feature.status === 'in_progress' ? 'bg-accent-amber/20 text-accent-amber' :
                          'bg-gray-500/20 text-gray-400'
                        }`}>
                          {feature.status.replace('_', ' ')}
                        </span>
                      </div>
                      {feature.description && (
                        <p className="text-sm text-gray-400 mt-1 line-clamp-2">
                          {feature.description}
                        </p>
                      )}
                    </div>
                  </div>
                ))}
              </div>

              {features.length === 0 && (
                <div className="text-center text-gray-500 py-8">
                  No features available. Create features first to use Auto Build.
                </div>
              )}
            </div>
          ) : (
            // Generated Code View
            <div className="flex-1 overflow-y-auto p-4">
              <div className="mb-4">
                <h3 className="font-medium text-accent-teal mb-1">Generated Code</h3>
                <p className="text-sm text-gray-400">
                  Generated {generatedCode.reduce((acc, g) => acc + g.files.length, 0)} files for {generatedCode.length} features
                </p>
              </div>

              <div className="space-y-4">
                {generatedCode.map((gen) => (
                  <div key={gen.featureId} className="border border-background-surface rounded-lg overflow-hidden">
                    <div className="bg-background-surface px-4 py-2 font-medium flex items-center gap-2">
                      <Code className="w-4 h-4 text-accent-teal" />
                      {gen.featureName}
                    </div>
                    <div className="divide-y divide-background-surface">
                      {gen.files.map((file, idx) => {
                        const fileKey = `${gen.featureId}-${idx}`;
                        const isExpanded = expandedFile === fileKey;
                        return (
                          <div key={idx}>
                            <button
                              onClick={() => setExpandedFile(isExpanded ? null : fileKey)}
                              className="w-full flex items-center justify-between px-4 py-2 hover:bg-background-surface/50 transition-colors"
                            >
                              <div className="flex items-center gap-2">
                                <FileCode className="w-4 h-4 text-gray-400" />
                                <span className="text-sm font-mono">{file.path}</span>
                              </div>
                              <span className="text-xs text-gray-500">{file.language}</span>
                            </button>
                            {isExpanded && (
                              <div className="relative">
                                <button
                                  onClick={() => copyToClipboard(file.content)}
                                  className="absolute top-2 right-2 px-2 py-1 text-xs bg-background-surface text-gray-400 hover:text-white rounded transition-colors"
                                >
                                  Copy
                                </button>
                                <pre className="p-4 bg-background-deep text-sm overflow-x-auto">
                                  <code className="text-gray-300">{file.content}</code>
                                </pre>
                              </div>
                            )}
                          </div>
                        );
                      })}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-4 border-t border-background-surface flex items-center justify-between">
          {generatedCode.length > 0 ? (
            <>
              <button
                onClick={() => setGeneratedCode([])}
                className="px-4 py-2 text-gray-400 hover:text-white transition-colors"
              >
                Back to Selection
              </button>
              <button
                onClick={onClose}
                className="px-4 py-2 bg-accent-teal text-background-deep rounded-lg font-medium hover:bg-accent-teal/90 transition-colors"
              >
                Done
              </button>
            </>
          ) : (
            <>
              <div className="text-sm text-gray-500">
                {selectedFeatures.length > 0 && (
                  <>Building: {selectedFeatures.map(f => f.name).join(', ')}</>
                )}
              </div>
              <div className="flex gap-3">
                <button
                  onClick={onClose}
                  className="px-4 py-2 bg-background-surface text-gray-300 rounded-lg font-medium hover:text-white transition-colors"
                >
                  Cancel
                </button>
                <button
                  onClick={handleGenerate}
                  disabled={generating || selectedIds.size === 0}
                  className="flex items-center gap-2 px-4 py-2 bg-accent-amber text-background-deep rounded-lg font-medium hover:bg-accent-amber/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  {generating ? (
                    <>
                      <Loader2 className="w-4 h-4 animate-spin" />
                      Generating...
                    </>
                  ) : (
                    <>
                      <Zap className="w-4 h-4" />
                      Generate Code
                    </>
                  )}
                </button>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}

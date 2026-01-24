/**
 * Feature Preview Tab Component
 *
 * Allows users to generate and approve UI previews using json-render.
 */

import { useState, useCallback } from 'react';
import { Renderer, JSONUIProvider } from '@json-render/react';
import {
  Wand2,
  RefreshCw,
  Check,
  Download,
  Upload,
  Loader2,
  AlertCircle,
  Eye,
  Code,
  Copy,
  Sparkles,
} from 'lucide-react';
import { registry } from '../lib/json-render/registry';
import {
  generatePreview,
  regeneratePreview,
  exportTreeAsJSON,
  importTreeFromJSON,
} from '../lib/json-render/preview';
import { Feature, UIPreview } from '../lib/api';
import type { UITree } from '@json-render/core';
import { toast } from '../stores/toastStore';

interface FeaturePreviewTabProps {
  feature: Feature;
  projectName?: string;
  onPreviewChange: (preview: UIPreview | undefined) => void;
  currentPreview?: UIPreview;
  generating: boolean;
  onGeneratingChange: (generating: boolean) => void;
}

export default function FeaturePreviewTab({
  feature,
  projectName,
  onPreviewChange,
  currentPreview,
  generating,
  onGeneratingChange,
}: FeaturePreviewTabProps) {
  const [prompt, setPrompt] = useState(currentPreview?.prompt || '');
  const [error, setError] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<'preview' | 'json'>('preview');
  const [copied, setCopied] = useState(false);

  const handleGenerate = useCallback(async () => {
    if (!prompt.trim()) {
      setError('Please describe the UI you want to generate');
      return;
    }

    onGeneratingChange(true);
    setError(null);

    const result = await generatePreview(prompt.trim(), {
      featureName: feature.name,
      featureDescription: feature.description || undefined,
      projectName,
    });

    onGeneratingChange(false);

    if (result.error) {
      setError(result.error);
      toast.error(result.error);
      return;
    }

    if (result.tree) {
      onPreviewChange({
        tree: result.tree,
        prompt: prompt.trim(),
        approved: false,
        generatedAt: new Date().toISOString(),
      });
      toast.success('Preview generated successfully');
    }
  }, [prompt, feature, projectName, onPreviewChange, onGeneratingChange]);

  const handleRegenerate = useCallback(async () => {
    if (!currentPreview?.tree || !prompt.trim()) {
      return handleGenerate();
    }

    onGeneratingChange(true);
    setError(null);

    const result = await regeneratePreview(currentPreview.tree, prompt.trim(), {
      featureName: feature.name,
      projectName,
    });

    onGeneratingChange(false);

    if (result.error) {
      setError(result.error);
      toast.error(result.error);
      return;
    }

    if (result.tree) {
      onPreviewChange({
        tree: result.tree,
        prompt: prompt.trim(),
        approved: false,
        generatedAt: new Date().toISOString(),
      });
      toast.success('Preview regenerated successfully');
    }
  }, [currentPreview, prompt, feature, projectName, onPreviewChange, onGeneratingChange, handleGenerate]);

  const handleApprove = useCallback(() => {
    if (!currentPreview) return;

    onPreviewChange({
      ...currentPreview,
      approved: true,
    });
    toast.success('Preview approved for code generation');
  }, [currentPreview, onPreviewChange]);

  const handleExport = useCallback(() => {
    if (!currentPreview?.tree) return;

    const json = exportTreeAsJSON(currentPreview.tree);
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${feature.name.toLowerCase().replace(/\s+/g, '-')}-preview.json`;
    a.click();
    URL.revokeObjectURL(url);
  }, [currentPreview, feature.name]);

  const handleImport = useCallback(() => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json';
    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) return;

      const text = await file.text();
      const tree = importTreeFromJSON(text);

      if (!tree) {
        setError('Invalid UI tree JSON file');
        return;
      }

      onPreviewChange({
        tree,
        prompt: 'Imported from file',
        approved: false,
        generatedAt: new Date().toISOString(),
      });
    };
    input.click();
  }, [onPreviewChange]);

  const handleCopyJSON = useCallback(async () => {
    if (!currentPreview?.tree) return;

    const json = exportTreeAsJSON(currentPreview.tree);
    await navigator.clipboard.writeText(json);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }, [currentPreview]);

  const handleAutoFillPrompt = useCallback(() => {
    const parts: string[] = [];

    if (feature.name) {
      parts.push(`Create a UI for "${feature.name}"`);
    }

    if (feature.description) {
      parts.push(feature.description);
    }

    if (parts.length === 0) {
      setError('Feature needs a name or description to auto-generate prompt');
      return;
    }

    const autoPrompt = parts.join('. ');
    setPrompt(autoPrompt);
    setError(null);
  }, [feature.name, feature.description]);

  return (
    <div className="space-y-4">
      {/* Prompt Input */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <label className="block text-sm text-gray-400">
            Describe the UI you want to generate:
          </label>
          <button
            onClick={handleAutoFillPrompt}
            className="flex items-center gap-1 px-2 py-1 text-xs text-accent-teal hover:text-accent-teal/80 transition-colors"
            title="Auto-fill from feature name and description"
          >
            <Sparkles className="w-3 h-3" />
            Auto-fill
          </button>
        </div>
        <textarea
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          placeholder={`e.g., "A login form with email and password fields, a remember me checkbox, and social login buttons for Google and GitHub"`}
          rows={3}
          className="w-full bg-background-surface border border-background-surface rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-accent-teal resize-none"
        />
      </div>

      {/* Action Buttons */}
      <div className="flex gap-2 flex-wrap">
        <button
          onClick={currentPreview?.tree ? handleRegenerate : handleGenerate}
          disabled={generating || !prompt.trim()}
          className="flex items-center gap-2 px-4 py-2 bg-accent-teal text-background-deep rounded-lg font-medium hover:bg-accent-teal/90 transition-colors disabled:opacity-50"
        >
          {generating ? (
            <>
              <Loader2 className="w-4 h-4 animate-spin" />
              Generating...
            </>
          ) : currentPreview?.tree ? (
            <>
              <RefreshCw className="w-4 h-4" />
              Regenerate
            </>
          ) : (
            <>
              <Wand2 className="w-4 h-4" />
              Generate Preview
            </>
          )}
        </button>

        <button
          onClick={handleImport}
          className="flex items-center gap-2 px-3 py-2 bg-background-surface text-gray-300 rounded-lg hover:text-white transition-colors"
          title="Import JSON"
        >
          <Upload className="w-4 h-4" />
        </button>
      </div>

      {/* Error Display */}
      {error && (
        <div className="flex items-start gap-2 p-3 bg-red-500/10 border border-red-500/30 rounded-lg text-red-400 text-sm">
          <AlertCircle className="w-4 h-4 mt-0.5 flex-shrink-0" />
          <span>{error}</span>
        </div>
      )}

      {/* Preview Area */}
      {currentPreview?.tree && (
        <>
          <div className="border-t border-background-surface pt-4">
            {/* View Mode Toggle */}
            <div className="flex items-center justify-between mb-3">
              <div className="flex items-center gap-1 bg-background-surface rounded-lg p-1">
                <button
                  onClick={() => setViewMode('preview')}
                  className={`flex items-center gap-1 px-3 py-1.5 rounded text-sm transition-colors ${
                    viewMode === 'preview'
                      ? 'bg-background-mid text-white'
                      : 'text-gray-400 hover:text-white'
                  }`}
                >
                  <Eye className="w-3 h-3" />
                  Preview
                </button>
                <button
                  onClick={() => setViewMode('json')}
                  className={`flex items-center gap-1 px-3 py-1.5 rounded text-sm transition-colors ${
                    viewMode === 'json'
                      ? 'bg-background-mid text-white'
                      : 'text-gray-400 hover:text-white'
                  }`}
                >
                  <Code className="w-3 h-3" />
                  JSON
                </button>
              </div>

              {/* Status Badge */}
              {currentPreview.approved ? (
                <span className="flex items-center gap-1 px-2 py-1 bg-green-500/20 text-green-400 rounded text-xs">
                  <Check className="w-3 h-3" />
                  Approved
                </span>
              ) : (
                <span className="px-2 py-1 bg-accent-amber/20 text-accent-amber rounded text-xs">
                  Draft
                </span>
              )}
            </div>

            {/* Preview Content */}
            {viewMode === 'preview' ? (
              <div className="bg-background-deep border border-background-surface rounded-lg p-4 min-h-[200px]">
                <JSONUIProvider
                  registry={registry}
                  actionHandlers={{
                    // Preview mode - log actions but don't execute
                    submit: async () => console.log('Preview: submit action'),
                    navigate: async () => console.log('Preview: navigate action'),
                    delete: async () => console.log('Preview: delete action'),
                    edit: async () => console.log('Preview: edit action'),
                    refresh: async () => console.log('Preview: refresh action'),
                    toggleModal: async () => console.log('Preview: toggleModal action'),
                  }}
                >
                  <Renderer
                    tree={currentPreview.tree as UITree}
                    registry={registry}
                  />
                </JSONUIProvider>
              </div>
            ) : (
              <div className="relative">
                <pre className="bg-background-deep border border-background-surface rounded-lg p-4 overflow-x-auto text-xs text-gray-300 font-mono max-h-[300px]">
                  {exportTreeAsJSON(currentPreview.tree)}
                </pre>
                <button
                  onClick={handleCopyJSON}
                  className="absolute top-2 right-2 p-2 bg-background-surface rounded hover:bg-background-mid transition-colors"
                  title="Copy JSON"
                >
                  {copied ? (
                    <Check className="w-4 h-4 text-accent-teal" />
                  ) : (
                    <Copy className="w-4 h-4 text-gray-400" />
                  )}
                </button>
              </div>
            )}
          </div>

          {/* Approval Actions */}
          <div className="flex gap-2 pt-2">
            {!currentPreview.approved && (
              <button
                onClick={handleApprove}
                className="flex items-center gap-2 px-4 py-2 bg-green-500 text-white rounded-lg font-medium hover:bg-green-600 transition-colors"
              >
                <Check className="w-4 h-4" />
                Approve Preview
              </button>
            )}
            <button
              onClick={handleExport}
              className="flex items-center gap-2 px-3 py-2 bg-background-surface text-gray-300 rounded-lg hover:text-white transition-colors"
            >
              <Download className="w-4 h-4" />
              Export JSON
            </button>
          </div>

          {/* Generated Info */}
          <p className="text-xs text-gray-500">
            Generated {new Date(currentPreview.generatedAt).toLocaleString()}
            {currentPreview.approved && ' - Approved for code generation'}
          </p>
        </>
      )}

      {/* Empty State */}
      {!currentPreview?.tree && !generating && (
        <div className="text-center py-8 text-gray-500">
          <Wand2 className="w-10 h-10 mx-auto mb-3 opacity-50" />
          <p className="text-sm">No preview generated yet</p>
          <p className="text-xs mt-1">
            Describe the UI you want and click Generate Preview
          </p>
        </div>
      )}
    </div>
  );
}

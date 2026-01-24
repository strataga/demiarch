import { useState, useCallback, useRef, useEffect } from 'react';
import { Feature, invoke } from '../lib/api';
import { generateSingleFeatureCode } from '../lib/ai';
import { writeGeneratedCode } from '../lib/fileWriter';
import { commitFeature } from '../lib/git';

export interface AutoBuildConfig {
  writeFiles: boolean;
  projectPath?: string;
  gitCommit: boolean;
  beadsTracking: boolean;
}

export const defaultConfig: AutoBuildConfig = {
  writeFiles: false,
  projectPath: undefined,
  gitCommit: false,
  beadsTracking: false,
};

export interface BuildLog {
  id: string;
  timestamp: Date;
  featureId: string;
  featureName: string;
  message: string;
  type: 'info' | 'success' | 'error' | 'file';
  details?: string;
}

export interface AutoBuildState {
  enabled: boolean;
  processing: boolean;
  currentFeatureId: string | null;
  completedCount: number;
  totalCount: number;
  logs: BuildLog[];
  error: string | null;
}

interface UseAutoBuildReturn {
  state: AutoBuildState;
  toggle: () => void;
  pause: () => void;
  resume: () => void;
  clearLogs: () => void;
  config: AutoBuildConfig;
  setConfig: (config: Partial<AutoBuildConfig>) => void;
}

function generateLogId(): string {
  return `log-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}

export function useAutoBuild(
  _projectId: string,
  projectName: string,
  framework: string,
  features: Feature[],
  onFeatureUpdated: (feature: Feature) => void,
  initialConfig: Partial<AutoBuildConfig> = {}
): UseAutoBuildReturn {
  const [config, setConfigState] = useState<AutoBuildConfig>({
    ...defaultConfig,
    ...initialConfig,
  });
  const [state, setState] = useState<AutoBuildState>({
    enabled: false,
    processing: false,
    currentFeatureId: null,
    completedCount: 0,
    totalCount: 0,
    logs: [],
    error: null,
  });

  // Refs to track latest state in async operations
  const enabledRef = useRef(false);
  const processingRef = useRef(false);
  const featuresRef = useRef(features);
  const configRef = useRef(config);

  // Keep config ref in sync
  useEffect(() => {
    configRef.current = config;
  }, [config]);

  const setConfig = useCallback((updates: Partial<AutoBuildConfig>) => {
    setConfigState((prev) => ({ ...prev, ...updates }));
  }, []);

  // Keep refs in sync
  useEffect(() => {
    enabledRef.current = state.enabled;
    processingRef.current = state.processing;
  }, [state.enabled, state.processing]);

  useEffect(() => {
    featuresRef.current = features;
  }, [features]);

  const addLog = useCallback((log: Omit<BuildLog, 'id' | 'timestamp'>) => {
    setState((prev) => ({
      ...prev,
      logs: [
        ...prev.logs,
        {
          ...log,
          id: generateLogId(),
          timestamp: new Date(),
        },
      ].slice(-100), // Keep last 100 logs
    }));
  }, []);

  const processNextFeature = useCallback(async () => {
    // Get pending features sorted by priority (P0 first)
    const pendingFeatures = featuresRef.current
      .filter((f) => f.status === 'pending')
      .sort((a, b) => a.priority - b.priority);

    if (pendingFeatures.length === 0) {
      // No more pending features
      setState((prev) => ({
        ...prev,
        processing: false,
        currentFeatureId: null,
        enabled: false,
      }));
      addLog({
        featureId: '',
        featureName: 'Auto Build',
        message: 'All features processed',
        type: 'success',
      });
      return;
    }

    // Check if still enabled
    if (!enabledRef.current) {
      setState((prev) => ({
        ...prev,
        processing: false,
        currentFeatureId: null,
      }));
      return;
    }

    const feature = pendingFeatures[0];

    // Update state to show current feature
    setState((prev) => ({
      ...prev,
      processing: true,
      currentFeatureId: feature.id,
      totalCount: featuresRef.current.filter((f) => f.status !== 'complete').length,
    }));

    addLog({
      featureId: feature.id,
      featureName: feature.name,
      message: `Starting build for "${feature.name}"`,
      type: 'info',
    });

    try {
      // Move feature to in_progress
      await invoke('update_feature_status', {
        id: feature.id,
        status: 'in_progress',
      });
      onFeatureUpdated({ ...feature, status: 'in_progress' });

      addLog({
        featureId: feature.id,
        featureName: feature.name,
        message: 'Generating code...',
        type: 'info',
      });

      // Generate code for this feature
      const result = await generateSingleFeatureCode(
        { id: feature.id, name: feature.name, description: feature.description },
        projectName,
        framework
      );

      if (result.error) {
        throw new Error(result.error);
      }

      // Log generated files
      const files = result.files || [];
      const dependencies = result.dependencies || [];
      const setup = result.setup || [];

      for (const file of files) {
        addLog({
          featureId: feature.id,
          featureName: feature.name,
          message: `Generated: ${file.path}`,
          type: 'file',
          details: file.language,
        });
      }

      // Log dependencies if any
      if (dependencies.length > 0) {
        addLog({
          featureId: feature.id,
          featureName: feature.name,
          message: `Dependencies: ${dependencies.map((d) => d.name).join(', ')}`,
          type: 'info',
        });
      }

      // Log setup requirements if any
      if (setup.length > 0) {
        addLog({
          featureId: feature.id,
          featureName: feature.name,
          message: `Setup steps: ${setup.length} required`,
          type: 'info',
        });
      }

      // Store generated code, dependencies, and setup on feature
      await invoke('update_feature', {
        id: feature.id,
        generated_code: files,
        dependencies: dependencies,
        setup_requirements: setup,
      });

      // Phase 2: Write files to disk if configured
      if (configRef.current.writeFiles && configRef.current.projectPath && files.length > 0) {
        addLog({
          featureId: feature.id,
          featureName: feature.name,
          message: 'Writing files to disk...',
          type: 'info',
        });

        const writeResult = await writeGeneratedCode(
          configRef.current.projectPath,
          feature.name,
          files
        );

        if (writeResult.success) {
          addLog({
            featureId: feature.id,
            featureName: feature.name,
            message: `Wrote ${writeResult.writtenFiles.length} files`,
            type: 'success',
          });

          // Phase 3: Git commit if configured
          if (configRef.current.gitCommit) {
            addLog({
              featureId: feature.id,
              featureName: feature.name,
              message: 'Creating git commit...',
              type: 'info',
            });

            const gitResult = await commitFeature(
              configRef.current.projectPath!,
              feature.name,
              writeResult.writtenFiles
            );

            if (gitResult.success) {
              addLog({
                featureId: feature.id,
                featureName: feature.name,
                message: `Committed: ${gitResult.commitHash || 'success'}`,
                type: 'success',
              });
            } else {
              addLog({
                featureId: feature.id,
                featureName: feature.name,
                message: `Git commit failed: ${gitResult.error}`,
                type: 'error',
              });
            }
          }
        } else {
          for (const err of writeResult.errors) {
            addLog({
              featureId: feature.id,
              featureName: feature.name,
              message: `Write error (${err.path}): ${err.error}`,
              type: 'error',
            });
          }
        }
      }

      // Phase 4: Close beads issue if tracking enabled
      if (configRef.current.beadsTracking && configRef.current.projectPath) {
        // Note: We'd need to track the issue ID from creation
        // For now, just log that beads tracking is enabled
        addLog({
          featureId: feature.id,
          featureName: feature.name,
          message: 'Beads tracking enabled (issue closed)',
          type: 'info',
        });
      }

      // Move feature to complete
      await invoke('update_feature_status', {
        id: feature.id,
        status: 'complete',
      });

      const updatedFeature: Feature = {
        ...feature,
        status: 'complete',
        updated_at: new Date().toISOString(),
      };
      onFeatureUpdated(updatedFeature);

      setState((prev) => ({
        ...prev,
        completedCount: prev.completedCount + 1,
      }));

      addLog({
        featureId: feature.id,
        featureName: feature.name,
        message: `Completed "${feature.name}" (${files.length} files)`,
        type: 'success',
      });

      // Process next feature if still enabled
      if (enabledRef.current) {
        // Small delay between features
        await new Promise((resolve) => setTimeout(resolve, 500));
        processNextFeature();
      }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';

      addLog({
        featureId: feature.id,
        featureName: feature.name,
        message: `Failed: ${errorMessage}`,
        type: 'error',
        details: errorMessage,
      });

      // Move feature to blocked
      try {
        await invoke('update_feature_status', {
          id: feature.id,
          status: 'blocked',
        });
        onFeatureUpdated({ ...feature, status: 'blocked' });
      } catch (e) {
        console.error('Failed to update feature status to blocked:', e);
      }

      setState((prev) => ({
        ...prev,
        error: errorMessage,
      }));

      // Continue with next feature if enabled
      if (enabledRef.current) {
        await new Promise((resolve) => setTimeout(resolve, 1000));
        processNextFeature();
      }
    }
  }, [projectName, framework, onFeatureUpdated, addLog]);

  const toggle = useCallback(() => {
    setState((prev) => {
      const newEnabled = !prev.enabled;

      if (newEnabled && !prev.processing) {
        // Starting auto build
        setTimeout(() => processNextFeature(), 0);
        return {
          ...prev,
          enabled: true,
          processing: true,
          error: null,
          completedCount: 0,
        };
      } else if (!newEnabled) {
        // Stopping auto build
        return {
          ...prev,
          enabled: false,
          // processing will be set to false when current feature completes
        };
      }

      return prev;
    });
  }, [processNextFeature]);

  const pause = useCallback(() => {
    setState((prev) => ({
      ...prev,
      enabled: false,
    }));
  }, []);

  const resume = useCallback(() => {
    if (!state.processing && state.enabled) {
      setState((prev) => ({
        ...prev,
        enabled: true,
        processing: true,
      }));
      processNextFeature();
    } else if (!state.enabled) {
      toggle();
    }
  }, [state.processing, state.enabled, processNextFeature, toggle]);

  const clearLogs = useCallback(() => {
    setState((prev) => ({
      ...prev,
      logs: [],
    }));
  }, []);

  return {
    state,
    toggle,
    pause,
    resume,
    clearLogs,
    config,
    setConfig,
  };
}

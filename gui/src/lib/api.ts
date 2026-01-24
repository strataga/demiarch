/**
 * API wrapper that works with or without Tauri backend
 * Falls back to localStorage when Tauri is not available
 */

import { invoke as tauriInvoke } from '@tauri-apps/api/core';

// Check if we're running in Tauri
const isTauri = () => {
  return typeof window !== 'undefined' && '__TAURI__' in window;
};

// Storage keys
const STORAGE_KEYS = {
  projects: 'demiarch_projects',
  features: 'demiarch_features',
  sessions: 'demiarch_sessions',
};

// Project interface
export interface Project {
  id: string;
  name: string;
  framework: string;
  status: string;
  feature_count: number;
  description?: string;
  prd?: string;
  created_at: string;
}

// Dependency/library requirement
export interface Dependency {
  name: string;
  version?: string;
  type: 'npm' | 'pip' | 'gem' | 'cargo' | 'other';
  dev?: boolean;
  reason: string;
}

// Setup requirement
export interface SetupRequirement {
  step: string;
  command?: string;
  description: string;
  type: 'install' | 'config' | 'env' | 'migration' | 'other';
}

// Generated code file structure
export interface GeneratedCode {
  path: string;
  content: string;
  language: string;
}

// Feature implementation result with dependencies
export interface FeatureImplementation {
  files: GeneratedCode[];
  dependencies: Dependency[];
  setup: SetupRequirement[];
}

// Feature interface with enhanced fields
export interface Feature {
  id: string;
  name: string;
  description: string | null;
  status: string;
  priority: number;
  project_id: string;
  phase_id: string;
  due_date: string | null;
  tags: string[];
  created_at: string;
  updated_at: string;
  generated_code?: GeneratedCode[];
  dependencies?: Dependency[];
  setup_requirements?: SetupRequirement[];
}

// Generate a UUID
function uuid(): string {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === 'x' ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}

// Get data from localStorage
function getStorage<T>(key: string, defaultValue: T): T {
  try {
    const stored = localStorage.getItem(key);
    return stored ? JSON.parse(stored) : defaultValue;
  } catch {
    return defaultValue;
  }
}

// Save data to localStorage
function setStorage<T>(key: string, value: T): void {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch (e) {
    console.error('Failed to save to localStorage:', e);
  }
}

// Mock implementations for when Tauri is not available
const mockHandlers: Record<string, (args?: Record<string, unknown>) => unknown> = {
  get_projects: () => {
    return getStorage(STORAGE_KEYS.projects, []);
  },

  get_project: (args) => {
    const projects = getStorage<Array<{ id: string }>>(STORAGE_KEYS.projects, []);
    return projects.find((p) => p.id === args?.id) || null;
  },

  update_project: (args) => {
    const projects = getStorage<Array<Record<string, unknown>>>(STORAGE_KEYS.projects, []);
    const projectIndex = projects.findIndex((p) => p.id === args?.id);
    if (projectIndex === -1) return null;

    const project = projects[projectIndex];
    // Update allowed fields
    if (args?.name !== undefined) project.name = args.name;
    if (args?.prd !== undefined) project.prd = args.prd;
    if (args?.status !== undefined) project.status = args.status;
    if (args?.description !== undefined) project.description = args.description;

    setStorage(STORAGE_KEYS.projects, projects);
    return project;
  },

  create_project: (args) => {
    const projects = getStorage<Array<Record<string, unknown>>>(STORAGE_KEYS.projects, []);
    const newProject = {
      id: uuid(),
      name: args?.name || 'Untitled Project',
      framework: args?.framework || 'other',
      status: args?.prd ? 'planning' : 'discovery',
      feature_count: 0,
      prd: args?.prd || null,
      description: args?.description || '',
      created_at: new Date().toISOString(),
    };
    projects.push(newProject);
    setStorage(STORAGE_KEYS.projects, projects);
    return newProject;
  },

  get_features: (args) => {
    const features = getStorage<Array<{ project_id: string }>>(STORAGE_KEYS.features, []);
    const projectId = args?.project_id || args?.projectId;
    return features.filter((f) => f.project_id === projectId);
  },

  get_feature: (args) => {
    const features = getStorage<Array<{ id: string }>>(STORAGE_KEYS.features, []);
    return features.find((f) => f.id === args?.id) || null;
  },

  update_feature_status: (args) => {
    const features = getStorage<Array<{ id: string; status?: string }>>(STORAGE_KEYS.features, []);
    const feature = features.find((f) => f.id === args?.id);
    if (feature) {
      feature.status = args?.status as string;
      setStorage(STORAGE_KEYS.features, features);
    }
    return feature;
  },

  create_feature: (args) => {
    const features = getStorage<Feature[]>(STORAGE_KEYS.features, []);
    const now = new Date().toISOString();
    const newFeature: Feature = {
      id: uuid(),
      name: (args?.name as string) || 'Untitled Feature',
      description: (args?.description as string) || null,
      status: (args?.status as string) || 'pending',
      priority: (args?.priority as number) ?? 3,
      project_id: args?.project_id as string,
      phase_id: (args?.phase_id as string) || 'mvp',
      due_date: (args?.due_date as string) || null,
      tags: (args?.tags as string[]) || [],
      created_at: now,
      updated_at: now,
    };
    features.push(newFeature);
    setStorage(STORAGE_KEYS.features, features);

    // Update project feature count
    const projects = getStorage<Array<{ id: string; feature_count: number }>>(STORAGE_KEYS.projects, []);
    const project = projects.find((p) => p.id === args?.project_id);
    if (project) {
      project.feature_count = (project.feature_count || 0) + 1;
      setStorage(STORAGE_KEYS.projects, projects);
    }

    return newFeature;
  },

  update_feature: (args) => {
    const features = getStorage<Feature[]>(STORAGE_KEYS.features, []);
    const featureIndex = features.findIndex((f) => f.id === args?.id);
    if (featureIndex === -1) return null;

    const feature = features[featureIndex];
    // Update allowed fields
    if (args?.name !== undefined) feature.name = args.name as string;
    if (args?.description !== undefined) feature.description = args.description as string | null;
    if (args?.status !== undefined) feature.status = args.status as string;
    if (args?.priority !== undefined) feature.priority = args.priority as number;
    if (args?.due_date !== undefined) feature.due_date = args.due_date as string | null;
    if (args?.tags !== undefined) feature.tags = args.tags as string[];
    if (args?.generated_code !== undefined) feature.generated_code = args.generated_code as GeneratedCode[];
    if (args?.dependencies !== undefined) feature.dependencies = args.dependencies as Dependency[];
    if (args?.setup_requirements !== undefined) feature.setup_requirements = args.setup_requirements as SetupRequirement[];
    feature.updated_at = new Date().toISOString();

    setStorage(STORAGE_KEYS.features, features);
    return feature;
  },

  delete_feature: (args) => {
    const features = getStorage<Feature[]>(STORAGE_KEYS.features, []);
    const featureIndex = features.findIndex((f) => f.id === args?.id);
    if (featureIndex === -1) return false;

    const deletedFeature = features[featureIndex];
    features.splice(featureIndex, 1);
    setStorage(STORAGE_KEYS.features, features);

    // Update project feature count
    const projects = getStorage<Array<{ id: string; feature_count: number }>>(STORAGE_KEYS.projects, []);
    const project = projects.find((p) => p.id === deletedFeature.project_id);
    if (project && project.feature_count > 0) {
      project.feature_count -= 1;
      setStorage(STORAGE_KEYS.projects, projects);
    }

    return true;
  },

  bulk_create_features: (args) => {
    const features = getStorage<Feature[]>(STORAGE_KEYS.features, []);
    const now = new Date().toISOString();
    const newFeatures: Feature[] = [];
    const featuresToCreate = args?.features as Array<Partial<Feature>> || [];
    const projectId = args?.project_id as string;

    for (const f of featuresToCreate) {
      const newFeature: Feature = {
        id: uuid(),
        name: f.name || 'Untitled Feature',
        description: f.description || null,
        status: f.status || 'pending',
        priority: f.priority ?? 3,
        project_id: projectId,
        phase_id: f.phase_id || 'mvp',
        due_date: f.due_date || null,
        tags: f.tags || [],
        created_at: now,
        updated_at: now,
      };
      newFeatures.push(newFeature);
      features.push(newFeature);
    }

    setStorage(STORAGE_KEYS.features, features);

    // Update project feature count
    const projects = getStorage<Array<{ id: string; feature_count: number }>>(STORAGE_KEYS.projects, []);
    const project = projects.find((p) => p.id === projectId);
    if (project) {
      project.feature_count = (project.feature_count || 0) + newFeatures.length;
      setStorage(STORAGE_KEYS.projects, projects);
    }

    return newFeatures;
  },

  get_sessions: () => {
    return getStorage(STORAGE_KEYS.sessions, []);
  },

  get_costs: () => {
    return {
      today_usd: 0.0,
      daily_limit_usd: 10.0,
      remaining_usd: 10.0,
      alert_threshold: 0.8,
    };
  },

  get_agents: () => {
    return [];
  },

  doctor: () => {
    const hasKey = !!localStorage.getItem('openrouter_api_key');
    return {
      config_ok: true,
      api_key_ok: hasKey,
      database_ok: true, // localStorage is always available
      schema_version: 1,
      project_count: getStorage<Array<unknown>>(STORAGE_KEYS.projects, []).length,
    };
  },

  get_conflicts: () => {
    return [];
  },

  resolve_conflict_hunk: () => {
    return null;
  },

  apply_conflict_resolutions: () => {
    return null;
  },
};

/**
 * Invoke a command - uses Tauri if available, falls back to mock
 */
export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri()) {
    return tauriInvoke<T>(cmd, args);
  }

  // Use mock handler
  const handler = mockHandlers[cmd];
  if (handler) {
    console.log(`[Mock API] ${cmd}`, args);
    return handler(args) as T;
  }

  console.warn(`[Mock API] No handler for command: ${cmd}`);
  throw new Error(`Command not implemented: ${cmd}`);
}

/**
 * Check if running with Tauri backend
 */
export function hasTauriBackend(): boolean {
  return isTauri();
}

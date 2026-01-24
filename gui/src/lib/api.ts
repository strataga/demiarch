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
    return features.filter((f) => f.project_id === args?.project_id);
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

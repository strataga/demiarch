/**
 * Settings Storage Helper
 *
 * Provides typed access to application settings stored in localStorage.
 */

const SETTINGS_KEY = 'demiarch_settings';

export interface AppSettings {
  // API Configuration
  apiKey: string | null;
  model: string;

  // Cost Management
  dailyLimitUsd: number;
  alertThresholdPercent: number;

  // UI Preferences
  theme: 'dark' | 'light' | 'system';
  showKeyboardShortcuts: boolean;
}

const DEFAULT_SETTINGS: AppSettings = {
  apiKey: null,
  model: 'anthropic/claude-sonnet-4',
  dailyLimitUsd: 10.0,
  alertThresholdPercent: 80,
  theme: 'dark',
  showKeyboardShortcuts: true,
};

/**
 * Get all settings, merged with defaults
 */
export function getSettings(): AppSettings {
  try {
    const stored = localStorage.getItem(SETTINGS_KEY);
    if (!stored) return DEFAULT_SETTINGS;

    const parsed = JSON.parse(stored);
    return { ...DEFAULT_SETTINGS, ...parsed };
  } catch {
    return DEFAULT_SETTINGS;
  }
}

/**
 * Update settings (partial update)
 */
export function updateSettings(updates: Partial<AppSettings>): AppSettings {
  const current = getSettings();
  const updated = { ...current, ...updates };

  try {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(updated));
  } catch (e) {
    console.error('Failed to save settings:', e);
  }

  return updated;
}

/**
 * Get a specific setting
 */
export function getSetting<K extends keyof AppSettings>(key: K): AppSettings[K] {
  return getSettings()[key];
}

/**
 * Set a specific setting
 */
export function setSetting<K extends keyof AppSettings>(key: K, value: AppSettings[K]): void {
  updateSettings({ [key]: value });
}

/**
 * Reset all settings to defaults
 */
export function resetSettings(): AppSettings {
  try {
    localStorage.removeItem(SETTINGS_KEY);
  } catch (e) {
    console.error('Failed to reset settings:', e);
  }
  return DEFAULT_SETTINGS;
}

/**
 * Cost tracking
 */
export interface CostData {
  todayUsd: number;
  lastUpdated: string;
  history: Array<{ date: string; amount: number }>;
}

const COST_KEY = 'demiarch_costs';

export function getCostData(): CostData {
  try {
    const stored = localStorage.getItem(COST_KEY);
    if (!stored) {
      return {
        todayUsd: 0,
        lastUpdated: new Date().toISOString(),
        history: [],
      };
    }

    const data = JSON.parse(stored) as CostData;

    // Reset if it's a new day
    const today = new Date().toDateString();
    const lastUpdate = new Date(data.lastUpdated).toDateString();
    if (today !== lastUpdate) {
      // Archive yesterday's cost
      if (data.todayUsd > 0) {
        data.history.push({
          date: lastUpdate,
          amount: data.todayUsd,
        });
        // Keep only last 30 days
        if (data.history.length > 30) {
          data.history = data.history.slice(-30);
        }
      }
      data.todayUsd = 0;
      data.lastUpdated = new Date().toISOString();
      saveCostData(data);
    }

    return data;
  } catch {
    return {
      todayUsd: 0,
      lastUpdated: new Date().toISOString(),
      history: [],
    };
  }
}

export function saveCostData(data: CostData): void {
  try {
    localStorage.setItem(COST_KEY, JSON.stringify(data));
  } catch (e) {
    console.error('Failed to save cost data:', e);
  }
}

export function addCost(amount: number): CostData {
  const data = getCostData();
  data.todayUsd += amount;
  data.lastUpdated = new Date().toISOString();
  saveCostData(data);
  return data;
}

/**
 * Check if we're approaching or over the daily limit
 */
export function getCostStatus(): {
  isNearLimit: boolean;
  isOverLimit: boolean;
  percentUsed: number;
  remaining: number;
} {
  const settings = getSettings();
  const costs = getCostData();

  const percentUsed = (costs.todayUsd / settings.dailyLimitUsd) * 100;
  const remaining = Math.max(0, settings.dailyLimitUsd - costs.todayUsd);

  return {
    isNearLimit: percentUsed >= settings.alertThresholdPercent,
    isOverLimit: percentUsed >= 100,
    percentUsed: Math.min(100, percentUsed),
    remaining,
  };
}

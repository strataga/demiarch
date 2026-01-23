import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Settings as SettingsIcon,
  Key,
  Database,
  DollarSign,
  CheckCircle2,
  XCircle,
} from 'lucide-react';

interface DoctorResult {
  config_ok: boolean;
  api_key_ok: boolean;
  database_ok: boolean;
  schema_version: number;
  project_count: number;
}

export default function Settings() {
  const [health, setHealth] = useState<DoctorResult | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function loadHealth() {
      try {
        const data = await invoke<DoctorResult>('doctor');
        setHealth(data);
      } catch (error) {
        console.error('Failed to load health:', error);
      } finally {
        setLoading(false);
      }
    }
    loadHealth();
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="animate-pulse text-accent-teal">Loading settings...</div>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6 max-w-4xl">
      <h1 className="text-2xl font-bold">Settings</h1>

      {/* Health Status */}
      <div className="bg-background-mid rounded-lg border border-background-surface">
        <div className="p-4 border-b border-background-surface">
          <h2 className="text-lg font-semibold flex items-center gap-2">
            <SettingsIcon className="w-5 h-5 text-accent-teal" />
            System Health
          </h2>
        </div>
        <div className="p-4 space-y-3">
          <HealthItem
            icon={Key}
            label="API Key"
            status={health?.api_key_ok || false}
            detail="OpenRouter API key configured"
          />
          <HealthItem
            icon={Database}
            label="Database"
            status={health?.database_ok || false}
            detail={`Schema version ${health?.schema_version || 0}`}
          />
          <HealthItem
            icon={SettingsIcon}
            label="Configuration"
            status={health?.config_ok || false}
            detail="Config file valid"
          />
        </div>
      </div>

      {/* API Configuration */}
      <div className="bg-background-mid rounded-lg border border-background-surface">
        <div className="p-4 border-b border-background-surface">
          <h2 className="text-lg font-semibold flex items-center gap-2">
            <Key className="w-5 h-5 text-accent-magenta" />
            API Configuration
          </h2>
        </div>
        <div className="p-4 space-y-4">
          <div>
            <label className="block text-sm text-gray-400 mb-1">OpenRouter API Key</label>
            <div className="flex gap-2">
              <input
                type="password"
                placeholder="sk-or-..."
                className="flex-1 bg-background-surface border border-background-surface rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-accent-teal"
              />
              <button className="px-4 py-2 bg-accent-teal text-background-deep rounded-lg font-medium hover:bg-accent-teal/90 transition-colors">
                Save
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Cost Limits */}
      <div className="bg-background-mid rounded-lg border border-background-surface">
        <div className="p-4 border-b border-background-surface">
          <h2 className="text-lg font-semibold flex items-center gap-2">
            <DollarSign className="w-5 h-5 text-accent-amber" />
            Cost Management
          </h2>
        </div>
        <div className="p-4 space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm text-gray-400 mb-1">Daily Limit (USD)</label>
              <input
                type="number"
                defaultValue={10}
                step={0.5}
                min={0}
                className="w-full bg-background-surface border border-background-surface rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-accent-teal"
              />
            </div>
            <div>
              <label className="block text-sm text-gray-400 mb-1">Alert Threshold (%)</label>
              <input
                type="number"
                defaultValue={80}
                min={0}
                max={100}
                className="w-full bg-background-surface border border-background-surface rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-accent-teal"
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

function HealthItem({
  icon: Icon,
  label,
  status,
  detail,
}: {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  status: boolean;
  detail: string;
}) {
  return (
    <div className="flex items-center gap-3 p-3 rounded-lg bg-background-surface">
      <Icon className="w-5 h-5 text-gray-400" />
      <div className="flex-1">
        <p className="font-medium">{label}</p>
        <p className="text-sm text-gray-400">{detail}</p>
      </div>
      {status ? (
        <CheckCircle2 className="w-5 h-5 text-accent-teal" />
      ) : (
        <XCircle className="w-5 h-5 text-accent-magenta" />
      )}
    </div>
  );
}

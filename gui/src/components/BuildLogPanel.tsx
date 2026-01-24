import { useState } from 'react';
import { ChevronDown, ChevronUp, CheckCircle, XCircle, FileCode, Info, Trash2 } from 'lucide-react';
import { BuildLog } from '../hooks/useAutoBuild';

interface BuildLogPanelProps {
  logs: BuildLog[];
  onClear: () => void;
  isProcessing: boolean;
}

export default function BuildLogPanel({ logs, onClear, isProcessing }: BuildLogPanelProps) {
  const [expanded, setExpanded] = useState(true);

  if (logs.length === 0) {
    return null;
  }

  const typeIcons = {
    info: <Info className="w-3.5 h-3.5 text-gray-400" />,
    success: <CheckCircle className="w-3.5 h-3.5 text-accent-teal" />,
    error: <XCircle className="w-3.5 h-3.5 text-red-400" />,
    file: <FileCode className="w-3.5 h-3.5 text-accent-amber" />,
  };

  const formatTime = (date: Date) => {
    return date.toLocaleTimeString(undefined, {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  };

  return (
    <div className="border-t border-background-surface bg-background-mid">
      {/* Header */}
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full flex items-center justify-between px-4 py-2 hover:bg-background-surface/50 transition-colors"
      >
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-gray-300">Build Log</span>
          <span className="text-xs text-gray-500">({logs.length} entries)</span>
          {isProcessing && (
            <span className="flex items-center gap-1">
              <span className="w-2 h-2 bg-accent-amber rounded-full animate-pulse" />
              <span className="text-xs text-accent-amber">Building...</span>
            </span>
          )}
        </div>
        <div className="flex items-center gap-2">
          {logs.length > 0 && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                onClear();
              }}
              className="p-1 text-gray-500 hover:text-gray-300 transition-colors"
              title="Clear logs"
            >
              <Trash2 className="w-4 h-4" />
            </button>
          )}
          {expanded ? (
            <ChevronDown className="w-4 h-4 text-gray-400" />
          ) : (
            <ChevronUp className="w-4 h-4 text-gray-400" />
          )}
        </div>
      </button>

      {/* Log entries */}
      {expanded && (
        <div className="max-h-48 overflow-y-auto border-t border-background-surface">
          <div className="divide-y divide-background-surface/50">
            {logs
              .slice()
              .reverse()
              .map((log) => (
                <div
                  key={log.id}
                  className={`flex items-start gap-2 px-4 py-2 text-sm ${
                    log.type === 'error' ? 'bg-red-500/5' : ''
                  }`}
                >
                  <span className="text-xs text-gray-500 font-mono flex-shrink-0 mt-0.5">
                    {formatTime(log.timestamp)}
                  </span>
                  <span className="flex-shrink-0 mt-0.5">{typeIcons[log.type]}</span>
                  <span
                    className={`flex-1 ${
                      log.type === 'error'
                        ? 'text-red-400'
                        : log.type === 'success'
                        ? 'text-accent-teal'
                        : log.type === 'file'
                        ? 'text-gray-400 font-mono text-xs'
                        : 'text-gray-300'
                    }`}
                  >
                    {log.message}
                    {log.details && log.type === 'file' && (
                      <span className="ml-2 text-gray-500">({log.details})</span>
                    )}
                  </span>
                </div>
              ))}
          </div>
        </div>
      )}
    </div>
  );
}

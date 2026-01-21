import { useState } from 'react';
import { useConflicts, type ConflictFile, type ResolutionStrategy } from '../../contexts/ConflictContext';
import { DiffViewer } from './DiffViewer';
import './ConflictPanel.css';

interface ConflictFileItemProps {
  file: ConflictFile;
  isSelected: boolean;
  onSelect: () => void;
  onResolve: (strategy: ResolutionStrategy) => void;
  onAcknowledge: () => void;
}

function ConflictFileItem({ file, isSelected, onSelect, onResolve, onAcknowledge }: ConflictFileItemProps) {
  const statusClass = `conflict-file--${file.status}`;
  const selectedClass = isSelected ? 'conflict-file--selected' : '';

  const statusIcon = file.status === 'modified' ? '~' : file.status === 'deleted' ? '-' : '';
  const statusLabel = file.status === 'modified' ? 'Modified' : file.status === 'deleted' ? 'Deleted' : 'Unchanged';

  return (
    <div className={`conflict-file ${statusClass} ${selectedClass}`}>
      <div className="conflict-file__header" onClick={onSelect}>
        <span className="conflict-file__status-icon">{statusIcon}</span>
        <span className="conflict-file__path">{file.filePath}</span>
        <span className={`conflict-file__status-badge conflict-file__status-badge--${file.status}`}>
          {statusLabel}
        </span>
      </div>

      {isSelected && (
        <div className="conflict-file__actions">
          {file.status === 'modified' && (
            <>
              <button
                className="conflict-file__action conflict-file__action--keep-user"
                onClick={() => onAcknowledge()}
                title="Accept your changes as the new baseline"
              >
                Keep My Changes
              </button>
              <button
                className="conflict-file__action conflict-file__action--keep-generated"
                onClick={() => onResolve('keep-generated')}
                title="Restore the AI-generated version"
              >
                Restore Original
              </button>
            </>
          )}
          {file.status === 'deleted' && (
            <>
              <button
                className="conflict-file__action conflict-file__action--keep-user"
                onClick={() => onAcknowledge()}
                title="Acknowledge deletion and stop tracking"
              >
                Accept Deletion
              </button>
              <button
                className="conflict-file__action conflict-file__action--keep-generated"
                onClick={() => onResolve('keep-generated')}
                title="Restore the deleted file"
              >
                Restore File
              </button>
            </>
          )}
        </div>
      )}
    </div>
  );
}

export function ConflictPanel() {
  const {
    conflicts,
    selectedFile,
    isPanelVisible,
    isLoading,
    error,
    selectFile,
    resolveConflict,
    resolveAllConflicts,
    acknowledgeEdits,
    setPanelVisible,
    checkForConflicts,
  } = useConflicts();

  const [isResolving, setIsResolving] = useState(false);

  if (!isPanelVisible) {
    return null;
  }

  const handleResolve = async (filePath: string, strategy: ResolutionStrategy) => {
    setIsResolving(true);
    try {
      await resolveConflict(filePath, strategy);
    } finally {
      setIsResolving(false);
    }
  };

  const handleAcknowledge = async (filePath: string) => {
    setIsResolving(true);
    try {
      await acknowledgeEdits([filePath]);
    } finally {
      setIsResolving(false);
    }
  };

  const handleResolveAll = async (strategy: ResolutionStrategy) => {
    setIsResolving(true);
    try {
      await resolveAllConflicts(strategy);
    } finally {
      setIsResolving(false);
    }
  };

  const handleAcknowledgeAll = async () => {
    setIsResolving(true);
    try {
      const modifiedPaths = conflicts
        .filter(c => c.status === 'modified' || c.status === 'deleted')
        .map(c => c.filePath);
      await acknowledgeEdits(modifiedPaths);
    } finally {
      setIsResolving(false);
    }
  };

  const modifiedCount = conflicts.filter(c => c.status === 'modified').length;
  const deletedCount = conflicts.filter(c => c.status === 'deleted').length;
  const totalConflicts = modifiedCount + deletedCount;

  return (
    <div className="conflict-panel">
      <div className="conflict-panel__header">
        <div className="conflict-panel__title">
          <h3>Code Changes Detected</h3>
          {totalConflicts > 0 && (
            <span className="conflict-panel__count">{totalConflicts} file{totalConflicts !== 1 ? 's' : ''}</span>
          )}
        </div>
        <button
          className="conflict-panel__close"
          onClick={() => setPanelVisible(false)}
          title="Close panel"
        >
          &times;
        </button>
      </div>

      {error && (
        <div className="conflict-panel__error">
          {error}
        </div>
      )}

      {isLoading ? (
        <div className="conflict-panel__loading">
          <div className="conflict-panel__spinner" />
          <span>Checking for changes...</span>
        </div>
      ) : totalConflicts === 0 ? (
        <div className="conflict-panel__empty">
          <span className="conflict-panel__empty-icon">&#10003;</span>
          <p>No conflicts detected</p>
          <p className="conflict-panel__empty-hint">All generated files are in sync</p>
          <button
            className="conflict-panel__refresh"
            onClick={() => checkForConflicts('current-project')}
          >
            Refresh
          </button>
        </div>
      ) : (
        <>
          <div className="conflict-panel__summary">
            {modifiedCount > 0 && (
              <span className="conflict-panel__summary-item conflict-panel__summary-item--modified">
                {modifiedCount} modified
              </span>
            )}
            {deletedCount > 0 && (
              <span className="conflict-panel__summary-item conflict-panel__summary-item--deleted">
                {deletedCount} deleted
              </span>
            )}
          </div>

          <div className="conflict-panel__content">
            <div className="conflict-panel__file-list">
              {conflicts.map(file => (
                <ConflictFileItem
                  key={file.filePath}
                  file={file}
                  isSelected={selectedFile?.filePath === file.filePath}
                  onSelect={() => selectFile(selectedFile?.filePath === file.filePath ? null : file)}
                  onResolve={(strategy) => handleResolve(file.filePath, strategy)}
                  onAcknowledge={() => handleAcknowledge(file.filePath)}
                />
              ))}
            </div>

            {selectedFile && selectedFile.status === 'modified' && selectedFile.originalContent && selectedFile.currentContent && (
              <div className="conflict-panel__diff-container">
                <DiffViewer
                  originalContent={selectedFile.originalContent}
                  currentContent={selectedFile.currentContent}
                  filePath={selectedFile.filePath}
                />
              </div>
            )}
          </div>

          <div className="conflict-panel__bulk-actions">
            <button
              className="conflict-panel__bulk-action conflict-panel__bulk-action--acknowledge"
              onClick={handleAcknowledgeAll}
              disabled={isResolving || totalConflicts === 0}
            >
              Accept All My Changes
            </button>
            <button
              className="conflict-panel__bulk-action conflict-panel__bulk-action--restore"
              onClick={() => handleResolveAll('keep-generated')}
              disabled={isResolving || totalConflicts === 0}
            >
              Restore All Original
            </button>
          </div>
        </>
      )}
    </div>
  );
}

export default ConflictPanel;

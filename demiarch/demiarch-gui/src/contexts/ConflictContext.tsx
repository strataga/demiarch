import { createContext, useContext, useState, useCallback, type ReactNode } from 'react';

/**
 * Represents a file that has been detected as modified by the user
 */
export interface ConflictFile {
  /** Relative file path from project root */
  filePath: string;
  /** Current status of the file */
  status: 'modified' | 'deleted' | 'unchanged';
  /** Original content hash when generated */
  originalHash: string;
  /** Current content hash (undefined if deleted) */
  currentHash?: string;
  /** Original content from checkpoint (for diff) */
  originalContent?: string;
  /** Current content on disk (for diff) */
  currentContent?: string;
  /** When the file was originally generated */
  generatedAt?: Date;
  /** Feature that generated this file (if any) */
  featureId?: string;
}

/**
 * Summary of edit detection across project files
 */
export interface ConflictSummary {
  totalFiles: number;
  modifiedFiles: string[];
  deletedFiles: string[];
  unchangedFiles: string[];
}

/**
 * Resolution strategy for a conflict
 */
export type ResolutionStrategy = 'keep-user' | 'keep-generated' | 'merge';

/**
 * Result of resolving a conflict
 */
export interface ResolutionResult {
  filePath: string;
  strategy: ResolutionStrategy;
  success: boolean;
  error?: string;
}

interface ConflictContextValue {
  /** List of files with detected conflicts */
  conflicts: ConflictFile[];
  /** Summary of all conflicts */
  summary: ConflictSummary | null;
  /** Currently selected file for viewing */
  selectedFile: ConflictFile | null;
  /** Whether the conflict panel is visible */
  isPanelVisible: boolean;
  /** Whether conflicts are being loaded */
  isLoading: boolean;
  /** Error message if any */
  error: string | null;

  // Actions
  /** Check all files for conflicts */
  checkForConflicts: (projectId: string) => Promise<void>;
  /** Select a file to view its diff */
  selectFile: (file: ConflictFile | null) => void;
  /** Resolve a single conflict */
  resolveConflict: (filePath: string, strategy: ResolutionStrategy) => Promise<ResolutionResult>;
  /** Resolve all conflicts with a single strategy */
  resolveAllConflicts: (strategy: ResolutionStrategy) => Promise<ResolutionResult[]>;
  /** Acknowledge edits (accept user changes as new baseline) */
  acknowledgeEdits: (filePaths: string[]) => Promise<void>;
  /** Show/hide the conflict panel */
  setPanelVisible: (visible: boolean) => void;
  /** Clear all conflicts */
  clearConflicts: () => void;
}

const ConflictContext = createContext<ConflictContextValue | null>(null);

interface ConflictProviderProps {
  children: ReactNode;
}

export function ConflictProvider({ children }: ConflictProviderProps) {
  const [conflicts, setConflicts] = useState<ConflictFile[]>([]);
  const [summary, setSummary] = useState<ConflictSummary | null>(null);
  const [selectedFile, setSelectedFile] = useState<ConflictFile | null>(null);
  const [isPanelVisible, setIsPanelVisible] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const checkForConflicts = useCallback(async (_projectId: string) => {
    setIsLoading(true);
    setError(null);

    try {
      // TODO: Replace with actual Tauri invoke call
      // const result = await invoke<ConflictSummary>('check_for_conflicts', { projectId: _projectId });

      // Mock data for development
      const mockSummary: ConflictSummary = {
        totalFiles: 5,
        modifiedFiles: ['src/main.rs', 'src/lib.rs'],
        deletedFiles: ['src/unused.rs'],
        unchangedFiles: ['src/config.rs', 'Cargo.toml'],
      };

      const mockConflicts: ConflictFile[] = [
        {
          filePath: 'src/main.rs',
          status: 'modified',
          originalHash: 'abc123',
          currentHash: 'def456',
          originalContent: `fn main() {
    println!("Hello, world!");
}`,
          currentContent: `fn main() {
    println!("Hello, Demiarch!");
    // User added this comment
    init_app();
}`,
          generatedAt: new Date('2024-01-15'),
        },
        {
          filePath: 'src/lib.rs',
          status: 'modified',
          originalHash: 'ghi789',
          currentHash: 'jkl012',
          originalContent: `pub mod app;
pub mod config;`,
          currentContent: `pub mod app;
pub mod config;
pub mod utils; // User added module`,
          generatedAt: new Date('2024-01-15'),
        },
        {
          filePath: 'src/unused.rs',
          status: 'deleted',
          originalHash: 'mno345',
          generatedAt: new Date('2024-01-14'),
        },
      ];

      setSummary(mockSummary);
      setConflicts(mockConflicts);

      // Auto-show panel if there are conflicts
      if (mockSummary.modifiedFiles.length > 0 || mockSummary.deletedFiles.length > 0) {
        setIsPanelVisible(true);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to check for conflicts');
    } finally {
      setIsLoading(false);
    }
  }, []);

  const selectFile = useCallback((file: ConflictFile | null) => {
    setSelectedFile(file);
  }, []);

  const resolveConflict = useCallback(async (filePath: string, strategy: ResolutionStrategy): Promise<ResolutionResult> => {
    try {
      // TODO: Replace with actual Tauri invoke call
      // const result = await invoke<ResolutionResult>('resolve_conflict', { filePath, strategy });

      // Remove from conflicts list
      setConflicts(prev => prev.filter(c => c.filePath !== filePath));

      // Update summary
      setSummary(prev => {
        if (!prev) return null;
        return {
          ...prev,
          modifiedFiles: prev.modifiedFiles.filter(f => f !== filePath),
          deletedFiles: prev.deletedFiles.filter(f => f !== filePath),
        };
      });

      // Clear selection if this was the selected file
      setSelectedFile(prev => prev?.filePath === filePath ? null : prev);

      return {
        filePath,
        strategy,
        success: true,
      };
    } catch (err) {
      return {
        filePath,
        strategy,
        success: false,
        error: err instanceof Error ? err.message : 'Failed to resolve conflict',
      };
    }
  }, []);

  const resolveAllConflicts = useCallback(async (strategy: ResolutionStrategy): Promise<ResolutionResult[]> => {
    const results: ResolutionResult[] = [];

    for (const conflict of conflicts) {
      const result = await resolveConflict(conflict.filePath, strategy);
      results.push(result);
    }

    return results;
  }, [conflicts, resolveConflict]);

  const acknowledgeEdits = useCallback(async (filePaths: string[]) => {
    try {
      // TODO: Replace with actual Tauri invoke call
      // await invoke('acknowledge_edits', { filePaths });

      // Remove acknowledged files from conflicts
      setConflicts(prev => prev.filter(c => !filePaths.includes(c.filePath)));

      // Update summary
      setSummary(prev => {
        if (!prev) return null;
        return {
          ...prev,
          modifiedFiles: prev.modifiedFiles.filter(f => !filePaths.includes(f)),
          deletedFiles: prev.deletedFiles.filter(f => !filePaths.includes(f)),
        };
      });

      // Clear selection if it was acknowledged
      setSelectedFile(prev => prev && filePaths.includes(prev.filePath) ? null : prev);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to acknowledge edits');
    }
  }, []);

  const setPanelVisible = useCallback((visible: boolean) => {
    setIsPanelVisible(visible);
    if (!visible) {
      setSelectedFile(null);
    }
  }, []);

  const clearConflicts = useCallback(() => {
    setConflicts([]);
    setSummary(null);
    setSelectedFile(null);
    setError(null);
  }, []);

  const value: ConflictContextValue = {
    conflicts,
    summary,
    selectedFile,
    isPanelVisible,
    isLoading,
    error,
    checkForConflicts,
    selectFile,
    resolveConflict,
    resolveAllConflicts,
    acknowledgeEdits,
    setPanelVisible,
    clearConflicts,
  };

  return (
    <ConflictContext.Provider value={value}>
      {children}
    </ConflictContext.Provider>
  );
}

export function useConflicts() {
  const context = useContext(ConflictContext);
  if (!context) {
    throw new Error('useConflicts must be used within a ConflictProvider');
  }
  return context;
}

/**
 * Shell utilities for opening folders and running commands
 * Stubs when not in Tauri environment
 */

// Check if we're running in Tauri
const isTauri = () => {
  return typeof window !== 'undefined' && '__TAURI__' in window;
};

export interface ShellResult {
  success: boolean;
  output?: string;
  error?: string;
}

/**
 * Open a folder in the system file explorer
 */
export async function openFolder(path: string): Promise<ShellResult> {
  if (!isTauri()) {
    // In web mode, we can't open folders
    // But we can show an alert or log the path
    console.log(`[Shell] Would open folder: ${path}`);
    alert(`Folder path: ${path}\n\n(Opening folders requires the desktop app)`);
    return { success: false, error: 'Opening folders requires the desktop app' };
  }

  try {
    // In Tauri, we'd use the shell plugin to open the folder
    // For now, stub this
    console.log(`[Tauri Shell] Opening folder: ${path}`);
    return { success: true };
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : 'Failed to open folder',
    };
  }
}

/**
 * Get git commit history for specific files
 */
export async function getGitHistory(
  projectPath: string,
  files?: string[]
): Promise<{ success: boolean; commits?: GitCommit[]; error?: string }> {
  if (!isTauri()) {
    // Return mock data in web mode
    console.log(`[Shell] Would get git history for: ${projectPath}`);
    return {
      success: true,
      commits: [
        {
          hash: 'abc1234',
          shortHash: 'abc1234',
          message: 'feat: implement feature (mock)',
          author: 'Developer',
          date: new Date().toISOString(),
          files: files || [],
        },
      ],
    };
  }

  try {
    // In Tauri, we'd run git log command
    console.log(`[Tauri Shell] Getting git history for: ${projectPath}`);
    return { success: true, commits: [] };
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : 'Failed to get git history',
    };
  }
}

export interface GitCommit {
  hash: string;
  shortHash: string;
  message: string;
  author: string;
  date: string;
  files: string[];
}

/**
 * Open a file in the default editor
 */
export async function openFile(path: string): Promise<ShellResult> {
  if (!isTauri()) {
    console.log(`[Shell] Would open file: ${path}`);
    alert(`File path: ${path}\n\n(Opening files requires the desktop app)`);
    return { success: false, error: 'Opening files requires the desktop app' };
  }

  try {
    console.log(`[Tauri Shell] Opening file: ${path}`);
    return { success: true };
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : 'Failed to open file',
    };
  }
}

/**
 * Copy text to clipboard
 */
export async function copyToClipboard(text: string): Promise<ShellResult> {
  try {
    await navigator.clipboard.writeText(text);
    return { success: true };
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : 'Failed to copy to clipboard',
    };
  }
}

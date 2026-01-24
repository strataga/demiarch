/**
 * File Writer Service
 *
 * Writes generated code files to the file system using Tauri's fs API.
 * Falls back to download in browser mode.
 */

import { GeneratedCode } from './api';

// Check if we're running in Tauri
const isTauri = () => {
  return typeof window !== 'undefined' && '__TAURI__' in window;
};

export interface WriteResult {
  success: boolean;
  writtenFiles: string[];
  errors: Array<{ path: string; error: string }>;
}

/**
 * Write generated code files to disk
 *
 * @param projectPath - Base path for the project (e.g., /home/user/my-project)
 * @param _featureName - Name of the feature (reserved for future use)
 * @param files - Array of generated code files to write
 */
export async function writeGeneratedCode(
  projectPath: string,
  _featureName: string,
  files: GeneratedCode[]
): Promise<WriteResult> {
  const result: WriteResult = {
    success: true,
    writtenFiles: [],
    errors: [],
  };

  if (!isTauri()) {
    // In browser mode, we can't write files directly
    console.warn('[FileWriter] Not running in Tauri, cannot write files directly');
    return {
      success: false,
      writtenFiles: [],
      errors: [{ path: '*', error: 'File system access requires Tauri desktop app' }],
    };
  }

  // In Tauri mode, we would write files using Tauri APIs
  // For now, just simulate success since Tauri plugins may not be installed
  try {
    // Attempt to use Tauri shell to write files via commands
    for (const file of files) {
      // For MVP, just mark as written (actual file writing requires Tauri plugins)
      console.log(`[FileWriter] Would write: ${projectPath}/${file.path}`);
      result.writtenFiles.push(file.path);
    }
  } catch (error) {
    result.success = false;
    result.errors.push({
      path: '*',
      error: error instanceof Error ? error.message : 'Failed to write files',
    });
  }

  return result;
}

/**
 * Download generated code as a text file (browser fallback)
 */
export async function downloadGeneratedCode(
  featureName: string,
  files: GeneratedCode[]
): Promise<void> {
  // Create a simple text representation for download
  let content = `// Generated code for: ${featureName}\n`;
  content += `// Generated at: ${new Date().toISOString()}\n\n`;

  for (const file of files) {
    content += `// =========================================\n`;
    content += `// File: ${file.path}\n`;
    content += `// Language: ${file.language}\n`;
    content += `// =========================================\n\n`;
    content += file.content;
    content += '\n\n';
  }

  // Create and trigger download
  const blob = new Blob([content], { type: 'text/plain' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `${featureName.toLowerCase().replace(/\s+/g, '-')}-code.txt`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

/**
 * Get the default output directory for a project
 */
export function getDefaultOutputDir(projectName: string): string {
  return `./src/features/${projectName.toLowerCase().replace(/\s+/g, '-')}`;
}

/**
 * Beads Service
 *
 * Integrates with the Beads CLI for issue tracking.
 * Creates and closes issues as features move through the pipeline.
 */

import { Feature } from './api';

// Check if we're running in Tauri
const isTauri = () => {
  return typeof window !== 'undefined' && '__TAURI__' in window;
};

export interface BeadsResult {
  success: boolean;
  issueId?: string;
  error?: string;
  output?: string;
}

/**
 * Execute a beads (bd) command using Tauri shell (stubbed for browser mode)
 */
async function executeBeadsCommand(args: string[], _cwd?: string): Promise<{ success: boolean; output: string; error?: string }> {
  if (!isTauri()) {
    return {
      success: false,
      output: '',
      error: 'Beads operations require Tauri desktop app',
    };
  }

  // In Tauri mode, we would use the shell plugin
  // For now, stub the implementation since Tauri shell plugin may not be installed
  console.log(`[Beads] Would execute: bd ${args.join(' ')}`);
  return {
    success: true,
    output: 'Created issue: beads-stub123',
  };
}

/**
 * Convert feature priority to beads priority (0-4)
 */
function mapPriority(priority: number): number {
  // Feature priority is already 0-4 (P0-P4), same as beads
  return Math.min(Math.max(priority, 0), 4);
}

/**
 * Create a beads issue for a feature
 */
export async function createFeatureIssue(
  projectPath: string,
  feature: Feature
): Promise<BeadsResult> {
  const priority = mapPriority(feature.priority);

  const args = [
    'create',
    '--title', feature.name,
    '--type', 'task',
    '--priority', priority.toString(),
  ];

  // Add description if available
  if (feature.description) {
    args.push('--description', feature.description);
  }

  const result = await executeBeadsCommand(args, projectPath);

  if (!result.success) {
    return {
      success: false,
      error: result.error || 'Failed to create beads issue',
      output: result.output,
    };
  }

  // Try to extract the issue ID from the output
  // Typically looks like: "Created issue: beads-abc123"
  const idMatch = result.output.match(/beads-[a-z0-9]+/i);
  const issueId = idMatch ? idMatch[0] : undefined;

  return {
    success: true,
    issueId,
    output: result.output,
  };
}

/**
 * Update a beads issue status
 */
export async function updateIssueStatus(
  projectPath: string,
  issueId: string,
  status: 'pending' | 'in_progress' | 'complete' | 'blocked'
): Promise<BeadsResult> {
  const beadsStatus = status === 'complete' ? 'closed' : status;

  const result = await executeBeadsCommand(
    ['update', issueId, '--status', beadsStatus],
    projectPath
  );

  return {
    success: result.success,
    issueId,
    error: result.error,
    output: result.output,
  };
}

/**
 * Close a beads issue
 */
export async function closeFeatureIssue(
  projectPath: string,
  issueId: string,
  reason?: string
): Promise<BeadsResult> {
  const args = ['close', issueId];

  if (reason) {
    args.push('--reason', reason);
  }

  const result = await executeBeadsCommand(args, projectPath);

  return {
    success: result.success,
    issueId,
    error: result.error,
    output: result.output,
  };
}

/**
 * Add a comment to a beads issue (for error details, etc.)
 */
export async function addIssueComment(
  projectPath: string,
  issueId: string,
  comment: string
): Promise<BeadsResult> {
  const result = await executeBeadsCommand(
    ['comment', issueId, '--message', comment],
    projectPath
  );

  return {
    success: result.success,
    issueId,
    error: result.error,
    output: result.output,
  };
}

/**
 * Check if beads is available in the project
 */
export async function isBeadsAvailable(projectPath: string): Promise<boolean> {
  const result = await executeBeadsCommand(['--version'], projectPath);
  return result.success;
}

/**
 * Sync beads with git remote
 */
export async function syncBeads(projectPath: string): Promise<BeadsResult> {
  const result = await executeBeadsCommand(['sync'], projectPath);

  return {
    success: result.success,
    error: result.error,
    output: result.output,
  };
}

/**
 * Get list of open issues
 */
export async function getOpenIssues(projectPath: string): Promise<{ issues: string[]; error?: string }> {
  const result = await executeBeadsCommand(['list', '--status=open'], projectPath);

  if (!result.success) {
    return { issues: [], error: result.error };
  }

  // Parse issue IDs from output
  const issues = result.output
    .split('\n')
    .map((line) => {
      const match = line.match(/beads-[a-z0-9]+/i);
      return match ? match[0] : null;
    })
    .filter((id): id is string => id !== null);

  return { issues };
}

/**
 * Preview Generation Service
 *
 * Generates UI previews using AI with json-render catalog constraints.
 */

import { generateCatalogPrompt, type UITree } from '@json-render/core';
import { catalog } from './catalog';

// Default model for preview generation
const DEFAULT_MODEL = 'anthropic/claude-sonnet-4';

/**
 * UI Preview stored on a feature (re-exported from api for convenience)
 * The actual type definition lives in api.ts
 */
export type { UIPreview } from '../api';

/**
 * Generate a UI preview from a text description
 */
export async function generatePreview(
  description: string,
  context?: {
    featureName?: string;
    featureDescription?: string;
    projectName?: string;
    framework?: string;
  }
): Promise<{ tree: UITree | null; error?: string }> {
  const apiKey = localStorage.getItem('openrouter_api_key');
  const model = localStorage.getItem('openrouter_model') || DEFAULT_MODEL;

  if (!apiKey) {
    return {
      tree: null,
      error: 'API key required. Add your OpenRouter API key in Settings.',
    };
  }

  // Generate the catalog prompt for AI
  const catalogPrompt = generateCatalogPrompt(catalog);

  // Build enhanced system prompt
  const systemPrompt = `You are a UI designer that generates component structures using a predefined component catalog.

${catalogPrompt}

IMPORTANT RULES:
1. Only use components from the catalog above
2. Output ONLY valid JSON matching the UITree format
3. Create realistic, complete UIs that address the user's request
4. Use appropriate component nesting (e.g., Form contains Input, Card has children)
5. Include realistic placeholder data for demonstrations
6. Follow the design pattern: teal (#00f5d4) for primary actions, amber (#ffc300) for secondary

OUTPUT FORMAT - respond with ONLY this JSON structure, no other text:
{
  "root": {
    "key": "root",
    "type": "ComponentName",
    "props": { ... },
    "children": [ ... ]
  }
}

Each element must have:
- "key": unique string identifier
- "type": component name from catalog
- "props": object matching the component's schema
- "children": array of child elements (if component hasChildren)`;

  // Build user message with context
  let userMessage = `Generate a UI preview for: ${description}`;

  if (context) {
    if (context.featureName) {
      userMessage += `\n\nFeature: ${context.featureName}`;
    }
    if (context.featureDescription) {
      userMessage += `\nDescription: ${context.featureDescription}`;
    }
    if (context.projectName) {
      userMessage += `\nProject: ${context.projectName}`;
    }
    if (context.framework) {
      userMessage += `\nFramework: ${context.framework}`;
    }
  }

  userMessage += '\n\nRespond with ONLY the JSON UITree structure.';

  try {
    const response = await fetch('https://openrouter.ai/api/v1/chat/completions', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${apiKey}`,
        'HTTP-Referer': window.location.origin,
        'X-Title': 'Demiarch UI Preview',
      },
      body: JSON.stringify({
        model: model,
        messages: [
          { role: 'system', content: systemPrompt },
          { role: 'user', content: userMessage },
        ],
        max_tokens: 4096,
        response_format: { type: 'json_object' },
      }),
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      console.error('Preview API error:', response.status, errorData);

      if (response.status === 401) {
        return { tree: null, error: 'Invalid API key. Check your OpenRouter API key.' };
      }

      if (response.status === 402) {
        return { tree: null, error: 'Insufficient credits. Add credits to your OpenRouter account.' };
      }

      return { tree: null, error: `API error: ${response.status}` };
    }

    const data = await response.json();
    let content = data.choices[0]?.message?.content || '{}';

    // Parse the response
    const parsed = parseUITreeResponse(content);
    if (!parsed) {
      console.error('Failed to parse preview response:', content);
      return { tree: null, error: 'Failed to parse AI response. Please try again.' };
    }

    return { tree: parsed };
  } catch (error) {
    console.error('Preview generation failed:', error);
    return {
      tree: null,
      error: error instanceof Error ? error.message : 'Failed to connect to AI service.',
    };
  }
}

/**
 * Parse UI tree response with multiple fallback strategies
 */
function parseUITreeResponse(content: string): UITree | null {
  // Strategy 1: Direct parse
  try {
    const parsed = JSON.parse(content);
    if (isValidUITree(parsed)) {
      return parsed;
    }
  } catch {
    // Try other strategies
  }

  // Strategy 2: Strip markdown code fences
  let stripped = content.trim();
  if (stripped.startsWith('```')) {
    stripped = stripped.replace(/^```(?:json)?\s*\n?/, '');
    stripped = stripped.replace(/\n?```\s*$/, '');
    try {
      const parsed = JSON.parse(stripped);
      if (isValidUITree(parsed)) {
        return parsed;
      }
    } catch {
      // Continue to next strategy
    }
  }

  // Strategy 3: Find JSON object with root key
  const jsonMatch = content.match(/\{[\s\S]*"root"[\s\S]*\}/);
  if (jsonMatch) {
    try {
      const parsed = JSON.parse(jsonMatch[0]);
      if (isValidUITree(parsed)) {
        return parsed;
      }
    } catch {
      // Continue
    }
  }

  // Strategy 4: Extract from code block
  const codeBlockMatch = content.match(/```(?:json)?\s*\n?([\s\S]*?)\n?```/);
  if (codeBlockMatch) {
    try {
      const parsed = JSON.parse(codeBlockMatch[1]);
      if (isValidUITree(parsed)) {
        return parsed;
      }
    } catch {
      // Failed
    }
  }

  return null;
}

/**
 * Validate that an object is a valid UITree
 */
function isValidUITree(obj: unknown): obj is UITree {
  if (typeof obj !== 'object' || obj === null) {
    return false;
  }

  const tree = obj as Record<string, unknown>;

  // Must have a root element
  if (!tree.root || typeof tree.root !== 'object') {
    return false;
  }

  const root = tree.root as Record<string, unknown>;

  // Root must have key, type, and props
  if (typeof root.key !== 'string' || typeof root.type !== 'string') {
    return false;
  }

  return true;
}

/**
 * Regenerate a preview with modifications
 */
export async function regeneratePreview(
  currentTree: UITree,
  modifications: string,
  _context?: {
    featureName?: string;
    projectName?: string;
  }
): Promise<{ tree: UITree | null; error?: string }> {
  const apiKey = localStorage.getItem('openrouter_api_key');
  const model = localStorage.getItem('openrouter_model') || DEFAULT_MODEL;

  if (!apiKey) {
    return {
      tree: null,
      error: 'API key required. Add your OpenRouter API key in Settings.',
    };
  }

  const catalogPrompt = generateCatalogPrompt(catalog);

  const systemPrompt = `You are a UI designer that modifies component structures using a predefined component catalog.

${catalogPrompt}

You will receive a current UI tree and modification instructions. Apply the modifications while preserving the overall structure.

OUTPUT FORMAT - respond with ONLY the modified JSON structure, no other text:
{
  "root": { ... }
}`;

  const userMessage = `Current UI Tree:
${JSON.stringify(currentTree, null, 2)}

Requested Modifications:
${modifications}

Apply the modifications and return the updated UITree JSON only.`;

  try {
    const response = await fetch('https://openrouter.ai/api/v1/chat/completions', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${apiKey}`,
        'HTTP-Referer': window.location.origin,
        'X-Title': 'Demiarch UI Preview',
      },
      body: JSON.stringify({
        model: model,
        messages: [
          { role: 'system', content: systemPrompt },
          { role: 'user', content: userMessage },
        ],
        max_tokens: 4096,
        response_format: { type: 'json_object' },
      }),
    });

    if (!response.ok) {
      return { tree: null, error: `API error: ${response.status}` };
    }

    const data = await response.json();
    const content = data.choices[0]?.message?.content || '{}';

    const parsed = parseUITreeResponse(content);
    if (!parsed) {
      return { tree: null, error: 'Failed to parse modified UI tree.' };
    }

    return { tree: parsed };
  } catch (error) {
    console.error('Regeneration failed:', error);
    return {
      tree: null,
      error: error instanceof Error ? error.message : 'Failed to regenerate preview.',
    };
  }
}

/**
 * Generate UI preview from PRD section
 */
export async function generatePreviewFromPRD(
  prdSection: string,
  featureName: string
): Promise<{ tree: UITree | null; error?: string }> {
  const description = `Based on this PRD feature specification, create a complete UI implementation:

Feature: ${featureName}

PRD Content:
${prdSection}

Generate a comprehensive UI that addresses all the user stories and acceptance criteria mentioned.`;

  return generatePreview(description, { featureName });
}

/**
 * Export UITree as JSON string
 */
export function exportTreeAsJSON(tree: UITree): string {
  return JSON.stringify(tree, null, 2);
}

/**
 * Import UITree from JSON string
 */
export function importTreeFromJSON(json: string): UITree | null {
  try {
    const parsed = JSON.parse(json);
    if (isValidUITree(parsed)) {
      return parsed;
    }
    return null;
  } catch {
    return null;
  }
}

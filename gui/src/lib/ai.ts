/**
 * AI Service for PRD Generation
 *
 * Uses OpenRouter API for live conversation to gather requirements and generate PRDs.
 * Supports multiple models through OpenRouter's unified interface.
 */

export interface Message {
  role: 'user' | 'assistant';
  content: string;
}

export interface ConversationState {
  phase: 'discovery' | 'clarification' | 'generation' | 'refinement';
  gatheredInfo: Record<string, string>;
  prd?: string;
}

// Default model - Claude Sonnet via OpenRouter
const DEFAULT_MODEL = 'anthropic/claude-sonnet-4';

/**
 * Build the system prompt for PRD generation
 */
function buildSystemPrompt(): string {
  return `<role>
You are an expert Product Manager and Technical Architect helping users create detailed Product Requirements Documents (PRDs). You have a conversational, friendly style but are thorough and precise.
</role>

<objective>
Guide the user through understanding what they want to build by asking smart, probing questions. Once you have enough information, generate a comprehensive PRD.
</objective>

<conversation_guidelines>
- Start by understanding the core problem or idea
- Ask ONE focused follow-up question at a time
- Dig deeper on vague answers - push for specifics
- After 4-6 exchanges where you've gathered enough info about: problem, users, features, constraints - offer to generate the PRD
- Be conversational but efficient - don't waste the user's time
- Challenge assumptions and identify edge cases
- If the user gives you a lot of info at once, acknowledge it and ask about gaps
</conversation_guidelines>

<question_areas>
Consider asking about (not necessarily in order, adapt to the conversation):
- What specific problem are they solving? Who has this problem?
- Who are the target users? What's their context and technical level?
- What's the core workflow or user journey?
- What are the must-have features for v1 vs nice-to-haves?
- Are there technical constraints, integrations, or platform requirements?
- How will success be measured?
- What's the timeline or urgency?
- Are there competitors or existing solutions to learn from?
</question_areas>

<prd_format>
When you have enough information and are ready to generate the PRD, create it in this format:

# Product Requirements Document: [Project Name]

## Executive Summary
2-3 sentences on what we're building and why.

## Problem Statement
- **The Problem**: Specific pain point
- **Who Has It**: Target users
- **Current Alternatives**: How they solve it today
- **Why Now**: Why this solution, why now

## Target Users
### Primary Persona
- Description, goals, pain points, technical level

## Goals & Success Metrics
| Metric | Target | How to Measure |
|--------|--------|----------------|
| ... | ... | ... |

## Core Features (MVP)
For each feature:
### Feature Name
- **User Story**: As a [user], I want [action] so that [benefit]
- **Acceptance Criteria**: Specific, testable criteria
- **Priority**: P0/P1/P2

## Technical Considerations
- Architecture approach
- Key integrations
- Data requirements
- Security/compliance needs

## User Flows
Key user journeys, step by step.

## Out of Scope (v1)
What we're explicitly NOT building yet.

## Open Questions
Decisions that still need to be made.

## Milestones
Rough phasing of the work.
</prd_format>

<response_rules>
- Keep responses concise but substantive
- Use markdown formatting
- Bold **key terms** for scannability
- When generating the PRD, be comprehensive and specific based on what you learned
- After generating the PRD, ask if they want to refine any sections
</response_rules>`;
}

/**
 * Process the conversation with OpenRouter API
 */
export async function processConversation(
  userInput: string,
  state: ConversationState,
  history: Message[]
): Promise<{ response: string; newState: ConversationState; prd?: string }> {
  const apiKey = localStorage.getItem('openrouter_api_key');
  const model = localStorage.getItem('openrouter_model') || DEFAULT_MODEL;

  if (!apiKey) {
    return {
      response: `⚠️ **API key required for live conversation.**

Click the **settings icon** (⚙️) in the header to add your OpenRouter API key.

Get one free at [openrouter.ai/keys](https://openrouter.ai/keys)`,
      newState: state,
    };
  }

  const systemPrompt = buildSystemPrompt();

  // Build messages array for OpenRouter (OpenAI-compatible format)
  const messages = [
    { role: 'system' as const, content: systemPrompt },
    ...history.map(m => ({ role: m.role as 'user' | 'assistant', content: m.content })),
    { role: 'user' as const, content: userInput },
  ];

  try {
    const response = await fetch('https://openrouter.ai/api/v1/chat/completions', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${apiKey}`,
        'HTTP-Referer': window.location.origin,
        'X-Title': 'Demiarch PRD Generator',
      },
      body: JSON.stringify({
        model: model,
        messages: messages,
        max_tokens: 4096,
      }),
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      console.error('API error:', response.status, errorData);

      if (response.status === 401) {
        return {
          response: "❌ **Invalid API key.** Please check your OpenRouter API key and try again.",
          newState: state,
        };
      }

      if (response.status === 402) {
        return {
          response: "❌ **Insufficient credits.** Please add credits to your OpenRouter account.",
          newState: state,
        };
      }

      throw new Error(`API error: ${response.status} - ${JSON.stringify(errorData)}`);
    }

    const data = await response.json();
    const aiResponse = data.choices[0]?.message?.content || 'No response received';

    // Detect if the response contains a full PRD
    const isPRD = aiResponse.includes('# Product Requirements Document');

    // Update phase based on conversation progress
    let newPhase = state.phase;
    if (isPRD) {
      newPhase = 'refinement';
    } else if (history.length >= 2) {
      newPhase = 'clarification';
    }

    const newState: ConversationState = {
      ...state,
      phase: newPhase,
      prd: isPRD ? aiResponse : state.prd,
    };

    return {
      response: aiResponse,
      newState,
      prd: isPRD ? aiResponse : undefined,
    };
  } catch (error) {
    console.error('API call failed:', error);
    return {
      response: `❌ **Error connecting to AI.**

${error instanceof Error ? error.message : 'Unknown error'}

Please check your API key and try again.`,
      newState: state,
    };
  }
}

/**
 * Get the initial greeting message
 */
export function getInitialMessage(): string {
  return `👋 **Hey! I'm here to help you create a detailed Product Requirements Document.**

Tell me about what you want to build. What problem are you trying to solve, and for whom?

*The more context you give me upfront, the better questions I can ask.*`;
}

/**
 * Check if API key is configured
 */
export function hasApiKey(): boolean {
  return !!localStorage.getItem('openrouter_api_key');
}

/**
 * Set the API key
 */
export function setApiKey(key: string): void {
  localStorage.setItem('openrouter_api_key', key);
}

/**
 * Clear the API key
 */
export function clearApiKey(): void {
  localStorage.removeItem('openrouter_api_key');
}

/**
 * Get current model
 */
export function getModel(): string {
  return localStorage.getItem('openrouter_model') || DEFAULT_MODEL;
}

/**
 * Set the model
 */
export function setModel(model: string): void {
  localStorage.setItem('openrouter_model', model);
}

/**
 * Available models on OpenRouter
 */
export const AVAILABLE_MODELS = [
  { id: 'anthropic/claude-sonnet-4', name: 'Claude Sonnet 4 (Recommended)' },
  { id: 'anthropic/claude-3.5-sonnet', name: 'Claude 3.5 Sonnet' },
  { id: 'anthropic/claude-3-opus', name: 'Claude 3 Opus' },
  { id: 'openai/gpt-4o', name: 'GPT-4o' },
  { id: 'openai/gpt-4-turbo', name: 'GPT-4 Turbo' },
  { id: 'google/gemini-pro-1.5', name: 'Gemini Pro 1.5' },
  { id: 'meta-llama/llama-3.1-405b-instruct', name: 'Llama 3.1 405B' },
];

/**
 * Extracted feature from PRD
 */
export interface ExtractedFeature {
  name: string;
  description: string;
  userStory: string;
  priority: number; // 0-4 (P0-P4)
  acceptanceCriteria: string[];
}

/**
 * Extract features from a PRD using AI
 */
export async function extractFeaturesFromPRD(prd: string): Promise<{
  features: ExtractedFeature[];
  error?: string;
}> {
  const apiKey = localStorage.getItem('openrouter_api_key');
  const model = localStorage.getItem('openrouter_model') || DEFAULT_MODEL;

  if (!apiKey) {
    return {
      features: [],
      error: 'API key required for feature extraction. Add your OpenRouter API key in Settings.',
    };
  }

  const systemPrompt = `You are a PRD parser that extracts features from Product Requirements Documents.

Your task is to analyze the PRD and extract all features/requirements mentioned.

For each feature, extract:
1. **name**: A short, descriptive feature name (3-6 words)
2. **description**: A 1-2 sentence description of what the feature does
3. **userStory**: The user story if explicitly mentioned, otherwise generate one in format "As a [user], I want [action] so that [benefit]"
4. **priority**: Convert priority to a number:
   - P0 or Critical = 0
   - P1 or High = 1
   - P2 or Medium or default = 2
   - P3 or Low = 3
   - P4 or Backlog = 4
5. **acceptanceCriteria**: List of testable acceptance criteria

Look for features in sections like:
- "Core Features (MVP)"
- "Features"
- Sections with "### Feature Name" headers
- Bullet points describing functionality

Respond with ONLY valid JSON in this exact format, no other text:
{
  "features": [
    {
      "name": "Feature Name",
      "description": "Description of the feature",
      "userStory": "As a user, I want X so that Y",
      "priority": 2,
      "acceptanceCriteria": ["Criteria 1", "Criteria 2"]
    }
  ]
}`;

  try {
    const response = await fetch('https://openrouter.ai/api/v1/chat/completions', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${apiKey}`,
        'HTTP-Referer': window.location.origin,
        'X-Title': 'Demiarch Feature Extractor',
      },
      body: JSON.stringify({
        model: model,
        messages: [
          { role: 'system', content: systemPrompt },
          { role: 'user', content: `Extract features from this PRD:\n\n${prd}` },
        ],
        max_tokens: 4096,
        response_format: { type: 'json_object' },
      }),
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      console.error('API error:', response.status, errorData);

      if (response.status === 401) {
        return { features: [], error: 'Invalid API key. Please check your OpenRouter API key.' };
      }

      if (response.status === 402) {
        return { features: [], error: 'Insufficient credits. Please add credits to your OpenRouter account.' };
      }

      return { features: [], error: `API error: ${response.status}` };
    }

    const data = await response.json();
    let content = data.choices[0]?.message?.content || '{}';

    // Strip markdown code fences if present
    content = content.trim();
    if (content.startsWith('```')) {
      // Remove opening fence (```json or ```)
      content = content.replace(/^```(?:json)?\s*\n?/, '');
      // Remove closing fence
      content = content.replace(/\n?```\s*$/, '');
    }

    try {
      const parsed = JSON.parse(content);
      return { features: parsed.features || [] };
    } catch (parseError) {
      console.error('Failed to parse AI response:', parseError, content);
      return { features: [], error: 'Failed to parse AI response. Please try again.' };
    }
  } catch (error) {
    console.error('Feature extraction failed:', error);
    return {
      features: [],
      error: error instanceof Error ? error.message : 'Failed to connect to AI service.',
    };
  }
}

/**
 * Extract features from PRD without AI (pattern matching fallback)
 */
export function extractFeaturesFromPRDLocal(prd: string): ExtractedFeature[] {
  const features: ExtractedFeature[] = [];

  // Match ### Feature headers
  const featureHeaderRegex = /### ([^\n]+)/g;
  let match;

  while ((match = featureHeaderRegex.exec(prd)) !== null) {
    const featureName = match[1].trim();
    const startIndex = match.index + match[0].length;

    // Find the next ### header or end of document
    const nextHeaderMatch = /\n### |\n## |\n# /g;
    nextHeaderMatch.lastIndex = startIndex;
    const nextMatch = nextHeaderMatch.exec(prd);
    const endIndex = nextMatch ? nextMatch.index : prd.length;

    const featureContent = prd.slice(startIndex, endIndex);

    // Extract user story
    const userStoryMatch = featureContent.match(/\*\*User Story\*\*:\s*(.+)/i);
    const userStory = userStoryMatch ? userStoryMatch[1].trim() : '';

    // Extract priority
    const priorityMatch = featureContent.match(/\*\*Priority\*\*:\s*(P[0-4]|Critical|High|Medium|Low|Backlog)/i);
    let priority = 2; // default to medium
    if (priorityMatch) {
      const p = priorityMatch[1].toLowerCase();
      if (p === 'p0' || p === 'critical') priority = 0;
      else if (p === 'p1' || p === 'high') priority = 1;
      else if (p === 'p2' || p === 'medium') priority = 2;
      else if (p === 'p3' || p === 'low') priority = 3;
      else if (p === 'p4' || p === 'backlog') priority = 4;
    }

    // Extract acceptance criteria
    const criteriaMatch = featureContent.match(/\*\*Acceptance Criteria\*\*:?\s*([\s\S]*?)(?=\n\*\*|$)/i);
    const acceptanceCriteria: string[] = [];
    if (criteriaMatch) {
      const criteriaText = criteriaMatch[1];
      const bulletPoints = criteriaText.match(/[-•]\s*(.+)/g);
      if (bulletPoints) {
        bulletPoints.forEach((bp) => {
          acceptanceCriteria.push(bp.replace(/^[-•]\s*/, '').trim());
        });
      }
    }

    // Get first paragraph as description
    const descLines = featureContent.split('\n').filter((l) => l.trim() && !l.startsWith('-') && !l.startsWith('*'));
    const description = descLines[0]?.trim() || '';

    features.push({
      name: featureName,
      description,
      userStory: userStory || `As a user, I want ${featureName.toLowerCase()}`,
      priority,
      acceptanceCriteria,
    });
  }

  return features;
}

/**
 * Generated code structure
 */
export interface GeneratedFile {
  path: string;
  content: string;
  language: string;
}

export interface GeneratedFeatureCode {
  featureId: string;
  featureName: string;
  files: GeneratedFile[];
}

// Dependency interface for AI response
export interface AIDependency {
  name: string;
  version?: string;
  type: 'npm' | 'pip' | 'gem' | 'cargo' | 'other';
  dev?: boolean;
  reason: string;
}

// Setup requirement interface for AI response
export interface AISetupRequirement {
  step: string;
  command?: string;
  description: string;
  type: 'install' | 'config' | 'env' | 'migration' | 'other';
}

// Full feature implementation result
export interface FeatureImplementationResult {
  files: GeneratedFile[];
  dependencies: AIDependency[];
  setup: AISetupRequirement[];
  error?: string;
}

/**
 * Generate implementation code for a single feature using AI
 */
export async function generateSingleFeatureCode(
  feature: { id: string; name: string; description: string | null },
  projectName: string,
  framework: string
): Promise<FeatureImplementationResult> {
  const apiKey = localStorage.getItem('openrouter_api_key');
  const model = localStorage.getItem('openrouter_model') || DEFAULT_MODEL;

  if (!apiKey) {
    return {
      files: [],
      dependencies: [],
      setup: [],
      error: 'API key required for code generation. Add your OpenRouter API key in Settings.',
    };
  }

  const systemPrompt = `You are an expert software developer that ONLY outputs valid JSON. Generate production-ready code for the given feature.

Project: ${projectName}
Framework: ${framework}

Generate the necessary files with:
1. Clean, well-structured code following best practices
2. Proper TypeScript types (if TypeScript project)
3. Modern React patterns (hooks, functional components) if React
4. Tailwind CSS for styling if applicable
5. Proper error handling

Also identify any npm dependencies needed and setup steps required.

YOU MUST RESPOND WITH ONLY A VALID JSON OBJECT - NO MARKDOWN, NO EXPLANATORY TEXT, JUST THE JSON:
{
  "files": [
    {
      "path": "src/components/FeatureName.tsx",
      "content": "// Full file content here with proper escaping",
      "language": "typescript"
    }
  ],
  "dependencies": [
    {
      "name": "package-name",
      "version": "^1.0.0",
      "type": "npm",
      "dev": false,
      "reason": "Brief explanation"
    }
  ],
  "setup": [
    {
      "step": "Install dependencies",
      "command": "npm install package-name",
      "description": "Install required packages",
      "type": "install"
    }
  ]
}

CRITICAL:
- Output ONLY the JSON object, nothing else
- Ensure all strings in code content are properly escaped for JSON
- If no dependencies or setup needed, use empty arrays []
- The JSON must be parseable by JSON.parse()`;

  const featureDescription = `Feature: ${feature.name}\nDescription: ${feature.description || 'No description provided'}`;

  try {
    const response = await fetch('https://openrouter.ai/api/v1/chat/completions', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${apiKey}`,
        'HTTP-Referer': window.location.origin,
        'X-Title': 'Demiarch Auto Build',
      },
      body: JSON.stringify({
        model: model,
        messages: [
          { role: 'system', content: systemPrompt },
          { role: 'user', content: `Generate code for this feature:\n\n${featureDescription}\n\nRespond with ONLY valid JSON.` },
        ],
        max_tokens: 8192,
        response_format: { type: 'json_object' },
      }),
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      console.error('API error:', response.status, errorData);

      if (response.status === 401) {
        return { files: [], dependencies: [], setup: [], error: 'Invalid API key. Please check your OpenRouter API key.' };
      }

      if (response.status === 402) {
        return { files: [], dependencies: [], setup: [], error: 'Insufficient credits. Please add credits to your OpenRouter account.' };
      }

      return { files: [], dependencies: [], setup: [], error: `API error: ${response.status}` };
    }

    const data = await response.json();
    const rawContent = data.choices[0]?.message?.content || '{}';

    // Try multiple strategies to extract JSON
    let content = rawContent.trim();
    let parsed: { files?: GeneratedFile[]; dependencies?: AIDependency[]; setup?: AISetupRequirement[] } | null = null;

    // Strategy 1: Direct parse (already valid JSON)
    try {
      parsed = JSON.parse(content);
    } catch {
      // Not valid JSON, try other strategies
    }

    // Strategy 2: Strip markdown code fences
    if (!parsed) {
      let stripped = content;
      if (stripped.startsWith('```')) {
        stripped = stripped.replace(/^```(?:json)?\s*\n?/, '');
        stripped = stripped.replace(/\n?```\s*$/, '');
      }
      try {
        parsed = JSON.parse(stripped);
      } catch {
        // Still not valid JSON
      }
    }

    // Strategy 3: Find JSON object in the content
    if (!parsed) {
      const jsonMatch = content.match(/\{[\s\S]*"files"[\s\S]*\}/);
      if (jsonMatch) {
        try {
          parsed = JSON.parse(jsonMatch[0]);
        } catch {
          // Couldn't parse the extracted JSON
        }
      }
    }

    // Strategy 4: Extract JSON from code block anywhere in content
    if (!parsed) {
      const codeBlockMatch = content.match(/```(?:json)?\s*\n?([\s\S]*?)\n?```/);
      if (codeBlockMatch) {
        try {
          parsed = JSON.parse(codeBlockMatch[1]);
        } catch {
          // Couldn't parse code block content
        }
      }
    }

    if (parsed && (parsed.files || parsed.dependencies || parsed.setup)) {
      return {
        files: parsed.files || [],
        dependencies: parsed.dependencies || [],
        setup: parsed.setup || [],
      };
    }

    console.error('Failed to parse AI response. Raw content:', rawContent);
    return { files: [], dependencies: [], setup: [], error: 'Failed to parse AI response. The AI may have returned invalid JSON. Please try again.' };
  } catch (error) {
    console.error('Code generation failed:', error);
    return {
      files: [],
      dependencies: [],
      setup: [],
      error: error instanceof Error ? error.message : 'Failed to connect to AI service.',
    };
  }
}

/**
 * Generate implementation code for features using AI
 */
export async function generateFeatureCode(
  features: Array<{ id: string; name: string; description: string | null }>,
  projectName: string,
  framework: string
): Promise<{ code: GeneratedFeatureCode[]; error?: string }> {
  const apiKey = localStorage.getItem('openrouter_api_key');
  const model = localStorage.getItem('openrouter_model') || DEFAULT_MODEL;

  if (!apiKey) {
    return {
      code: [],
      error: 'API key required for code generation. Add your OpenRouter API key in Settings.',
    };
  }

  const systemPrompt = `You are an expert software developer. Generate production-ready code for the given features.

Project: ${projectName}
Framework: ${framework}

For each feature, generate the necessary files with:
1. Clean, well-structured code following best practices
2. Proper TypeScript types
3. Modern React patterns (hooks, functional components)
4. Tailwind CSS for styling
5. Proper error handling

Respond with ONLY valid JSON in this exact format, no other text:
{
  "features": [
    {
      "featureId": "feature-id-here",
      "featureName": "Feature Name",
      "files": [
        {
          "path": "src/components/FeatureName.tsx",
          "content": "// Full file content here",
          "language": "typescript"
        }
      ]
    }
  ]
}`;

  const featureDescriptions = features
    .map((f) => `- ID: ${f.id}\n  Name: ${f.name}\n  Description: ${f.description || 'No description'}`)
    .join('\n\n');

  try {
    const response = await fetch('https://openrouter.ai/api/v1/chat/completions', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${apiKey}`,
        'HTTP-Referer': window.location.origin,
        'X-Title': 'Demiarch Auto Build',
      },
      body: JSON.stringify({
        model: model,
        messages: [
          { role: 'system', content: systemPrompt },
          { role: 'user', content: `Generate code for these features:\n\n${featureDescriptions}` },
        ],
        max_tokens: 8192,
      }),
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      console.error('API error:', response.status, errorData);

      if (response.status === 401) {
        return { code: [], error: 'Invalid API key. Please check your OpenRouter API key.' };
      }

      if (response.status === 402) {
        return { code: [], error: 'Insufficient credits. Please add credits to your OpenRouter account.' };
      }

      return { code: [], error: `API error: ${response.status}` };
    }

    const data = await response.json();
    let content = data.choices[0]?.message?.content || '{}';

    // Strip markdown code fences if present
    content = content.trim();
    if (content.startsWith('```')) {
      content = content.replace(/^```(?:json)?\s*\n?/, '');
      content = content.replace(/\n?```\s*$/, '');
    }

    try {
      const parsed = JSON.parse(content);
      return { code: parsed.features || [] };
    } catch (parseError) {
      console.error('Failed to parse AI response:', parseError, content);
      return { code: [], error: 'Failed to parse AI response. Please try again.' };
    }
  } catch (error) {
    console.error('Code generation failed:', error);
    return {
      code: [],
      error: error instanceof Error ? error.message : 'Failed to connect to AI service.',
    };
  }
}

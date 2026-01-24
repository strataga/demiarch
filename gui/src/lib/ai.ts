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

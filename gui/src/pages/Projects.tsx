import { useEffect, useState, useRef } from 'react';
import { invoke } from '../lib/api';
import { Link } from 'react-router-dom';
import { Plus, FolderOpen, ArrowRight, X, Send, Bot, User, Sparkles, Settings, Check } from 'lucide-react';
import {
  processConversation,
  getInitialMessage,
  hasApiKey,
  setApiKey,
  ConversationState,
  Message,
} from '../lib/ai';

interface ProjectSummary {
  id: string;
  name: string;
  framework: string;
  status: string;
  feature_count: number;
}

interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
}

export default function Projects() {
  const [projects, setProjects] = useState<ProjectSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showModal, setShowModal] = useState(false);

  async function loadProjects() {
    try {
      setError(null);
      const data = await invoke<ProjectSummary[]>('get_projects');
      setProjects(data);
    } catch (err) {
      console.error('Failed to load projects:', err);
      setError(String(err));
      setProjects([]);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    loadProjects();
  }, []);

  function handleProjectCreated(project: ProjectSummary) {
    setProjects([...projects, project]);
    setShowModal(false);
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="animate-pulse text-accent-teal">Loading projects...</div>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      <div className="flex justify-between items-center">
        <h1 className="text-2xl font-bold">Projects</h1>
        <button
          onClick={() => setShowModal(true)}
          className="flex items-center gap-2 px-4 py-2 bg-accent-teal text-background-deep rounded-lg font-medium hover:bg-accent-teal/90 transition-colors"
        >
          <Plus className="w-5 h-5" />
          New Project
        </button>
      </div>

      {error && (
        <div className="bg-red-500/10 border border-red-500/30 text-red-400 px-4 py-3 rounded-lg">
          <p className="text-sm">{error}</p>
        </div>
      )}

      {projects.length === 0 && !error ? (
        <div className="text-center py-12">
          <FolderOpen className="w-16 h-16 mx-auto text-gray-500 mb-4" />
          <h3 className="text-lg font-medium text-gray-300 mb-2">No projects yet</h3>
          <p className="text-gray-500 mb-4">Describe what you want to build and I'll help create a detailed PRD</p>
          <button
            onClick={() => setShowModal(true)}
            className="inline-flex items-center gap-2 px-4 py-2 bg-accent-teal text-background-deep rounded-lg font-medium hover:bg-accent-teal/90 transition-colors"
          >
            <Sparkles className="w-5 h-5" />
            Start Building
          </button>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {projects.map((project) => (
            <Link
              key={project.id}
              to={`/projects/${project.id}`}
              className="bg-background-mid rounded-lg border border-background-surface p-4 card-hover group"
            >
              <div className="flex items-start justify-between mb-3">
                <div className="w-12 h-12 rounded-lg bg-accent-teal/10 flex items-center justify-center">
                  <FolderOpen className="w-6 h-6 text-accent-teal" />
                </div>
                <StatusBadge status={project.status} />
              </div>
              <h3 className="font-semibold text-lg mb-1">{project.name}</h3>
              <p className="text-sm text-gray-400 mb-3">{project.framework}</p>
              <div className="flex justify-between items-center text-sm">
                <span className="text-gray-500">{project.feature_count} features</span>
                <span className="text-accent-teal opacity-0 group-hover:opacity-100 transition-opacity flex items-center gap-1">
                  Open <ArrowRight className="w-4 h-4" />
                </span>
              </div>
            </Link>
          ))}
        </div>
      )}

      {showModal && (
        <ProjectChatModal
          onClose={() => setShowModal(false)}
          onProjectCreated={handleProjectCreated}
        />
      )}
    </div>
  );
}

interface ProjectChatModalProps {
  onClose: () => void;
  onProjectCreated: (project: ProjectSummary) => void;
}

function ProjectChatModal({ onClose, onProjectCreated }: ProjectChatModalProps) {
  const [messages, setMessages] = useState<ChatMessage[]>(() => {
    const hasKey = hasApiKey();
    return [
      {
        id: '1',
        role: 'assistant',
        content: hasKey
          ? getInitialMessage()
          : `🔑 **First, let's set up your API key.**

Click the **settings icon** (⚙️) in the top right to add your OpenRouter API key.

This enables a live AI conversation to help you create a detailed PRD. Get a free key at [openrouter.ai/keys](https://openrouter.ai/keys)`,
        timestamp: new Date(),
      },
    ];
  });
  const [input, setInput] = useState('');
  const [isThinking, setIsThinking] = useState(false);
  const [conversationState, setConversationState] = useState<ConversationState>({
    phase: 'discovery',
    gatheredInfo: {},
  });
  const [showApiKeyInput, setShowApiKeyInput] = useState(false);
  const [apiKeyInput, setApiKeyInput] = useState('');
  const [isCreating, setIsCreating] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  async function handleSend() {
    if (!input.trim() || isThinking) return;

    const userMessage: ChatMessage = {
      id: Date.now().toString(),
      role: 'user',
      content: input.trim(),
      timestamp: new Date(),
    };

    setMessages((prev) => [...prev, userMessage]);
    setInput('');
    setIsThinking(true);

    try {
      // Convert chat messages to API format
      const history: Message[] = messages.map((m) => ({
        role: m.role,
        content: m.content,
      }));

      const { response, newState, prd } = await processConversation(
        userMessage.content,
        conversationState,
        history
      );

      setConversationState(newState);
      if (prd) {
        setConversationState((prev) => ({ ...prev, prd }));
      }

      const assistantMessage: ChatMessage = {
        id: (Date.now() + 1).toString(),
        role: 'assistant',
        content: response,
        timestamp: new Date(),
      };

      setMessages((prev) => [...prev, assistantMessage]);
    } catch (error) {
      console.error('Error processing conversation:', error);
      const errorMessage: ChatMessage = {
        id: (Date.now() + 1).toString(),
        role: 'assistant',
        content: "I encountered an error processing your response. Let's continue - could you rephrase that?",
        timestamp: new Date(),
      };
      setMessages((prev) => [...prev, errorMessage]);
    } finally {
      setIsThinking(false);
    }
  }

  async function handleCreateProject() {
    if (!conversationState.prd || isCreating) return;

    setIsCreating(true);
    const firstUserMsg = messages.find((m) => m.role === 'user');
    const projectName = extractProjectName(firstUserMsg?.content || 'New Project');

    try {
      const created = await invoke<ProjectSummary>('create_project', {
        name: projectName,
        framework: 'custom',
        prd: conversationState.prd,
        description: firstUserMsg?.content || '',
      });
      onProjectCreated(created);
    } catch (err) {
      console.error('Failed to create project:', err);
      // Show error in chat
      const errorMessage: ChatMessage = {
        id: Date.now().toString(),
        role: 'assistant',
        content: `❌ **Failed to create project.** ${err instanceof Error ? err.message : 'Please try again.'}`,
        timestamp: new Date(),
      };
      setMessages((prev) => [...prev, errorMessage]);
    } finally {
      setIsCreating(false);
    }
  }

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  }

  function handleSaveApiKey() {
    if (apiKeyInput.trim()) {
      setApiKey(apiKeyInput.trim());
      setShowApiKeyInput(false);
      setApiKeyInput('');
      // Reset messages to show proper initial greeting
      setMessages([
        {
          id: '1',
          role: 'assistant',
          content: getInitialMessage(),
          timestamp: new Date(),
        },
      ]);
    }
  }

  const hasKey = hasApiKey();

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
      <div className="bg-background-mid border border-background-surface rounded-lg w-full max-w-3xl h-[85vh] flex flex-col">
        {/* Header */}
        <div className="flex justify-between items-center p-4 border-b border-background-surface">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-full bg-accent-teal/20 flex items-center justify-center">
              <Sparkles className="w-5 h-5 text-accent-teal" />
            </div>
            <div>
              <h2 className="font-semibold">New Project</h2>
              <p className="text-sm text-gray-400">
                {conversationState.phase === 'discovery' && 'Tell me about your project'}
                {conversationState.phase === 'clarification' && 'Gathering requirements...'}
                {conversationState.phase === 'generation' && 'Generating PRD...'}
                {conversationState.phase === 'refinement' && 'PRD ready - refine or create'}
              </p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={() => setShowApiKeyInput(!showApiKeyInput)}
              className={`p-2 rounded-lg transition-colors ${
                hasKey ? 'text-accent-teal bg-accent-teal/10' : 'text-gray-400 hover:text-white'
              }`}
              title={hasKey ? 'API key configured' : 'Configure API key for smarter responses'}
            >
              {hasKey ? <Check className="w-5 h-5" /> : <Settings className="w-5 h-5" />}
            </button>
            <button
              onClick={onClose}
              className="text-gray-400 hover:text-white transition-colors"
            >
              <X className="w-5 h-5" />
            </button>
          </div>
        </div>

        {/* API Key Input */}
        {showApiKeyInput && (
          <div className="p-4 border-b border-background-surface bg-background-surface/50">
            <p className="text-sm text-gray-400 mb-2">
              Add your OpenRouter API key for live AI conversation:
            </p>
            <div className="flex gap-2">
              <input
                type="password"
                value={apiKeyInput}
                onChange={(e) => setApiKeyInput(e.target.value)}
                placeholder="sk-or-..."
                className="flex-1 px-3 py-2 bg-background-deep border border-background-surface rounded-lg text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-accent-teal text-sm"
              />
              <button
                onClick={handleSaveApiKey}
                className="px-4 py-2 bg-accent-teal text-background-deep rounded-lg font-medium text-sm hover:bg-accent-teal/90"
              >
                Save
              </button>
            </div>
          </div>
        )}

        {/* Progress Indicator */}
        <div className="px-4 py-2 border-b border-background-surface">
          <div className="flex gap-2">
            {['discovery', 'clarification', 'generation', 'refinement'].map((phase, i) => {
              const phases = ['discovery', 'clarification', 'generation', 'refinement'];
              const currentIndex = phases.indexOf(conversationState.phase);
              const isActive = i <= currentIndex;
              const isCurrent = phase === conversationState.phase;

              return (
                <div key={phase} className="flex-1">
                  <div
                    className={`h-1 rounded-full transition-colors ${
                      isActive ? 'bg-accent-teal' : 'bg-background-surface'
                    } ${isCurrent ? 'animate-pulse' : ''}`}
                  />
                  <p className={`text-xs mt-1 ${isActive ? 'text-accent-teal' : 'text-gray-500'}`}>
                    {phase.charAt(0).toUpperCase() + phase.slice(1)}
                  </p>
                </div>
              );
            })}
          </div>
        </div>

        {/* Messages */}
        <div className="flex-1 overflow-y-auto p-4 space-y-4">
          {messages.map((message) => (
            <div
              key={message.id}
              className={`flex gap-3 ${message.role === 'user' ? 'flex-row-reverse' : ''}`}
            >
              <div
                className={`w-8 h-8 rounded-full flex items-center justify-center flex-shrink-0 ${
                  message.role === 'assistant' ? 'bg-accent-teal/20' : 'bg-accent-purple/20'
                }`}
              >
                {message.role === 'assistant' ? (
                  <Bot className="w-4 h-4 text-accent-teal" />
                ) : (
                  <User className="w-4 h-4 text-accent-purple" />
                )}
              </div>
              <div
                className={`max-w-[85%] rounded-lg p-3 ${
                  message.role === 'assistant' ? 'bg-background-surface' : 'bg-accent-teal/20'
                }`}
              >
                <div className="text-sm whitespace-pre-wrap leading-relaxed">
                  <MarkdownContent content={message.content} />
                </div>
              </div>
            </div>
          ))}

          {isThinking && (
            <div className="flex gap-3">
              <div className="w-8 h-8 rounded-full bg-accent-teal/20 flex items-center justify-center">
                <Bot className="w-4 h-4 text-accent-teal" />
              </div>
              <div className="bg-background-surface rounded-lg p-3">
                <div className="flex gap-1">
                  <span
                    className="w-2 h-2 bg-accent-teal rounded-full animate-bounce"
                    style={{ animationDelay: '0ms' }}
                  />
                  <span
                    className="w-2 h-2 bg-accent-teal rounded-full animate-bounce"
                    style={{ animationDelay: '150ms' }}
                  />
                  <span
                    className="w-2 h-2 bg-accent-teal rounded-full animate-bounce"
                    style={{ animationDelay: '300ms' }}
                  />
                </div>
              </div>
            </div>
          )}

          <div ref={messagesEndRef} />
        </div>

        {/* Input */}
        <div className="p-4 border-t border-background-surface">
          {conversationState.prd && (
            <div className="mb-3">
              <button
                onClick={handleCreateProject}
                disabled={isCreating}
                className="w-full px-4 py-3 bg-accent-teal text-background-deep rounded-lg font-medium hover:bg-accent-teal/90 transition-colors flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {isCreating ? (
                  <>
                    <div className="w-5 h-5 border-2 border-background-deep border-t-transparent rounded-full animate-spin" />
                    Creating Project...
                  </>
                ) : (
                  <>
                    <Check className="w-5 h-5" />
                    Create Project with this PRD
                  </>
                )}
              </button>
            </div>
          )}
          <div className="flex gap-2">
            <textarea
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={
                conversationState.phase === 'refinement'
                  ? 'Ask for changes or click Create Project...'
                  : 'Type your response...'
              }
              rows={2}
              className="flex-1 px-3 py-2 bg-background-deep border border-background-surface rounded-lg text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-accent-teal focus:border-transparent resize-none"
            />
            <button
              onClick={handleSend}
              disabled={!input.trim() || isThinking}
              className="px-4 py-2 bg-accent-teal text-background-deep rounded-lg font-medium hover:bg-accent-teal/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed self-end"
            >
              <Send className="w-5 h-5" />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

/**
 * Simple markdown renderer for chat messages
 */
function MarkdownContent({ content }: { content: string }) {
  // Simple markdown parsing for display
  const lines = content.split('\n');

  return (
    <>
      {lines.map((line, i) => {
        // Headers
        if (line.startsWith('# ')) {
          return (
            <h1 key={i} className="text-xl font-bold mt-4 mb-2 text-accent-teal">
              {line.slice(2)}
            </h1>
          );
        }
        if (line.startsWith('## ')) {
          return (
            <h2 key={i} className="text-lg font-semibold mt-3 mb-2 text-white">
              {line.slice(3)}
            </h2>
          );
        }
        if (line.startsWith('### ')) {
          return (
            <h3 key={i} className="text-base font-medium mt-2 mb-1 text-gray-200">
              {line.slice(4)}
            </h3>
          );
        }
        // Bold text
        if (line.includes('**')) {
          const parts = line.split(/\*\*([^*]+)\*\*/g);
          return (
            <p key={i} className="mb-1">
              {parts.map((part, j) =>
                j % 2 === 1 ? (
                  <strong key={j} className="font-semibold text-white">
                    {part}
                  </strong>
                ) : (
                  part
                )
              )}
            </p>
          );
        }
        // List items
        if (line.startsWith('- ')) {
          return (
            <li key={i} className="ml-4 list-disc mb-1">
              {line.slice(2)}
            </li>
          );
        }
        if (line.match(/^\d+\. /)) {
          return (
            <li key={i} className="ml-4 list-decimal mb-1">
              {line.replace(/^\d+\. /, '')}
            </li>
          );
        }
        // Checkbox items
        if (line.includes('[ ]')) {
          return (
            <div key={i} className="flex items-center gap-2 ml-4 mb-1">
              <input type="checkbox" disabled className="rounded" />
              <span>{line.replace(/^[\s-]*\[ \] /, '')}</span>
            </div>
          );
        }
        // Table rows (simple)
        if (line.startsWith('|') && line.endsWith('|')) {
          if (line.includes('---')) return null; // Skip separator
          const cells = line.split('|').filter(Boolean);
          return (
            <div key={i} className="flex gap-4 text-sm py-1 border-b border-background-surface/50">
              {cells.map((cell, j) => (
                <span key={j} className="flex-1">
                  {cell.trim()}
                </span>
              ))}
            </div>
          );
        }
        // Horizontal rule
        if (line === '---') {
          return <hr key={i} className="my-4 border-background-surface" />;
        }
        // Empty line
        if (!line.trim()) {
          return <div key={i} className="h-2" />;
        }
        // Regular paragraph
        return (
          <p key={i} className="mb-1">
            {line}
          </p>
        );
      })}
    </>
  );
}

function extractProjectName(input: string): string {
  const words = input
    .replace(/[^\w\s]/g, '')
    .split(/\s+/)
    .filter(
      (w) =>
        w.length > 2 &&
        !['the', 'and', 'for', 'that', 'with', 'want', 'build', 'create', 'make', 'app', 'application', 'need', 'would', 'like'].includes(
          w.toLowerCase()
        )
    )
    .slice(0, 3);

  if (words.length === 0) return 'New Project';
  return words.map((w) => w.charAt(0).toUpperCase() + w.slice(1).toLowerCase()).join(' ');
}

function StatusBadge({ status }: { status: string }) {
  const colors: Record<string, string> = {
    discovery: 'bg-blue-500/20 text-blue-400',
    planning: 'bg-purple-500/20 text-purple-400',
    building: 'bg-accent-amber/20 text-accent-amber',
    complete: 'bg-accent-teal/20 text-accent-teal',
    archived: 'bg-gray-500/20 text-gray-400',
  };

  return (
    <span className={`px-2 py-1 rounded text-xs font-medium ${colors[status] || colors.discovery}`}>
      {status}
    </span>
  );
}

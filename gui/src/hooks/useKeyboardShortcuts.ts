import { useEffect, useCallback } from 'react';

export interface KeyboardShortcut {
  key: string;
  ctrl?: boolean;
  meta?: boolean;
  shift?: boolean;
  alt?: boolean;
  action: () => void;
  description?: string;
}

/**
 * Hook for registering keyboard shortcuts
 */
export function useKeyboardShortcuts(shortcuts: KeyboardShortcut[]) {
  const handleKeyDown = useCallback(
    (event: KeyboardEvent) => {
      // Don't trigger shortcuts when typing in inputs
      const target = event.target as HTMLElement;
      if (
        target.tagName === 'INPUT' ||
        target.tagName === 'TEXTAREA' ||
        target.isContentEditable
      ) {
        // Allow Escape to work in inputs
        if (event.key !== 'Escape') {
          return;
        }
      }

      for (const shortcut of shortcuts) {
        const keyMatch = event.key.toLowerCase() === shortcut.key.toLowerCase();
        const ctrlMatch = !!shortcut.ctrl === (event.ctrlKey || event.metaKey);
        const shiftMatch = !!shortcut.shift === event.shiftKey;
        const altMatch = !!shortcut.alt === event.altKey;

        if (keyMatch && ctrlMatch && shiftMatch && altMatch) {
          event.preventDefault();
          shortcut.action();
          return;
        }
      }
    },
    [shortcuts]
  );

  useEffect(() => {
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);
}

/**
 * Common shortcuts for modals
 */
export function useModalShortcuts(onClose: () => void, onSubmit?: () => void) {
  const shortcuts: KeyboardShortcut[] = [
    {
      key: 'Escape',
      action: onClose,
      description: 'Close modal',
    },
  ];

  if (onSubmit) {
    shortcuts.push({
      key: 'Enter',
      ctrl: true,
      action: onSubmit,
      description: 'Submit',
    });
  }

  useKeyboardShortcuts(shortcuts);
}

/**
 * Hook for Kanban-specific shortcuts
 */
export function useKanbanShortcuts({
  onNewFeature,
  onSearch,
}: {
  onNewFeature: () => void;
  onSearch: () => void;
}) {
  useKeyboardShortcuts([
    {
      key: 'n',
      action: onNewFeature,
      description: 'New feature',
    },
    {
      key: '/',
      action: onSearch,
      description: 'Focus search',
    },
  ]);
}

/**
 * Format shortcut for display
 */
export function formatShortcut(shortcut: KeyboardShortcut): string {
  const parts: string[] = [];

  if (shortcut.ctrl) {
    parts.push(navigator.platform.includes('Mac') ? '\u2318' : 'Ctrl');
  }
  if (shortcut.alt) {
    parts.push(navigator.platform.includes('Mac') ? '\u2325' : 'Alt');
  }
  if (shortcut.shift) {
    parts.push('\u21E7');
  }

  const keyDisplay =
    shortcut.key === 'Escape'
      ? 'Esc'
      : shortcut.key === 'Enter'
      ? '\u21B5'
      : shortcut.key.toUpperCase();

  parts.push(keyDisplay);

  return parts.join(navigator.platform.includes('Mac') ? '' : '+');
}

/**
 * Get all available shortcuts for help display
 */
export function getGlobalShortcuts(): Array<{ shortcut: string; description: string }> {
  return [
    { shortcut: 'n', description: 'New feature (in Kanban)' },
    { shortcut: '/', description: 'Focus search' },
    { shortcut: 'Esc', description: 'Close modal' },
    { shortcut: '\u2318+Enter', description: 'Submit form' },
  ];
}

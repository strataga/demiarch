/**
 * json-render Component Catalog
 *
 * Defines the available UI components for AI-generated previews.
 * Uses Zod schemas to validate and constrain AI outputs.
 */

import { createCatalog } from '@json-render/core';
import { z } from 'zod';

export const catalog = createCatalog({
  components: {
    // Layout Components
    Card: {
      props: z.object({
        title: z.string().describe('Card title'),
        description: z.string().nullable().describe('Optional subtitle or description'),
      }),
      hasChildren: true,
      description: 'A container card with title and optional description. Use for grouping related content.',
    },

    Container: {
      props: z.object({
        className: z.string().optional().describe('Additional Tailwind classes for layout'),
        direction: z.enum(['row', 'col']).default('col').describe('Flex direction'),
        gap: z.enum(['none', 'sm', 'md', 'lg']).default('md').describe('Spacing between children'),
      }),
      hasChildren: true,
      description: 'A flexible container for arranging child elements. Use for layout control.',
    },

    Section: {
      props: z.object({
        title: z.string().describe('Section heading'),
        description: z.string().nullable().describe('Optional section description'),
      }),
      hasChildren: true,
      description: 'A page section with heading. Use to organize page content into logical areas.',
    },

    // Form Components
    Form: {
      props: z.object({
        id: z.string().describe('Unique form identifier'),
        title: z.string().nullable().describe('Optional form title'),
      }),
      hasChildren: true,
      description: 'A form container. Use to wrap input elements that submit together.',
    },

    Input: {
      props: z.object({
        label: z.string().describe('Input label shown to user'),
        name: z.string().describe('Field name for form submission'),
        type: z.enum(['text', 'email', 'password', 'number', 'tel', 'url']).default('text'),
        placeholder: z.string().optional().describe('Placeholder text'),
        required: z.boolean().default(false),
        disabled: z.boolean().default(false),
      }),
      description: 'A text input field. Use for single-line text entry.',
    },

    Textarea: {
      props: z.object({
        label: z.string().describe('Input label'),
        name: z.string().describe('Field name'),
        placeholder: z.string().optional(),
        rows: z.number().default(3).describe('Number of visible rows'),
        required: z.boolean().default(false),
      }),
      description: 'A multi-line text input. Use for longer text content.',
    },

    Select: {
      props: z.object({
        label: z.string().describe('Select label'),
        name: z.string().describe('Field name'),
        options: z.array(
          z.object({
            value: z.string(),
            label: z.string(),
          })
        ).describe('Available options'),
        required: z.boolean().default(false),
      }),
      description: 'A dropdown select input. Use when user chooses from predefined options.',
    },

    Checkbox: {
      props: z.object({
        label: z.string().describe('Checkbox label'),
        name: z.string().describe('Field name'),
        checked: z.boolean().default(false),
      }),
      description: 'A checkbox input. Use for boolean/toggle options.',
    },

    // Display Components
    Button: {
      props: z.object({
        label: z.string().describe('Button text'),
        action: z.string().describe('Action to trigger on click'),
        variant: z.enum(['primary', 'secondary', 'danger', 'ghost']).default('primary'),
        size: z.enum(['sm', 'md', 'lg']).default('md'),
        disabled: z.boolean().default(false),
      }),
      description: 'A clickable button. Use for user actions.',
    },

    Text: {
      props: z.object({
        content: z.string().describe('Text content to display'),
        variant: z.enum(['h1', 'h2', 'h3', 'h4', 'p', 'span', 'label']).default('p'),
        color: z.enum(['default', 'muted', 'accent', 'error', 'success']).default('default'),
      }),
      description: 'Text display element. Use for headings, paragraphs, and labels.',
    },

    Badge: {
      props: z.object({
        label: z.string().describe('Badge text'),
        color: z.enum(['green', 'red', 'yellow', 'blue', 'gray', 'purple']).default('gray'),
      }),
      description: 'A small status badge. Use for tags, labels, and status indicators.',
    },

    Alert: {
      props: z.object({
        title: z.string().describe('Alert title'),
        message: z.string().describe('Alert message body'),
        type: z.enum(['info', 'success', 'warning', 'error']).default('info'),
      }),
      description: 'An alert/notification box. Use for important messages and feedback.',
    },

    // Data Display Components
    Table: {
      props: z.object({
        columns: z.array(
          z.object({
            key: z.string().describe('Data key to display'),
            label: z.string().describe('Column header'),
            width: z.string().optional().describe('Column width (e.g., "100px", "20%")'),
          })
        ),
        data: z
          .array(z.record(z.string(), z.unknown()))
          .optional()
          .describe('Table data rows (optional, can be bound dynamically)'),
        emptyMessage: z.string().default('No data').describe('Message when table is empty'),
      }),
      description: 'A data table. Use for displaying structured data in rows and columns.',
    },

    List: {
      props: z.object({
        items: z.array(z.string()).describe('List items to display'),
        type: z.enum(['bullet', 'number', 'none']).default('bullet'),
      }),
      description: 'A list of items. Use for displaying enumerated content.',
    },

    Stats: {
      props: z.object({
        label: z.string().describe('Metric label'),
        value: z.string().describe('Metric value'),
        change: z.string().optional().describe('Change indicator (e.g., "+12%")'),
        changeType: z.enum(['positive', 'negative', 'neutral']).default('neutral'),
      }),
      description: 'A single stat/metric display. Use for KPIs and dashboard stats.',
    },

    Avatar: {
      props: z.object({
        name: z.string().describe('User name for initials'),
        src: z.string().optional().describe('Image URL'),
        size: z.enum(['sm', 'md', 'lg']).default('md'),
      }),
      description: 'User avatar display. Use for user identification.',
    },

    // Navigation Components
    Link: {
      props: z.object({
        label: z.string().describe('Link text'),
        href: z.string().describe('Link destination'),
        external: z.boolean().default(false).describe('Open in new tab'),
      }),
      description: 'A navigation link. Use for internal or external navigation.',
    },

    Tabs: {
      props: z.object({
        tabs: z.array(
          z.object({
            id: z.string(),
            label: z.string(),
          })
        ).describe('Tab definitions'),
        defaultTab: z.string().optional().describe('Initially selected tab ID'),
      }),
      hasChildren: true,
      description: 'Tab navigation container. Children should be TabPanel components.',
    },

    TabPanel: {
      props: z.object({
        tabId: z.string().describe('ID matching parent Tabs tab definition'),
      }),
      hasChildren: true,
      description: 'Content panel for a tab. Must be child of Tabs component.',
    },

    // Utility Components
    Divider: {
      props: z.object({
        orientation: z.enum(['horizontal', 'vertical']).default('horizontal'),
      }),
      description: 'A visual separator. Use between sections or items.',
    },

    Spacer: {
      props: z.object({
        size: z.enum(['sm', 'md', 'lg', 'xl']).default('md'),
      }),
      description: 'Empty space. Use for adding vertical spacing.',
    },

    Image: {
      props: z.object({
        src: z.string().describe('Image URL or placeholder like "/placeholder-400x300.png"'),
        alt: z.string().describe('Alt text for accessibility'),
        width: z.string().optional().describe('Image width'),
        height: z.string().optional().describe('Image height'),
      }),
      description: 'An image display. Use for visual content.',
    },
  },

  actions: {
    submit: {
      params: z.object({
        formId: z.string().describe('ID of form to submit'),
      }),
      description: 'Submit a form',
    },

    navigate: {
      params: z.object({
        path: z.string().describe('Path or URL to navigate to'),
      }),
      description: 'Navigate to another page or route',
    },

    delete: {
      params: z.object({
        id: z.string().describe('ID of item to delete'),
        confirm: z.boolean().default(true).describe('Show confirmation dialog'),
      }),
      description: 'Delete an item',
    },

    edit: {
      params: z.object({
        id: z.string().describe('ID of item to edit'),
      }),
      description: 'Open item for editing',
    },

    refresh: {
      params: z.object({}),
      description: 'Refresh the current view or data',
    },

    toggleModal: {
      params: z.object({
        modalId: z.string().describe('ID of modal to toggle'),
      }),
      description: 'Open or close a modal dialog',
    },
  },
});

export type Catalog = typeof catalog;

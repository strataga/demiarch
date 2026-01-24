/**
 * json-render Component Registry
 *
 * Maps catalog component names to React implementations with Tailwind styling.
 * All components follow the Demiarch design system.
 */

import React from 'react';
import type { ComponentRenderProps, ComponentRegistry } from '@json-render/react';

// Alias for cleaner code
type Props = ComponentRenderProps;

// Helper to extract props safely with type
function getProps<T extends Record<string, unknown>>(element: { props: Record<string, unknown> }): T {
  return element.props as T;
}

// ============================================================================
// Layout Components
// ============================================================================

function Card({ element, children }: Props) {
  const props = getProps<{ title: string; description: string | null }>(element);
  return (
    <div className="bg-background-surface rounded-lg border border-background-surface/50 p-4">
      <div className="mb-3">
        <h3 className="text-lg font-semibold text-white">{props.title}</h3>
        {props.description && (
          <p className="text-sm text-gray-400 mt-1">{props.description}</p>
        )}
      </div>
      {children && <div>{children}</div>}
    </div>
  );
}

function Container({ element, children }: Props) {
  const props = getProps<{
    className?: string;
    direction?: 'row' | 'col';
    gap?: 'none' | 'sm' | 'md' | 'lg';
  }>(element);

  const gapClasses = {
    none: 'gap-0',
    sm: 'gap-2',
    md: 'gap-4',
    lg: 'gap-6',
  };

  return (
    <div
      className={`flex ${props.direction === 'row' ? 'flex-row' : 'flex-col'} ${
        gapClasses[props.gap || 'md']
      } ${props.className || ''}`}
    >
      {children}
    </div>
  );
}

function Section({ element, children }: Props) {
  const props = getProps<{ title: string; description: string | null }>(element);
  return (
    <section className="py-4">
      <div className="mb-4">
        <h2 className="text-xl font-semibold text-white">{props.title}</h2>
        {props.description && (
          <p className="text-sm text-gray-400 mt-1">{props.description}</p>
        )}
      </div>
      {children}
    </section>
  );
}

// ============================================================================
// Form Components
// ============================================================================

function Form({ element, children }: Props) {
  const props = getProps<{ id: string; title: string | null }>(element);
  return (
    <form
      id={props.id}
      className="space-y-4"
      onSubmit={(e) => e.preventDefault()}
    >
      {props.title && (
        <h3 className="text-lg font-medium text-white mb-4">{props.title}</h3>
      )}
      {children}
    </form>
  );
}

function Input({ element }: Props) {
  const props = getProps<{
    label: string;
    name: string;
    type?: 'text' | 'email' | 'password' | 'number' | 'tel' | 'url';
    placeholder?: string;
    required?: boolean;
    disabled?: boolean;
  }>(element);

  return (
    <div className="space-y-1">
      <label className="block text-sm font-medium text-gray-300">
        {props.label}
        {props.required && <span className="text-red-400 ml-1">*</span>}
      </label>
      <input
        type={props.type || 'text'}
        name={props.name}
        placeholder={props.placeholder}
        required={props.required}
        disabled={props.disabled}
        className="w-full bg-background-mid border border-background-surface rounded-lg px-3 py-2 text-white placeholder-gray-500 focus:outline-none focus:border-accent-teal disabled:opacity-50"
      />
    </div>
  );
}

function Textarea({ element }: Props) {
  const props = getProps<{
    label: string;
    name: string;
    placeholder?: string;
    rows?: number;
    required?: boolean;
  }>(element);

  return (
    <div className="space-y-1">
      <label className="block text-sm font-medium text-gray-300">
        {props.label}
        {props.required && <span className="text-red-400 ml-1">*</span>}
      </label>
      <textarea
        name={props.name}
        placeholder={props.placeholder}
        rows={props.rows || 3}
        required={props.required}
        className="w-full bg-background-mid border border-background-surface rounded-lg px-3 py-2 text-white placeholder-gray-500 focus:outline-none focus:border-accent-teal resize-none"
      />
    </div>
  );
}

function Select({ element }: Props) {
  const props = getProps<{
    label: string;
    name: string;
    options: Array<{ value: string; label: string }>;
    required?: boolean;
  }>(element);

  return (
    <div className="space-y-1">
      <label className="block text-sm font-medium text-gray-300">
        {props.label}
        {props.required && <span className="text-red-400 ml-1">*</span>}
      </label>
      <select
        name={props.name}
        required={props.required}
        className="w-full bg-background-mid border border-background-surface rounded-lg px-3 py-2 text-white focus:outline-none focus:border-accent-teal"
      >
        {props.options.map((opt) => (
          <option key={opt.value} value={opt.value}>
            {opt.label}
          </option>
        ))}
      </select>
    </div>
  );
}

function Checkbox({ element }: Props) {
  const props = getProps<{
    label: string;
    name: string;
    checked?: boolean;
  }>(element);

  return (
    <label className="flex items-center gap-2 cursor-pointer">
      <input
        type="checkbox"
        name={props.name}
        defaultChecked={props.checked}
        className="w-4 h-4 rounded border-background-surface bg-background-mid text-accent-teal focus:ring-accent-teal focus:ring-offset-0"
      />
      <span className="text-sm text-gray-300">{props.label}</span>
    </label>
  );
}

// ============================================================================
// Display Components
// ============================================================================

function Button({ element, onAction }: Props) {
  const props = getProps<{
    label: string;
    action: string;
    variant?: 'primary' | 'secondary' | 'danger' | 'ghost';
    size?: 'sm' | 'md' | 'lg';
    disabled?: boolean;
  }>(element);

  const variantClasses = {
    primary: 'bg-accent-teal text-background-deep hover:bg-accent-teal/90',
    secondary: 'bg-background-surface text-white hover:bg-background-surface/80',
    danger: 'bg-red-500 text-white hover:bg-red-600',
    ghost: 'bg-transparent text-gray-300 hover:bg-background-surface',
  };

  const sizeClasses = {
    sm: 'px-2 py-1 text-sm',
    md: 'px-4 py-2',
    lg: 'px-6 py-3 text-lg',
  };

  const handleClick = () => {
    if (onAction) {
      onAction({ name: props.action });
    }
  };

  return (
    <button
      onClick={handleClick}
      disabled={props.disabled}
      className={`rounded-lg font-medium transition-colors disabled:opacity-50 ${
        variantClasses[props.variant || 'primary']
      } ${sizeClasses[props.size || 'md']}`}
    >
      {props.label}
    </button>
  );
}

function Text({ element }: Props) {
  const props = getProps<{
    content: string;
    variant?: 'h1' | 'h2' | 'h3' | 'h4' | 'p' | 'span' | 'label';
    color?: 'default' | 'muted' | 'accent' | 'error' | 'success';
  }>(element);

  const colorClasses = {
    default: 'text-white',
    muted: 'text-gray-400',
    accent: 'text-accent-teal',
    error: 'text-red-400',
    success: 'text-green-400',
  };

  const variantClasses = {
    h1: 'text-3xl font-bold',
    h2: 'text-2xl font-semibold',
    h3: 'text-xl font-semibold',
    h4: 'text-lg font-medium',
    p: 'text-base',
    span: 'text-base',
    label: 'text-sm font-medium',
  };

  const Tag = props.variant || 'p';
  return (
    <Tag className={`${variantClasses[Tag]} ${colorClasses[props.color || 'default']}`}>
      {props.content}
    </Tag>
  );
}

function Badge({ element }: Props) {
  const props = getProps<{
    label: string;
    color?: 'green' | 'red' | 'yellow' | 'blue' | 'gray' | 'purple';
  }>(element);

  const colorClasses = {
    green: 'bg-green-500/20 text-green-400',
    red: 'bg-red-500/20 text-red-400',
    yellow: 'bg-yellow-500/20 text-yellow-400',
    blue: 'bg-blue-500/20 text-blue-400',
    gray: 'bg-gray-500/20 text-gray-400',
    purple: 'bg-purple-500/20 text-purple-400',
  };

  return (
    <span
      className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${
        colorClasses[props.color || 'gray']
      }`}
    >
      {props.label}
    </span>
  );
}

function Alert({ element }: Props) {
  const props = getProps<{
    title: string;
    message: string;
    type?: 'info' | 'success' | 'warning' | 'error';
  }>(element);

  const typeClasses = {
    info: 'bg-blue-500/10 border-blue-500/30 text-blue-400',
    success: 'bg-green-500/10 border-green-500/30 text-green-400',
    warning: 'bg-yellow-500/10 border-yellow-500/30 text-yellow-400',
    error: 'bg-red-500/10 border-red-500/30 text-red-400',
  };

  return (
    <div className={`p-4 rounded-lg border ${typeClasses[props.type || 'info']}`}>
      <h4 className="font-medium mb-1">{props.title}</h4>
      <p className="text-sm opacity-80">{props.message}</p>
    </div>
  );
}

// ============================================================================
// Data Display Components
// ============================================================================

function Table({ element }: Props) {
  const props = getProps<{
    columns: Array<{ key: string; label: string; width?: string }>;
    data?: Array<Record<string, unknown>>;
    emptyMessage?: string;
  }>(element);

  const data = props.data || [];

  return (
    <div className="overflow-x-auto">
      <table className="w-full">
        <thead>
          <tr className="border-b border-background-surface">
            {props.columns.map((col) => (
              <th
                key={col.key}
                className="px-4 py-3 text-left text-sm font-medium text-gray-400"
                style={col.width ? { width: col.width } : undefined}
              >
                {col.label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {data.length === 0 ? (
            <tr>
              <td
                colSpan={props.columns.length}
                className="px-4 py-8 text-center text-gray-500"
              >
                {props.emptyMessage || 'No data'}
              </td>
            </tr>
          ) : (
            data.map((row, idx) => (
              <tr key={idx} className="border-b border-background-surface/50">
                {props.columns.map((col) => (
                  <td key={col.key} className="px-4 py-3 text-sm text-white">
                    {String(row[col.key] ?? '')}
                  </td>
                ))}
              </tr>
            ))
          )}
        </tbody>
      </table>
    </div>
  );
}

function List({ element }: Props) {
  const props = getProps<{
    items: string[];
    type?: 'bullet' | 'number' | 'none';
  }>(element);

  const Tag = props.type === 'number' ? 'ol' : 'ul';
  const listClass =
    props.type === 'number'
      ? 'list-decimal'
      : props.type === 'bullet'
      ? 'list-disc'
      : 'list-none';

  return (
    <Tag className={`${listClass} pl-5 space-y-1 text-gray-300`}>
      {props.items.map((item, idx) => (
        <li key={idx}>{item}</li>
      ))}
    </Tag>
  );
}

function Stats({ element }: Props) {
  const props = getProps<{
    label: string;
    value: string;
    change?: string;
    changeType?: 'positive' | 'negative' | 'neutral';
  }>(element);

  const changeColorClasses = {
    positive: 'text-green-400',
    negative: 'text-red-400',
    neutral: 'text-gray-400',
  };

  return (
    <div className="bg-background-surface rounded-lg p-4">
      <p className="text-sm text-gray-400 mb-1">{props.label}</p>
      <div className="flex items-baseline gap-2">
        <p className="text-2xl font-bold text-white">{props.value}</p>
        {props.change && (
          <span
            className={`text-sm ${changeColorClasses[props.changeType || 'neutral']}`}
          >
            {props.change}
          </span>
        )}
      </div>
    </div>
  );
}

function Avatar({ element }: Props) {
  const props = getProps<{
    name: string;
    src?: string;
    size?: 'sm' | 'md' | 'lg';
  }>(element);

  const sizeClasses = {
    sm: 'w-8 h-8 text-xs',
    md: 'w-10 h-10 text-sm',
    lg: 'w-14 h-14 text-lg',
  };

  const initials = props.name
    .split(' ')
    .map((n) => n[0])
    .join('')
    .toUpperCase()
    .slice(0, 2);

  if (props.src) {
    return (
      <img
        src={props.src}
        alt={props.name}
        className={`rounded-full object-cover ${sizeClasses[props.size || 'md']}`}
      />
    );
  }

  return (
    <div
      className={`rounded-full bg-accent-teal/20 text-accent-teal flex items-center justify-center font-medium ${
        sizeClasses[props.size || 'md']
      }`}
    >
      {initials}
    </div>
  );
}

// ============================================================================
// Navigation Components
// ============================================================================

function Link({ element }: Props) {
  const props = getProps<{
    label: string;
    href: string;
    external?: boolean;
  }>(element);

  return (
    <a
      href={props.href}
      target={props.external ? '_blank' : undefined}
      rel={props.external ? 'noopener noreferrer' : undefined}
      className="text-accent-teal hover:underline"
    >
      {props.label}
    </a>
  );
}

function Tabs({ element, children }: Props) {
  const props = getProps<{
    tabs: Array<{ id: string; label: string }>;
    defaultTab?: string;
  }>(element);

  const [activeTab, setActiveTab] = React.useState(props.defaultTab || props.tabs[0]?.id);

  return (
    <div>
      <div className="flex border-b border-background-surface">
        {props.tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors ${
              activeTab === tab.id
                ? 'border-accent-teal text-accent-teal'
                : 'border-transparent text-gray-400 hover:text-white'
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>
      <div className="py-4">
        {React.Children.map(children, (child) => {
          if (React.isValidElement(child)) {
            // Type assertion for json-render component props
            const childProps = child.props as { element?: { props?: { tabId?: string } } };
            if (childProps.element?.props?.tabId === activeTab) {
              return child;
            }
          }
          return null;
        })}
      </div>
    </div>
  );
}

function TabPanel({ element, children }: Props) {
  const props = getProps<{ tabId: string }>(element);
  // TabPanel visibility is controlled by parent Tabs
  return <div data-tab-id={props.tabId}>{children}</div>;
}

// ============================================================================
// Utility Components
// ============================================================================

function Divider({ element }: Props) {
  const props = getProps<{ orientation?: 'horizontal' | 'vertical' }>(element);

  if (props.orientation === 'vertical') {
    return <div className="w-px h-full bg-background-surface" />;
  }
  return <div className="h-px w-full bg-background-surface" />;
}

function Spacer({ element }: Props) {
  const props = getProps<{ size?: 'sm' | 'md' | 'lg' | 'xl' }>(element);

  const sizeClasses = {
    sm: 'h-2',
    md: 'h-4',
    lg: 'h-8',
    xl: 'h-12',
  };

  return <div className={sizeClasses[props.size || 'md']} />;
}

function Image({ element }: Props) {
  const props = getProps<{
    src: string;
    alt: string;
    width?: string;
    height?: string;
  }>(element);

  // Handle placeholder images
  const src = props.src.startsWith('/placeholder')
    ? `https://placehold.co/${props.width || '400'}x${props.height || '300'}/253346/00f5d4?text=Preview`
    : props.src;

  return (
    <img
      src={src}
      alt={props.alt}
      style={{
        width: props.width,
        height: props.height,
      }}
      className="rounded-lg object-cover"
    />
  );
}

// ============================================================================
// Registry Export
// ============================================================================

export const registry: ComponentRegistry = {
  Card,
  Container,
  Section,
  Form,
  Input,
  Textarea,
  Select,
  Checkbox,
  Button,
  Text,
  Badge,
  Alert,
  Table,
  List,
  Stats,
  Avatar,
  Link,
  Tabs,
  TabPanel,
  Divider,
  Spacer,
  Image,
};

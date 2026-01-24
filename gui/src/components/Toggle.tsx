interface ToggleProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  size?: 'sm' | 'md';
}

export default function Toggle({ checked, onChange, disabled = false, size = 'md' }: ToggleProps) {
  const sizes = {
    sm: {
      track: 'w-8 h-4',
      thumb: 'w-3 h-3',
      translate: 'translate-x-4',
    },
    md: {
      track: 'w-11 h-6',
      thumb: 'w-5 h-5',
      translate: 'translate-x-5',
    },
  };

  const s = sizes[size];

  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      disabled={disabled}
      onClick={() => !disabled && onChange(!checked)}
      className={`
        relative inline-flex flex-shrink-0 ${s.track}
        border-2 border-transparent rounded-full cursor-pointer
        transition-colors ease-in-out duration-200
        focus:outline-none focus:ring-2 focus:ring-accent-teal focus:ring-offset-2 focus:ring-offset-background-deep
        ${checked ? 'bg-accent-amber' : 'bg-background-surface'}
        ${disabled ? 'opacity-50 cursor-not-allowed' : ''}
      `}
    >
      <span
        className={`
          pointer-events-none inline-block ${s.thumb} rounded-full
          bg-white shadow transform ring-0
          transition ease-in-out duration-200
          ${checked ? s.translate : 'translate-x-0'}
        `}
      />
    </button>
  );
}

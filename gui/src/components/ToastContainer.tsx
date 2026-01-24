/**
 * Toast Container Component
 *
 * Renders toast notifications in the bottom-right corner.
 */

import { X, CheckCircle2, AlertCircle, Info, AlertTriangle } from 'lucide-react';
import { useToastStore, Toast, ToastType } from '../stores/toastStore';

const typeStyles: Record<ToastType, { bg: string; border: string; icon: string }> = {
  success: {
    bg: 'bg-green-500/10',
    border: 'border-green-500/30',
    icon: 'text-green-400',
  },
  error: {
    bg: 'bg-red-500/10',
    border: 'border-red-500/30',
    icon: 'text-red-400',
  },
  info: {
    bg: 'bg-accent-teal/10',
    border: 'border-accent-teal/30',
    icon: 'text-accent-teal',
  },
  warning: {
    bg: 'bg-accent-amber/10',
    border: 'border-accent-amber/30',
    icon: 'text-accent-amber',
  },
};

const TypeIcon = ({ type }: { type: ToastType }) => {
  const className = `w-5 h-5 ${typeStyles[type].icon}`;
  switch (type) {
    case 'success':
      return <CheckCircle2 className={className} />;
    case 'error':
      return <AlertCircle className={className} />;
    case 'warning':
      return <AlertTriangle className={className} />;
    default:
      return <Info className={className} />;
  }
};

function ToastItem({ toast }: { toast: Toast }) {
  const removeToast = useToastStore((state) => state.removeToast);
  const styles = typeStyles[toast.type];

  return (
    <div
      className={`flex items-start gap-3 p-4 ${styles.bg} border ${styles.border} rounded-lg shadow-lg backdrop-blur-sm animate-slide-in-right`}
    >
      <TypeIcon type={toast.type} />
      <p className="flex-1 text-sm text-white">{toast.message}</p>
      <button
        onClick={() => removeToast(toast.id)}
        className="text-gray-400 hover:text-white transition-colors"
      >
        <X className="w-4 h-4" />
      </button>
    </div>
  );
}

export default function ToastContainer() {
  const toasts = useToastStore((state) => state.toasts);

  if (toasts.length === 0) return null;

  return (
    <div className="fixed bottom-4 right-4 z-[100] flex flex-col gap-2 max-w-sm">
      {toasts.map((toast) => (
        <ToastItem key={toast.id} toast={toast} />
      ))}
    </div>
  );
}

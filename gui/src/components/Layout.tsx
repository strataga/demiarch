import { Outlet, NavLink } from 'react-router-dom';
import {
  LayoutDashboard,
  FolderKanban,
  Bot,
  Settings,
  Sparkles
} from 'lucide-react';

const navItems = [
  { to: '/', icon: LayoutDashboard, label: 'Dashboard' },
  { to: '/projects', icon: FolderKanban, label: 'Projects' },
  { to: '/agents', icon: Bot, label: 'Agents' },
  { to: '/settings', icon: Settings, label: 'Settings' },
];

export default function Layout() {
  return (
    <div className="flex h-screen bg-background-deep">
      {/* Sidebar */}
      <aside className="w-64 bg-background-mid border-r border-background-surface flex flex-col">
        {/* Logo */}
        <div className="p-4 border-b border-background-surface">
          <div className="flex items-center gap-2">
            <Sparkles className="w-8 h-8 text-accent-teal" />
            <span className="text-xl font-bold">Demiarch</span>
          </div>
          <p className="text-xs text-gray-400 mt-1">AI App Builder</p>
        </div>

        {/* Navigation */}
        <nav className="flex-1 p-4 space-y-1">
          {navItems.map(({ to, icon: Icon, label }) => (
            <NavLink
              key={to}
              to={to}
              className={({ isActive }) =>
                `flex items-center gap-3 px-3 py-2 rounded-lg transition-colors ${
                  isActive
                    ? 'bg-accent-teal/20 text-accent-teal'
                    : 'text-gray-400 hover:text-white hover:bg-background-surface'
                }`
              }
            >
              <Icon className="w-5 h-5" />
              <span>{label}</span>
            </NavLink>
          ))}
        </nav>

        {/* Footer */}
        <div className="p-4 border-t border-background-surface">
          <div className="text-xs text-gray-500">
            <p>Version 0.1.0</p>
          </div>
        </div>
      </aside>

      {/* Main content */}
      <main className="flex-1 overflow-auto">
        <Outlet />
      </main>
    </div>
  );
}

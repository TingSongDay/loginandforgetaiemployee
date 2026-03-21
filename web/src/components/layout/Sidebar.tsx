import { NavLink } from 'react-router-dom';
import {
  LayoutDashboard,
  MessageSquare,
  Wrench,
  Clock,
  Puzzle,
  Brain,
  Settings,
  DollarSign,
  Activity,
  Stethoscope,
} from 'lucide-react';
import { t } from '@/lib/i18n';

const navItems = [
  { to: '/', icon: LayoutDashboard, labelKey: 'nav.dashboard' },
  { to: '/agent', icon: MessageSquare, labelKey: 'nav.agent' },
  { to: '/tools', icon: Wrench, labelKey: 'nav.tools' },
  { to: '/cron', icon: Clock, labelKey: 'nav.cron' },
  { to: '/integrations', icon: Puzzle, labelKey: 'nav.integrations' },
  { to: '/memory', icon: Brain, labelKey: 'nav.memory' },
  { to: '/config', icon: Settings, labelKey: 'nav.config' },
  { to: '/cost', icon: DollarSign, labelKey: 'nav.cost' },
  { to: '/logs', icon: Activity, labelKey: 'nav.logs' },
  { to: '/doctor', icon: Stethoscope, labelKey: 'nav.doctor' },
];

export default function Sidebar() {
  return (
    <aside className="nh-shell-sidebar">
      <div className="nh-sidebar-brand">
        <div className="nh-sidebar-logo">
          NH
        </div>
        <div>
          <p className="nh-sidebar-kicker">Digital human station</p>
          <p className="nh-sidebar-title">NeoHuman</p>
        </div>
      </div>

      <nav className="nh-sidebar-nav">
        {navItems.map(({ to, icon: Icon, labelKey }) => (
          <NavLink
            key={to}
            to={to}
            end={to === '/'}
            className={({ isActive }) =>
              [
                'nh-sidebar-link',
                isActive ? 'nh-sidebar-link-active' : '',
              ].join(' ')
            }
          >
            <span className="nh-sidebar-link-icon">
              <Icon className="h-5 w-5 flex-shrink-0" />
            </span>
            <span>{t(labelKey)}</span>
          </NavLink>
        ))}
      </nav>
    </aside>
  );
}

import { useLocation } from 'react-router-dom';
import { LogOut, Sparkles } from 'lucide-react';
import { t } from '@/lib/i18n';
import { useAuth } from '@/hooks/useAuth';

const routeTitles: Record<string, string> = {
  '/': 'nav.dashboard',
  '/agent': 'nav.agent',
  '/tools': 'nav.tools',
  '/cron': 'nav.cron',
  '/integrations': 'nav.integrations',
  '/memory': 'nav.memory',
  '/config': 'nav.config',
  '/cost': 'nav.cost',
  '/logs': 'nav.logs',
  '/doctor': 'nav.doctor',
};

export default function Header() {
  const location = useLocation();
  const { logout } = useAuth();

  const titleKey = routeTitles[location.pathname] ?? 'nav.dashboard';
  const pageTitle = t(titleKey);

  return (
    <header className="nh-shell-header">
      <div className="nh-shell-header-inner">
        <div>
          <p className="nh-header-kicker">NeoHuman operator surface</p>
          <h1 className="nh-header-title">{pageTitle}</h1>
        </div>

        <div className="nh-header-actions">
          <div className="nh-header-badge">
            <Sparkles className="h-3.5 w-3.5" />
            Worker B live
          </div>
          <button
            type="button"
            onClick={logout}
            className="nh-header-action"
          >
            <LogOut className="h-4 w-4" />
            <span>{t('auth.logout')}</span>
          </button>
        </div>
      </div>
    </header>
  );
}

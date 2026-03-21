import { useState, useEffect } from 'react';
import {
  Activity,
  Bot,
  BrainCircuit,
  CheckCircle2,
  Clock3,
  Cpu,
  Database,
  DollarSign,
  Globe,
  HeartPulse,
  MessageSquareWarning,
  MonitorSmartphone,
  Orbit,
  Radio,
  ShieldCheck,
  Sparkles,
  TriangleAlert,
  Waves,
} from 'lucide-react';
import type { StatusResponse, CostSummary } from '@/types/api';
import { getStatus, getCost, postStationWorkerAction } from '@/lib/api';

function formatUptime(seconds: number): string {
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (d > 0) return `${d}d ${h}h ${m}m`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function formatUSD(value: number): string {
  return `$${value.toFixed(4)}`;
}

function formatTimestamp(value: string | null): string {
  if (!value) return 'Not yet';
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString();
}

function healthColor(status: string): string {
  switch (status.toLowerCase()) {
    case 'ok':
    case 'healthy':
      return 'bg-lime-400';
    case 'warn':
    case 'warning':
    case 'degraded':
      return 'bg-amber-400';
    default:
      return 'bg-rose-400';
  }
}

function healthBorder(status: string): string {
  switch (status.toLowerCase()) {
    case 'ok':
    case 'healthy':
      return 'border-lime-300/25';
    case 'warn':
    case 'warning':
    case 'degraded':
      return 'border-amber-300/25';
    default:
      return 'border-rose-300/25';
  }
}

function sentenceCase(value: string | null | undefined, fallback = 'Unavailable'): string {
  if (!value) return fallback;
  return value
    .split(/[_-]/g)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

function stationStateLabel(
  station: StatusResponse['station'],
  leftSurface: StatusResponse['station']['left_surface'],
): string {
  if (!station.enabled) return 'Offline';
  if (leftSurface?.attention_reason || leftSurface?.last_error) return 'Needs Human Attention';
  if (leftSurface?.paused) return 'Paused';
  if (leftSurface?.session_status) return sentenceCase(leftSurface.session_status);
  return 'Live';
}

function stationStateTone(
  station: StatusResponse['station'],
  leftSurface: StatusResponse['station']['left_surface'],
): 'live' | 'warn' | 'muted' | 'danger' {
  if (!station.enabled) return 'danger';
  if (leftSurface?.attention_reason || leftSurface?.last_error) return 'warn';
  if (leftSurface?.paused) return 'muted';
  return 'live';
}

function primaryHealthSignal(status: StatusResponse): { title: string; detail: string } {
  const attention = status.station.left_surface?.attention_reason || status.station.left_surface?.last_error;
  if (attention) {
    return {
      title: 'Human handoff requested',
      detail: attention,
    };
  }

  const unhealthy = Object.entries(status.health.components).find(([, component]) => {
    const current = component.status.toLowerCase();
    return !['ok', 'healthy'].includes(current);
  });

  if (unhealthy) {
    return {
      title: `Vitality drift in ${unhealthy[0]}`,
      detail: sentenceCase(unhealthy[1].status),
    };
  }

  return {
    title: 'System vitality stable',
    detail: 'Worker duet is responsive and synchronized.',
  };
}

export default function Dashboard() {
  const [status, setStatus] = useState<StatusResponse | null>(null);
  const [cost, setCost] = useState<CostSummary | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [stationAction, setStationAction] = useState<string | null>(null);

  const refresh = () => {
    Promise.all([getStatus(), getCost()])
      .then(([s, c]) => {
        setStatus(s);
        setCost(c);
        setError(null);
      })
      .catch((err) => setError(err.message));
  };

  useEffect(() => {
    refresh();
  }, []);

  async function runStationAction(
    workerId: string,
    action: 'pause' | 'resume' | 'mark-login-complete' | 'mark-challenge-complete',
  ) {
    try {
      setStationAction(action);
      await postStationWorkerAction(workerId, action);
      refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setStationAction(null);
    }
  }

  if (error) {
    return (
      <div className="nh-dashboard-shell">
        <div className="nh-dashboard-error">
          <TriangleAlert className="h-5 w-5 shrink-0" />
          <div>
            <p className="font-semibold text-white">Dashboard unavailable</p>
            <p className="mt-1 text-sm text-rose-100/80">Failed to load dashboard: {error}</p>
          </div>
        </div>
      </div>
    );
  }

  if (!status || !cost) {
    return (
      <div className="nh-dashboard-shell">
        <div className="nh-dashboard-loading">
          <div className="nh-dashboard-spinner" />
          <div>
            <p className="text-sm uppercase tracking-[0.3em] text-cyan-200/70">NeoHuman</p>
            <p className="mt-2 text-lg font-semibold text-white">
              Connecting the digital human cockpit...
            </p>
          </div>
        </div>
      </div>
    );
  }

  const leftSurface = status.station.left_surface;
  const maxCost = Math.max(
    cost.session_cost_usd,
    cost.daily_cost_usd,
    cost.monthly_cost_usd,
    0.001,
  );
  const stateLabel = stationStateLabel(status.station, leftSurface);
  const stateTone = stationStateTone(status.station, leftSurface);
  const healthSignal = primaryHealthSignal(status);
  const metrics = [
    {
      label: 'Provider / Model',
      value: status.provider ?? 'Unknown',
      detail: status.model,
      icon: Cpu,
      tone: 'cyan',
    },
    {
      label: 'Uptime',
      value: formatUptime(status.uptime_seconds),
      detail: 'Since last restart',
      icon: Clock3,
      tone: 'amber',
    },
    {
      label: 'Gateway / Locale',
      value: `:${status.gateway_port}`,
      detail: `Locale ${status.locale}`,
      icon: Globe,
      tone: 'violet',
    },
    {
      label: 'Memory / Pairing',
      value: sentenceCase(status.memory_backend),
      detail: status.paired ? 'Paired and trusted' : 'Not paired yet',
      icon: Database,
      tone: 'lime',
    },
  ];
  const resourceBurn = [
    { label: 'Session', value: cost.session_cost_usd, tone: 'cyan' },
    { label: 'Daily', value: cost.daily_cost_usd, tone: 'coral' },
    { label: 'Monthly', value: cost.monthly_cost_usd, tone: 'violet' },
  ];

  return (
    <div className="nh-dashboard-shell">
      <section className="nh-hero-card">
        <div className="nh-hero-copy">
          <div className="nh-hero-badge">
            <Sparkles className="h-4 w-4" />
            NeoHuman worker_b dashboard
          </div>
          <div className="space-y-4">
            <p className="text-sm uppercase tracking-[0.35em] text-cyan-100/60">
              Digital human cockpit
            </p>
            <h1 className="nh-hero-title">NeoHuman</h1>
            <p className="nh-hero-subtitle">
              A vivid operator presence for work that should have evolved already. Worker B should
              feel like a living digital collaborator, not a back-office terminal.
            </p>
          </div>
          <div className="nh-hero-status-row">
            <div className={`nh-state-pill nh-state-pill-${stateTone}`}>
              <span className="nh-state-dot" />
              {stateLabel}
            </div>
            <div className="nh-state-context">
              <p className="text-xs uppercase tracking-[0.22em] text-white/45">Reply mode</p>
              <p className="text-sm font-medium text-white">
                {sentenceCase(status.station.reply_mode)}
              </p>
            </div>
            <div className="nh-state-context">
              <p className="text-xs uppercase tracking-[0.22em] text-white/45">Operator</p>
              <p className="text-sm font-medium text-white">
                {status.station.operator_display_name ?? 'Unspecified'}
              </p>
            </div>
          </div>
        </div>

        <div className="nh-hero-orbital-panel">
          <div className="nh-hero-orbital-ring" />
          <div className="nh-hero-orbital-core">
            <Bot className="h-9 w-9 text-cyan-100" />
          </div>
          <div className="nh-hero-signal-card">
            <div className="flex items-center gap-2 text-cyan-100">
              <HeartPulse className="h-4 w-4" />
              <span className="text-sm font-medium">{healthSignal.title}</span>
            </div>
            <p className="mt-2 text-sm text-white/70">{healthSignal.detail}</p>
          </div>
        </div>
      </section>

      <section className="nh-metric-ribbon">
        {metrics.map(({ label, value, detail, icon: Icon, tone }) => (
          <article key={label} className={`nh-metric-pill nh-metric-pill-${tone}`}>
            <div className="nh-metric-icon">
              <Icon className="h-5 w-5" />
            </div>
            <div>
              <p className="text-[11px] uppercase tracking-[0.24em] text-white/45">{label}</p>
              <p className="mt-1 text-lg font-semibold text-white">{value}</p>
              <p className="text-sm text-white/55">{detail}</p>
            </div>
          </article>
        ))}
      </section>

      <section className="nh-worker-grid">
        <article className="nh-surface-card nh-surface-card-a">
          <div className="nh-surface-head">
            <div>
              <div className="nh-surface-label">
                <MonitorSmartphone className="h-4 w-4" />
                Worker A
              </div>
              <h2 className="nh-surface-title">
                {leftSurface?.display_name ?? 'Automation surface unavailable'}
              </h2>
              <p className="nh-surface-copy">
                The active execution surface for conversations, snap-back control, and human
                recovery.
              </p>
            </div>
            <div className={`nh-state-pill nh-state-pill-${leftSurface?.paused ? 'muted' : stateTone}`}>
              <span className="nh-state-dot" />
              {leftSurface?.session_status ?? 'Idle'}
            </div>
          </div>

          <div className="nh-surface-stats">
            <div className="nh-detail-block">
              <p className="nh-detail-label">Backend</p>
              <p className="nh-detail-value">{leftSurface?.backend ?? 'Not launched'}</p>
            </div>
            <div className="nh-detail-block">
              <p className="nh-detail-label">Geometry</p>
              <p className="nh-detail-value">
                {leftSurface
                  ? `${leftSurface.window_origin_x},${leftSurface.window_origin_y} • ${leftSurface.viewport_width}x${leftSurface.viewport_height}`
                  : 'N/A'}
              </p>
            </div>
            <div className="nh-detail-block">
              <p className="nh-detail-label">Last inbound</p>
              <p className="nh-detail-value">
                {formatTimestamp(leftSurface?.last_inbound_message_at ?? null)}
              </p>
            </div>
            <div className="nh-detail-block">
              <p className="nh-detail-label">Last reply</p>
              <p className="nh-detail-value">
                {formatTimestamp(leftSurface?.last_reply_sent_at ?? null)}
              </p>
            </div>
          </div>

          <div className="nh-attention-panel">
            <div className="flex items-center gap-2 text-white">
              <TriangleAlert className="h-4 w-4 text-amber-300" />
              <span className="text-sm font-medium">Intervention signal</span>
            </div>
            <p className="mt-3 text-sm text-white/78">
              {leftSurface?.attention_reason ?? leftSurface?.last_error ?? 'No intervention requested'}
            </p>
            {leftSurface?.pending_reply_text && (
              <p className="mt-3 rounded-2xl border border-cyan-300/20 bg-cyan-300/10 px-4 py-3 text-sm text-cyan-50">
                Pending reply: {leftSurface.pending_reply_text}
              </p>
            )}
          </div>

          {leftSurface && (
            <div className="nh-action-cluster">
              <button
                type="button"
                onClick={() => runStationAction(leftSurface.worker_id, 'resume')}
                disabled={stationAction !== null}
                className="nh-action-button nh-action-button-primary"
              >
                Resume
              </button>
              <button
                type="button"
                onClick={() => runStationAction(leftSurface.worker_id, 'pause')}
                disabled={stationAction !== null}
                className="nh-action-button nh-action-button-warn"
              >
                Pause
              </button>
              <button
                type="button"
                onClick={() => runStationAction(leftSurface.worker_id, 'mark-login-complete')}
                disabled={stationAction !== null}
                className="nh-action-button nh-action-button-secondary"
              >
                Mark Login Complete
              </button>
              <button
                type="button"
                onClick={() => runStationAction(leftSurface.worker_id, 'mark-challenge-complete')}
                disabled={stationAction !== null}
                className="nh-action-button nh-action-button-secondary"
              >
                Mark Challenge Complete
              </button>
            </div>
          )}
        </article>

        <article className="nh-surface-card nh-surface-card-b">
          <div className="nh-surface-head">
            <div>
              <div className="nh-surface-label">
                <BrainCircuit className="h-4 w-4" />
                Worker B
              </div>
              <h2 className="nh-surface-title">
                {sentenceCase(status.station.right_panel.runtime_mode)}
              </h2>
              <p className="nh-surface-copy">
                The supervising digital human interface that interprets intent and orchestrates
                Worker A.
              </p>
            </div>
            <div className="nh-presence-chip">
              <Orbit className="h-4 w-4" />
              Operator presence live
            </div>
          </div>

          <div className="nh-surface-stats">
            <div className="nh-detail-block">
              <p className="nh-detail-label">Path</p>
              <p className="nh-detail-value">{status.station.right_panel.local_url_or_path}</p>
            </div>
            <div className="nh-detail-block">
              <p className="nh-detail-label">Geometry</p>
              <p className="nh-detail-value">
                {status.station.right_panel.geometry_managed ? 'Managed' : 'Free-positioned'}
              </p>
            </div>
            <div className="nh-detail-block">
              <p className="nh-detail-label">Operator</p>
              <p className="nh-detail-value">
                {status.station.operator_display_name ?? 'Unspecified'}
              </p>
            </div>
            <div className="nh-detail-block">
              <p className="nh-detail-label">Reply mode</p>
              <p className="nh-detail-value">{sentenceCase(status.station.reply_mode)}</p>
            </div>
          </div>

          <div className="nh-presence-panel">
            <div className="flex items-center gap-2 text-white">
              <MessageSquareWarning className="h-4 w-4 text-rose-200" />
              <span className="text-sm font-medium">Control actions</span>
            </div>
            <p className="mt-3 text-sm text-white/78">
              {status.station.right_panel.control_actions.join(', ')}
            </p>
          </div>

          <div className="nh-worker-b-callout">
            <Waves className="h-5 w-5 text-orange-200" />
            <div>
              <p className="text-sm font-medium text-white">Human-centric orchestration</p>
              <p className="mt-1 text-sm text-white/68">
                Worker B exists to feel intentional, warm, and decisive while replacing routine
                operator work.
              </p>
            </div>
          </div>
        </article>
      </section>

      <section className="nh-intelligence-grid">
        <article className="nh-intel-card nh-intel-card-burn">
          <div className="nh-intel-head">
            <div className="nh-intel-icon">
              <DollarSign className="h-5 w-5" />
            </div>
            <div>
              <p className="nh-intel-kicker">Operational intelligence</p>
              <h3 className="nh-intel-title">Resource Burn</h3>
            </div>
          </div>
          <div className="space-y-4">
            {resourceBurn.map(({ label, value, tone }) => (
              <div key={label}>
                <div className="mb-1 flex items-center justify-between text-sm">
                  <span className="text-white/60">{label}</span>
                  <span className="font-medium text-white">{formatUSD(value)}</span>
                </div>
                <div className="nh-burn-track">
                  <div
                    className={`nh-burn-fill nh-burn-fill-${tone}`}
                    style={{ width: `${Math.max((value / maxCost) * 100, 2)}%` }}
                  />
                </div>
              </div>
            ))}
          </div>
          <div className="mt-5 grid grid-cols-2 gap-3">
            <div className="nh-mini-stat">
              <p className="nh-detail-label">Total tokens</p>
              <p className="nh-detail-value">{cost.total_tokens.toLocaleString()}</p>
            </div>
            <div className="nh-mini-stat">
              <p className="nh-detail-label">Requests</p>
              <p className="nh-detail-value">{cost.request_count.toLocaleString()}</p>
            </div>
          </div>
        </article>

        <article className="nh-intel-card nh-intel-card-health">
          <div className="nh-intel-head">
            <div className="nh-intel-icon">
              <HeartPulse className="h-5 w-5" />
            </div>
            <div>
              <p className="nh-intel-kicker">Operational intelligence</p>
              <h3 className="nh-intel-title">System Vitality</h3>
            </div>
          </div>
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
            {Object.entries(status.health.components).length === 0 ? (
              <p className="text-sm text-white/45">No components reporting</p>
            ) : (
              Object.entries(status.health.components).map(([name, comp]) => (
                <div key={name} className={`nh-vitality-chip ${healthBorder(comp.status)}`}>
                  <div className="flex items-center gap-2">
                    <span className={`inline-block h-2.5 w-2.5 rounded-full ${healthColor(comp.status)}`} />
                    <span className="truncate text-sm font-medium text-white">{name}</span>
                  </div>
                  <p className="mt-2 text-xs uppercase tracking-[0.2em] text-white/40">
                    {comp.status}
                  </p>
                  {comp.restart_count > 0 && (
                    <p className="mt-2 text-xs text-amber-200">Restarts: {comp.restart_count}</p>
                  )}
                </div>
              ))
            )}
          </div>
        </article>

        <article className="nh-intel-card nh-intel-card-contract">
          <div className="nh-intel-head">
            <div className="nh-intel-icon">
              <ShieldCheck className="h-5 w-5" />
            </div>
            <div>
              <p className="nh-intel-kicker">Operational intelligence</p>
              <h3 className="nh-intel-title">Trust Envelope</h3>
            </div>
          </div>
          <div className="space-y-3">
            <div className="nh-mini-stat">
              <p className="nh-detail-label">Snap-back before interaction</p>
              <p className="nh-detail-value">
                {leftSurface?.snap_back_before_interaction ? 'Enabled' : 'Disabled'}
              </p>
            </div>
            <div className="nh-mini-stat">
              <p className="nh-detail-label">Preflight verification</p>
              <p className="nh-detail-value">
                {leftSurface?.preflight_verification_enabled ? 'Enabled' : 'Disabled'}
              </p>
            </div>
            <div className="nh-mini-stat">
              <p className="nh-detail-label">Display scale mode</p>
              <p className="nh-detail-value">{leftSurface?.display_scale_mode ?? 'N/A'}</p>
            </div>
            <div className="nh-mini-stat">
              <p className="nh-detail-label">Actual placement</p>
              <p className="nh-detail-value">
                {leftSurface &&
                leftSurface.actual_window_origin_x !== null &&
                leftSurface.actual_window_origin_y !== null &&
                leftSurface.actual_viewport_width !== null &&
                leftSurface.actual_viewport_height !== null
                  ? `${leftSurface.actual_window_origin_x},${leftSurface.actual_window_origin_y} • ${leftSurface.actual_viewport_width}x${leftSurface.actual_viewport_height}`
                  : 'Pending verification'}
              </p>
            </div>
          </div>
        </article>

        <article className="nh-intel-card nh-intel-card-channels">
          <div className="nh-intel-head">
            <div className="nh-intel-icon">
              <Radio className="h-5 w-5" />
            </div>
            <div>
              <p className="nh-intel-kicker">Operational intelligence</p>
              <h3 className="nh-intel-title">Live Channels</h3>
            </div>
          </div>
          <div className="space-y-2">
            {Object.entries(status.channels).length === 0 ? (
              <p className="text-sm text-white/45">No channels configured</p>
            ) : (
              Object.entries(status.channels).map(([name, active]) => (
                <div key={name} className="nh-channel-row">
                  <div className="flex items-center gap-3">
                    {active ? (
                      <CheckCircle2 className="h-4 w-4 text-lime-300" />
                    ) : (
                      <Activity className="h-4 w-4 text-white/35" />
                    )}
                    <span className="text-sm text-white capitalize">{name}</span>
                  </div>
                  <span
                    className={`nh-channel-pill ${active ? 'nh-channel-pill-on' : 'nh-channel-pill-off'}`}
                  >
                    {active ? 'Active' : 'Inactive'}
                  </span>
                </div>
              ))
            )}
          </div>
        </article>
      </section>
    </div>
  );
}

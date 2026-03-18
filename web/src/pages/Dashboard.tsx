import { useState, useEffect } from 'react';
import {
  Cpu,
  Clock,
  Globe,
  Database,
  Activity,
  DollarSign,
  Radio,
  MonitorSmartphone,
  MessageSquareWarning,
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
      return 'bg-green-500';
    case 'warn':
    case 'warning':
    case 'degraded':
      return 'bg-yellow-500';
    default:
      return 'bg-red-500';
  }
}

function healthBorder(status: string): string {
  switch (status.toLowerCase()) {
    case 'ok':
    case 'healthy':
      return 'border-green-500/30';
    case 'warn':
    case 'warning':
    case 'degraded':
      return 'border-yellow-500/30';
    default:
      return 'border-red-500/30';
  }
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
      <div className="p-6">
        <div className="rounded-lg bg-red-900/30 border border-red-700 p-4 text-red-300">
          Failed to load dashboard: {error}
        </div>
      </div>
    );
  }

  if (!status || !cost) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-2 border-blue-500 border-t-transparent" />
      </div>
    );
  }

  const maxCost = Math.max(cost.session_cost_usd, cost.daily_cost_usd, cost.monthly_cost_usd, 0.001);
  const leftSurface = status.station.left_surface;

  return (
    <div className="p-6 space-y-6">
      <div className="grid grid-cols-1 xl:grid-cols-3 gap-6">
        <div className="xl:col-span-2 bg-gray-900 rounded-xl p-5 border border-gray-800">
          <div className="flex items-center justify-between gap-4 mb-4">
            <div className="flex items-center gap-2">
              <MonitorSmartphone className="h-5 w-5 text-blue-400" />
              <h2 className="text-base font-semibold text-white">NeoHUman Station</h2>
            </div>
            <span className="text-xs uppercase tracking-wide text-gray-400">
              {status.station.enabled ? 'Enabled' : 'Disabled'}
            </span>
          </div>
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
            <div className="rounded-xl border border-gray-800 bg-gray-800/40 p-4 space-y-3">
              <div className="flex items-center justify-between gap-3">
                <div>
                  <p className="text-sm text-gray-400">Left Automation Surface</p>
                  <p className="text-lg font-semibold text-white">
                    {leftSurface?.display_name ?? 'Not configured'}
                  </p>
                </div>
                <span className="text-xs uppercase tracking-wide text-gray-400">
                  {leftSurface?.session_status ?? 'idle'}
                </span>
              </div>
              <div className="grid grid-cols-2 gap-3 text-sm">
                <div>
                  <p className="text-gray-500">Browser</p>
                  <p className="text-white">{leftSurface?.backend ?? 'Not launched'}</p>
                </div>
                <div>
                  <p className="text-gray-500">Geometry</p>
                  <p className="text-white">
                    {leftSurface
                      ? `${leftSurface.window_origin_x},${leftSurface.window_origin_y} • ${leftSurface.viewport_width}x${leftSurface.viewport_height}`
                      : 'N/A'}
                  </p>
                </div>
                <div>
                  <p className="text-gray-500">Last Inbound</p>
                  <p className="text-white">{formatTimestamp(leftSurface?.last_inbound_message_at ?? null)}</p>
                </div>
                <div>
                  <p className="text-gray-500">Last Reply</p>
                  <p className="text-white">{formatTimestamp(leftSurface?.last_reply_sent_at ?? null)}</p>
                </div>
              </div>
              <div className="rounded-lg bg-gray-900/80 border border-gray-800 p-3 text-sm">
                <p className="text-gray-400">Recovery / Attention</p>
                <p className="text-white mt-1">
                  {leftSurface?.attention_reason ??
                    leftSurface?.last_error ??
                    'No intervention requested'}
                </p>
                {leftSurface?.pending_reply_text && (
                  <p className="text-blue-300 mt-2">
                    Pending reply: {leftSurface.pending_reply_text}
                  </p>
                )}
              </div>
              {leftSurface && (
                <div className="flex flex-wrap gap-2">
                  <button
                    type="button"
                    onClick={() => runStationAction(leftSurface.worker_id, 'pause')}
                    disabled={stationAction !== null}
                    className="px-3 py-2 rounded-lg bg-amber-600/20 border border-amber-500/30 text-amber-200 text-sm disabled:opacity-50"
                  >
                    Pause
                  </button>
                  <button
                    type="button"
                    onClick={() => runStationAction(leftSurface.worker_id, 'resume')}
                    disabled={stationAction !== null}
                    className="px-3 py-2 rounded-lg bg-green-600/20 border border-green-500/30 text-green-200 text-sm disabled:opacity-50"
                  >
                    Resume
                  </button>
                  <button
                    type="button"
                    onClick={() => runStationAction(leftSurface.worker_id, 'mark-login-complete')}
                    disabled={stationAction !== null}
                    className="px-3 py-2 rounded-lg bg-blue-600/20 border border-blue-500/30 text-blue-200 text-sm disabled:opacity-50"
                  >
                    Mark Login Complete
                  </button>
                  <button
                    type="button"
                    onClick={() => runStationAction(leftSurface.worker_id, 'mark-challenge-complete')}
                    disabled={stationAction !== null}
                    className="px-3 py-2 rounded-lg bg-purple-600/20 border border-purple-500/30 text-purple-200 text-sm disabled:opacity-50"
                  >
                    Mark Challenge Complete
                  </button>
                </div>
              )}
            </div>

            <div className="rounded-xl border border-gray-800 bg-gray-800/40 p-4 space-y-3">
              <div>
                <p className="text-sm text-gray-400">Right Operator Panel</p>
                <p className="text-lg font-semibold text-white">
                  {status.station.right_panel.runtime_mode}
                </p>
              </div>
              <div className="grid grid-cols-1 gap-3 text-sm">
                <div>
                  <p className="text-gray-500">Path</p>
                  <p className="text-white">{status.station.right_panel.local_url_or_path}</p>
                </div>
                <div>
                  <p className="text-gray-500">Geometry</p>
                  <p className="text-white">
                    {status.station.right_panel.geometry_managed ? 'Managed' : 'Free-positioned'}
                  </p>
                </div>
                <div>
                  <p className="text-gray-500">Operator</p>
                  <p className="text-white">
                    {status.station.operator_display_name ?? 'Unspecified'}
                  </p>
                </div>
                <div>
                  <p className="text-gray-500">Reply Mode</p>
                  <p className="text-white">{status.station.reply_mode}</p>
                </div>
              </div>
              <div className="rounded-lg bg-gray-900/80 border border-gray-800 p-3 text-sm">
                <div className="flex items-center gap-2 text-gray-400">
                  <MessageSquareWarning className="h-4 w-4 text-blue-400" />
                  Control surface actions
                </div>
                <p className="text-white mt-2">
                  {status.station.right_panel.control_actions.join(', ')}
                </p>
              </div>
            </div>
          </div>
        </div>

        <div className="bg-gray-900 rounded-xl p-5 border border-gray-800">
          <div className="flex items-center gap-2 mb-4">
            <Activity className="h-5 w-5 text-blue-400" />
            <h2 className="text-base font-semibold text-white">Kiosk Contract</h2>
          </div>
          <div className="space-y-3 text-sm">
            <div className="rounded-lg bg-gray-800/50 p-3">
              <p className="text-gray-400">Snap-back before interaction</p>
              <p className="text-white mt-1">
                {leftSurface?.snap_back_before_interaction ? 'Enabled' : 'Disabled'}
              </p>
            </div>
            <div className="rounded-lg bg-gray-800/50 p-3">
              <p className="text-gray-400">Preflight verification</p>
              <p className="text-white mt-1">
                {leftSurface?.preflight_verification_enabled ? 'Enabled' : 'Disabled'}
              </p>
            </div>
            <div className="rounded-lg bg-gray-800/50 p-3">
              <p className="text-gray-400">Display scale mode</p>
              <p className="text-white mt-1">{leftSurface?.display_scale_mode ?? 'N/A'}</p>
            </div>
            <div className="rounded-lg bg-gray-800/50 p-3">
              <p className="text-gray-400">Actual placement</p>
              <p className="text-white mt-1">
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
        </div>
      </div>

      {/* Status Cards Grid */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <div className="bg-gray-900 rounded-xl p-5 border border-gray-800">
          <div className="flex items-center gap-3 mb-3">
            <div className="p-2 bg-blue-600/20 rounded-lg">
              <Cpu className="h-5 w-5 text-blue-400" />
            </div>
            <span className="text-sm text-gray-400">Provider / Model</span>
          </div>
          <p className="text-lg font-semibold text-white truncate">
            {status.provider ?? 'Unknown'}
          </p>
          <p className="text-sm text-gray-400 truncate">{status.model}</p>
        </div>

        <div className="bg-gray-900 rounded-xl p-5 border border-gray-800">
          <div className="flex items-center gap-3 mb-3">
            <div className="p-2 bg-green-600/20 rounded-lg">
              <Clock className="h-5 w-5 text-green-400" />
            </div>
            <span className="text-sm text-gray-400">Uptime</span>
          </div>
          <p className="text-lg font-semibold text-white">
            {formatUptime(status.uptime_seconds)}
          </p>
          <p className="text-sm text-gray-400">Since last restart</p>
        </div>

        <div className="bg-gray-900 rounded-xl p-5 border border-gray-800">
          <div className="flex items-center gap-3 mb-3">
            <div className="p-2 bg-purple-600/20 rounded-lg">
              <Globe className="h-5 w-5 text-purple-400" />
            </div>
            <span className="text-sm text-gray-400">Gateway Port</span>
          </div>
          <p className="text-lg font-semibold text-white">
            :{status.gateway_port}
          </p>
          <p className="text-sm text-gray-400">Locale: {status.locale}</p>
        </div>

        <div className="bg-gray-900 rounded-xl p-5 border border-gray-800">
          <div className="flex items-center gap-3 mb-3">
            <div className="p-2 bg-orange-600/20 rounded-lg">
              <Database className="h-5 w-5 text-orange-400" />
            </div>
            <span className="text-sm text-gray-400">Memory Backend</span>
          </div>
          <p className="text-lg font-semibold text-white capitalize">
            {status.memory_backend}
          </p>
          <p className="text-sm text-gray-400">
            Paired: {status.paired ? 'Yes' : 'No'}
          </p>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Cost Widget */}
        <div className="bg-gray-900 rounded-xl p-5 border border-gray-800">
          <div className="flex items-center gap-2 mb-4">
            <DollarSign className="h-5 w-5 text-blue-400" />
            <h2 className="text-base font-semibold text-white">Cost Overview</h2>
          </div>
          <div className="space-y-4">
            {[
              { label: 'Session', value: cost.session_cost_usd, color: 'bg-blue-500' },
              { label: 'Daily', value: cost.daily_cost_usd, color: 'bg-green-500' },
              { label: 'Monthly', value: cost.monthly_cost_usd, color: 'bg-purple-500' },
            ].map(({ label, value, color }) => (
              <div key={label}>
                <div className="flex justify-between text-sm mb-1">
                  <span className="text-gray-400">{label}</span>
                  <span className="text-white font-medium">{formatUSD(value)}</span>
                </div>
                <div className="w-full h-2 bg-gray-800 rounded-full overflow-hidden">
                  <div
                    className={`h-full rounded-full ${color}`}
                    style={{ width: `${Math.max((value / maxCost) * 100, 2)}%` }}
                  />
                </div>
              </div>
            ))}
          </div>
          <div className="mt-4 pt-3 border-t border-gray-800 flex justify-between text-sm">
            <span className="text-gray-400">Total Tokens</span>
            <span className="text-white">{cost.total_tokens.toLocaleString()}</span>
          </div>
          <div className="flex justify-between text-sm mt-1">
            <span className="text-gray-400">Requests</span>
            <span className="text-white">{cost.request_count.toLocaleString()}</span>
          </div>
        </div>

        {/* Active Channels */}
        <div className="bg-gray-900 rounded-xl p-5 border border-gray-800">
          <div className="flex items-center gap-2 mb-4">
            <Radio className="h-5 w-5 text-blue-400" />
            <h2 className="text-base font-semibold text-white">Active Channels</h2>
          </div>
          <div className="space-y-2">
            {Object.entries(status.channels).length === 0 ? (
              <p className="text-sm text-gray-500">No channels configured</p>
            ) : (
              Object.entries(status.channels).map(([name, active]) => (
                <div
                  key={name}
                  className="flex items-center justify-between py-2 px-3 rounded-lg bg-gray-800/50"
                >
                  <span className="text-sm text-white capitalize">{name}</span>
                  <div className="flex items-center gap-2">
                    <span
                      className={`inline-block h-2.5 w-2.5 rounded-full ${
                        active ? 'bg-green-500' : 'bg-gray-500'
                      }`}
                    />
                    <span className="text-xs text-gray-400">
                      {active ? 'Active' : 'Inactive'}
                    </span>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>

        {/* Health Grid */}
        <div className="bg-gray-900 rounded-xl p-5 border border-gray-800">
          <div className="flex items-center gap-2 mb-4">
            <Activity className="h-5 w-5 text-blue-400" />
            <h2 className="text-base font-semibold text-white">Component Health</h2>
          </div>
          <div className="grid grid-cols-2 gap-3">
            {Object.entries(status.health.components).length === 0 ? (
              <p className="text-sm text-gray-500 col-span-2">No components reporting</p>
            ) : (
              Object.entries(status.health.components).map(([name, comp]) => (
                <div
                  key={name}
                  className={`rounded-lg p-3 border ${healthBorder(comp.status)} bg-gray-800/50`}
                >
                  <div className="flex items-center gap-2 mb-1">
                    <span className={`inline-block h-2 w-2 rounded-full ${healthColor(comp.status)}`} />
                    <span className="text-sm font-medium text-white capitalize truncate">
                      {name}
                    </span>
                  </div>
                  <p className="text-xs text-gray-400 capitalize">{comp.status}</p>
                  {comp.restart_count > 0 && (
                    <p className="text-xs text-yellow-400 mt-1">
                      Restarts: {comp.restart_count}
                    </p>
                  )}
                </div>
              ))
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

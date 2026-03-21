import { useState } from 'react';
import {
  Stethoscope,
  Play,
  CheckCircle,
  AlertTriangle,
  XCircle,
  Loader2,
} from 'lucide-react';
import type { DiagResult } from '@/types/api';
import { runDoctor } from '@/lib/api';

function severityIcon(severity: DiagResult['severity']) {
  switch (severity) {
    case 'ok':
      return <CheckCircle className="h-4 w-4 text-green-400 flex-shrink-0" />;
    case 'warn':
      return <AlertTriangle className="h-4 w-4 text-yellow-400 flex-shrink-0" />;
    case 'error':
      return <XCircle className="h-4 w-4 text-red-400 flex-shrink-0" />;
  }
}

function severityBorder(severity: DiagResult['severity']): string {
  switch (severity) {
    case 'ok':
      return 'border-green-700/40';
    case 'warn':
      return 'border-yellow-700/40';
    case 'error':
      return 'border-red-700/40';
  }
}

function severityBg(severity: DiagResult['severity']): string {
  switch (severity) {
    case 'ok':
      return 'bg-green-900/10';
    case 'warn':
      return 'bg-yellow-900/10';
    case 'error':
      return 'bg-red-900/10';
  }
}

export default function Doctor() {
  const [results, setResults] = useState<DiagResult[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleRun = async () => {
    setLoading(true);
    setError(null);
    setResults(null);
    try {
      const data = await runDoctor();
      setResults(data);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Failed to run diagnostics');
    } finally {
      setLoading(false);
    }
  };

  // Compute summary counts
  const okCount = results?.filter((r) => r.severity === 'ok').length ?? 0;
  const warnCount = results?.filter((r) => r.severity === 'warn').length ?? 0;
  const errorCount = results?.filter((r) => r.severity === 'error').length ?? 0;

  // Group by category
  const grouped =
    results?.reduce<Record<string, DiagResult[]>>((acc, item) => {
      const key = item.category;
      if (!acc[key]) acc[key] = [];
      acc[key].push(item);
      return acc;
    }, {}) ?? {};

  return (
    <div className="nh-page-shell">
      <section className="nh-page-hero">
        <div>
          <p className="nh-page-kicker">System vitality</p>
          <h2 className="nh-page-title">Doctor</h2>
          <p className="nh-page-subtitle">
            Inspect the health envelope of the NeoHuman station, from component status to
            operator-facing recovery signals.
          </p>
        </div>
        <button
          onClick={handleRun}
          disabled={loading}
          className="nh-button-primary"
        >
          {loading ? (
            <>
              <Loader2 className="h-4 w-4 animate-spin" />
              Running...
            </>
          ) : (
            <>
              <Play className="h-4 w-4" />
              Run Diagnostics
            </>
          )}
        </button>
      </section>

      {error && (
        <div className="nh-page-note nh-page-note-danger text-rose-200">
          {error}
        </div>
      )}

      {loading && (
        <div className="nh-panel nh-empty-state py-16">
          <Loader2 className="h-10 w-10 text-blue-500 animate-spin mb-4" />
          <p className="text-gray-400">Running diagnostics...</p>
          <p className="text-sm text-gray-500 mt-1">
            This may take a few seconds.
          </p>
        </div>
      )}

      {/* Results */}
      {results && !loading && (
        <>
          <div className="nh-panel flex items-center gap-4 p-4">
            <div className="flex items-center gap-2">
              <CheckCircle className="h-5 w-5 text-green-400" />
              <span className="text-sm text-white font-medium">
                {okCount} <span className="text-gray-400 font-normal">ok</span>
              </span>
            </div>
            <div className="w-px h-5 bg-gray-700" />
            <div className="flex items-center gap-2">
              <AlertTriangle className="h-5 w-5 text-yellow-400" />
              <span className="text-sm text-white font-medium">
                {warnCount}{' '}
                <span className="text-gray-400 font-normal">
                  warning{warnCount !== 1 ? 's' : ''}
                </span>
              </span>
            </div>
            <div className="w-px h-5 bg-gray-700" />
            <div className="flex items-center gap-2">
              <XCircle className="h-5 w-5 text-red-400" />
              <span className="text-sm text-white font-medium">
                {errorCount}{' '}
                <span className="text-gray-400 font-normal">
                  error{errorCount !== 1 ? 's' : ''}
                </span>
              </span>
            </div>

            {/* Overall indicator */}
            <div className="ml-auto">
              {errorCount > 0 ? (
                <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-medium bg-red-900/40 text-red-400 border border-red-700/50">
                  Issues Found
                </span>
              ) : warnCount > 0 ? (
                <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-medium bg-yellow-900/40 text-yellow-400 border border-yellow-700/50">
                  Warnings
                </span>
              ) : (
                <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-medium bg-green-900/40 text-green-400 border border-green-700/50">
                  All Clear
                </span>
              )}
            </div>
          </div>

          {Object.entries(grouped)
            .sort(([a], [b]) => a.localeCompare(b))
            .map(([category, items]) => (
              <div key={category}>
                <h3 className="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-3 capitalize">
                  {category}
                </h3>
                <div className="space-y-2">
                  {items.map((result, idx) => (
                    <div
                      key={`${category}-${idx}`}
                      className={`nh-panel-soft flex items-start gap-3 rounded-lg border p-3 ${severityBorder(
                        result.severity,
                      )} ${severityBg(result.severity)}`}
                    >
                      {severityIcon(result.severity)}
                      <div className="min-w-0">
                        <p className="text-sm text-white">{result.message}</p>
                        <p className="text-xs text-gray-500 mt-0.5 capitalize">
                          {result.severity}
                        </p>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            ))}
        </>
      )}

      {/* Empty state */}
      {!results && !loading && !error && (
        <div className="nh-panel nh-empty-state py-16">
          <span className="nh-icon-well h-14 w-14 rounded-[1.25rem]">
            <Stethoscope className="h-7 w-7" />
          </span>
          <p className="text-lg font-medium text-white">System Diagnostics</p>
          <p className="text-sm mt-1">
            Click "Run Diagnostics" to check your NeoHuman installation.
          </p>
        </div>
      )}
    </div>
  );
}

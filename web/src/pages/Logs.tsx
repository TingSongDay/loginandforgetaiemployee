import { useState, useEffect, useRef, useCallback } from 'react';
import {
  Activity,
  Pause,
  Play,
  ArrowDown,
  Filter,
} from 'lucide-react';
import type { SSEEvent } from '@/types/api';
import { SSEClient } from '@/lib/sse';

function formatTimestamp(ts?: string): string {
  if (!ts) return new Date().toLocaleTimeString();
  return new Date(ts).toLocaleTimeString();
}

function eventTypeBadgeColor(type: string): string {
  switch (type.toLowerCase()) {
    case 'error':
      return 'bg-red-900/50 text-red-400 border-red-700/50';
    case 'warn':
    case 'warning':
      return 'bg-yellow-900/50 text-yellow-400 border-yellow-700/50';
    case 'tool_call':
    case 'tool_result':
      return 'bg-purple-900/50 text-purple-400 border-purple-700/50';
    case 'message':
    case 'chat':
      return 'bg-blue-900/50 text-blue-400 border-blue-700/50';
    case 'health':
    case 'status':
      return 'bg-green-900/50 text-green-400 border-green-700/50';
    default:
      return 'bg-gray-800 text-gray-400 border-gray-700';
  }
}

interface LogEntry {
  id: string;
  event: SSEEvent;
}

export default function Logs() {
  const [entries, setEntries] = useState<LogEntry[]>([]);
  const [paused, setPaused] = useState(false);
  const [connected, setConnected] = useState(false);
  const [autoScroll, setAutoScroll] = useState(true);
  const [typeFilters, setTypeFilters] = useState<Set<string>>(new Set());

  const containerRef = useRef<HTMLDivElement>(null);
  const sseRef = useRef<SSEClient | null>(null);
  const pausedRef = useRef(false);
  const entryIdRef = useRef(0);

  // Keep pausedRef in sync
  useEffect(() => {
    pausedRef.current = paused;
  }, [paused]);

  useEffect(() => {
    const client = new SSEClient();

    client.onConnect = () => {
      setConnected(true);
    };

    client.onError = () => {
      setConnected(false);
    };

    client.onEvent = (event: SSEEvent) => {
      if (pausedRef.current) return;
      entryIdRef.current += 1;
      const entry: LogEntry = {
        id: `log-${entryIdRef.current}`,
        event,
      };
      setEntries((prev) => {
        // Cap at 500 entries for performance
        const next = [...prev, entry];
        return next.length > 500 ? next.slice(-500) : next;
      });
    };

    client.connect();
    sseRef.current = client;

    return () => {
      client.disconnect();
    };
  }, []);

  // Auto-scroll to bottom
  useEffect(() => {
    if (autoScroll && containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [entries, autoScroll]);

  // Detect user scroll to toggle auto-scroll
  const handleScroll = useCallback(() => {
    if (!containerRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = containerRef.current;
    const isAtBottom = scrollHeight - scrollTop - clientHeight < 50;
    setAutoScroll(isAtBottom);
  }, []);

  const jumpToBottom = () => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
    setAutoScroll(true);
  };

  // Collect all event types for filter checkboxes
  const allTypes = Array.from(new Set(entries.map((e) => e.event.type))).sort();

  const toggleTypeFilter = (type: string) => {
    setTypeFilters((prev) => {
      const next = new Set(prev);
      if (next.has(type)) {
        next.delete(type);
      } else {
        next.add(type);
      }
      return next;
    });
  };

  const filteredEntries =
    typeFilters.size === 0
      ? entries
      : entries.filter((e) => typeFilters.has(e.event.type));

  return (
    <div className="nh-page-shell h-[calc(100vh-6.5rem)]">
      <section className="nh-page-hero">
        <div>
          <p className="nh-page-kicker">Live telemetry</p>
          <h2 className="nh-page-title">Logs</h2>
          <p className="nh-page-subtitle">
            Observe the station pulse in real time, filter event types, and keep operator attention
            focused on meaningful runtime changes.
          </p>
        </div>
        <div className={`nh-state-pill ${connected ? 'nh-state-pill-live' : 'nh-state-pill-danger'}`}>
          <span className="nh-state-dot" />
          {connected ? 'Connected' : 'Disconnected'}
        </div>
      </section>

      <div className="nh-panel flex flex-1 min-h-0 flex-col overflow-hidden">
      <div className="flex items-center justify-between px-6 py-3 border-b border-white/8 bg-white/4">
        <div className="flex items-center gap-3">
          <span className="nh-icon-well">
            <Activity className="h-5 w-5" />
          </span>
          <h2 className="text-base font-semibold text-white">Live Logs</h2>
          <div className="flex items-center gap-2 ml-2">
            <span
              className={`inline-block h-2 w-2 rounded-full ${
                connected ? 'bg-green-500' : 'bg-red-500'
              }`}
            />
            <span className="text-xs text-gray-500">
              {connected ? 'Connected' : 'Disconnected'}
            </span>
          </div>
          <span className="text-xs text-gray-500 ml-2">
            {filteredEntries.length} events
          </span>
        </div>

        <div className="flex items-center gap-2">
          {/* Pause/Resume */}
          <button
            onClick={() => setPaused(!paused)}
            className={`flex items-center gap-1.5 text-sm font-medium ${
              paused
                ? 'nh-button-success'
                : 'nh-button-secondary'
            }`}
          >
            {paused ? (
              <>
                <Play className="h-3.5 w-3.5" /> Resume
              </>
            ) : (
              <>
                <Pause className="h-3.5 w-3.5" /> Pause
              </>
            )}
          </button>

          {/* Jump to Bottom */}
          {!autoScroll && (
            <button
              onClick={jumpToBottom}
              className="nh-button-primary"
            >
              <ArrowDown className="h-3.5 w-3.5" />
              Jump to bottom
            </button>
          )}
        </div>
      </div>

      {/* Event type filters */}
      {allTypes.length > 0 && (
        <div className="flex items-center gap-2 px-6 py-2 border-b border-white/8 bg-white/3 overflow-x-auto">
          <Filter className="h-4 w-4 text-gray-500 flex-shrink-0" />
          <span className="text-xs text-gray-500 flex-shrink-0">Filter:</span>
          {allTypes.map((type) => (
            <label
              key={type}
              className="flex items-center gap-1.5 cursor-pointer flex-shrink-0"
            >
              <input
                type="checkbox"
                checked={typeFilters.has(type)}
                onChange={() => toggleTypeFilter(type)}
                className="rounded bg-gray-800 border-gray-600 text-blue-500 focus:ring-blue-500 focus:ring-offset-0 h-3.5 w-3.5"
              />
              <span className="text-xs text-gray-400 capitalize">{type}</span>
            </label>
          ))}
          {typeFilters.size > 0 && (
            <button
              onClick={() => setTypeFilters(new Set())}
              className="text-xs text-blue-400 hover:text-blue-300 flex-shrink-0 ml-1"
            >
              Clear
            </button>
          )}
        </div>
      )}

      {/* Log entries */}
      <div
        ref={containerRef}
        onScroll={handleScroll}
        className="flex-1 overflow-y-auto p-4 space-y-2"
      >
        {filteredEntries.length === 0 ? (
          <div className="nh-empty-state h-full">
            <span className="nh-icon-well">
              <Activity className="h-5 w-5" />
            </span>
            <p className="text-sm">
              {paused
                ? 'Log streaming is paused.'
                : 'Waiting for events...'}
            </p>
          </div>
        ) : (
          filteredEntries.map((entry) => {
            const { event } = entry;
            const detail =
              event.message ??
              event.content ??
              event.data ??
              JSON.stringify(
                Object.fromEntries(
                  Object.entries(event).filter(
                    ([k]) => k !== 'type' && k !== 'timestamp',
                  ),
                ),
              );

            return (
              <div
                key={entry.id}
                className="nh-panel-soft rounded-lg p-3"
              >
                <div className="flex items-start gap-3">
                  <span className="text-xs text-gray-500 font-mono whitespace-nowrap mt-0.5">
                    {formatTimestamp(event.timestamp)}
                  </span>
                  <span
                    className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium border capitalize flex-shrink-0 ${eventTypeBadgeColor(
                      event.type,
                    )}`}
                  >
                    {event.type}
                  </span>
                  <p className="text-sm text-gray-300 break-all min-w-0">
                    {typeof detail === 'string' ? detail : JSON.stringify(detail)}
                  </p>
                </div>
              </div>
            );
          })
        )}
      </div>
      </div>
    </div>
  );
}

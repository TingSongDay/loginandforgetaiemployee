import { useState, useEffect } from 'react';
import {
  Wrench,
  Search,
  ChevronDown,
  ChevronRight,
  Terminal,
  Package,
} from 'lucide-react';
import type { ToolSpec, CliTool } from '@/types/api';
import { getTools, getCliTools } from '@/lib/api';

export default function Tools() {
  const [tools, setTools] = useState<ToolSpec[]>([]);
  const [cliTools, setCliTools] = useState<CliTool[]>([]);
  const [search, setSearch] = useState('');
  const [expandedTool, setExpandedTool] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([getTools(), getCliTools()])
      .then(([t, c]) => {
        setTools(t);
        setCliTools(c);
      })
      .catch((err) => setError(err.message))
      .finally(() => setLoading(false));
  }, []);

  const filtered = tools.filter(
    (t) =>
      t.name.toLowerCase().includes(search.toLowerCase()) ||
      t.description.toLowerCase().includes(search.toLowerCase()),
  );

  const filteredCli = cliTools.filter(
    (t) =>
      t.name.toLowerCase().includes(search.toLowerCase()) ||
      t.category.toLowerCase().includes(search.toLowerCase()),
  );

  if (error) {
    return (
      <div className="nh-page-shell">
        <div className="nh-page-note nh-page-note-danger text-rose-200">
          Failed to load tools: {error}
        </div>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="nh-page-shell flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-2 border-blue-500 border-t-transparent" />
      </div>
    );
  }

  return (
    <div className="nh-page-shell">
      <section className="nh-page-hero">
        <div>
          <p className="nh-page-kicker">Capability registry</p>
          <h2 className="nh-page-title">Tools</h2>
          <p className="nh-page-subtitle">
            Explore the NeoHuman action surface, inspect agent capabilities, and verify local CLI
            utilities without leaving the operator cockpit.
          </p>
        </div>
        <div className="relative max-w-md w-full">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-white/35" />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search tools..."
            className="nh-input pl-10 pr-4 py-3 text-sm"
          />
        </div>
      </section>

      <div>
        <div className="flex items-center gap-3 mb-4">
          <span className="nh-icon-well">
            <Wrench className="h-5 w-5" />
          </span>
          <h2 className="text-base font-semibold text-white">
            Agent Tools ({filtered.length})
          </h2>
        </div>

        {filtered.length === 0 ? (
          <div className="nh-panel nh-empty-state">
            <p className="text-sm">No tools match your search.</p>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
            {filtered.map((tool) => {
              const isExpanded = expandedTool === tool.name;
              return (
                <div
                  key={tool.name}
                  className="nh-panel overflow-hidden"
                >
                  <button
                    onClick={() =>
                      setExpandedTool(isExpanded ? null : tool.name)
                    }
                    className="w-full text-left p-4 hover:bg-white/4 transition-colors"
                  >
                    <div className="flex items-start justify-between gap-2">
                      <div className="flex items-center gap-2 min-w-0">
                        <Package className="h-4 w-4 text-cyan-200 flex-shrink-0 mt-0.5" />
                        <h3 className="text-sm font-semibold text-white truncate">
                          {tool.name}
                        </h3>
                      </div>
                      {isExpanded ? (
                        <ChevronDown className="h-4 w-4 text-gray-400 flex-shrink-0" />
                      ) : (
                        <ChevronRight className="h-4 w-4 text-gray-400 flex-shrink-0" />
                      )}
                    </div>
                    <p className="text-sm text-gray-400 mt-2 line-clamp-2">
                      {tool.description}
                    </p>
                  </button>

                  {isExpanded && tool.parameters && (
                    <div className="border-t border-gray-800 p-4">
                      <p className="text-xs text-white/40 mb-2 font-medium uppercase tracking-wider">
                        Parameter Schema
                      </p>
                      <pre className="text-xs text-gray-300 rounded-xl border border-white/8 bg-black/30 p-3 overflow-x-auto max-h-64 overflow-y-auto">
                        {JSON.stringify(tool.parameters, null, 2)}
                      </pre>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* CLI Tools Section */}
      {filteredCli.length > 0 && (
        <div>
          <div className="flex items-center gap-3 mb-4">
            <span className="nh-icon-well">
              <Terminal className="h-5 w-5" />
            </span>
            <h2 className="text-base font-semibold text-white">
              CLI Tools ({filteredCli.length})
            </h2>
          </div>

          <div className="nh-panel nh-table-shell overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-gray-800">
                  <th className="text-left px-4 py-3 text-gray-400 font-medium">
                    Name
                  </th>
                  <th className="text-left px-4 py-3 text-gray-400 font-medium">
                    Path
                  </th>
                  <th className="text-left px-4 py-3 text-gray-400 font-medium">
                    Version
                  </th>
                  <th className="text-left px-4 py-3 text-gray-400 font-medium">
                    Category
                  </th>
                </tr>
              </thead>
              <tbody>
                {filteredCli.map((tool) => (
                  <tr
                    key={tool.name}
                    className="border-b border-gray-800/50 hover:bg-gray-800/30 transition-colors"
                  >
                    <td className="px-4 py-3 text-white font-medium">
                      {tool.name}
                    </td>
                    <td className="px-4 py-3 text-gray-400 font-mono text-xs truncate max-w-[200px]">
                      {tool.path}
                    </td>
                    <td className="px-4 py-3 text-gray-400">
                      {tool.version ?? '-'}
                    </td>
                    <td className="px-4 py-3">
                      <span className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium border border-white/8 bg-white/6 text-gray-200 capitalize">
                        {tool.category}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}

import { useState, useEffect } from 'react';
import {
  Save,
  CheckCircle,
  AlertTriangle,
  ShieldAlert,
} from 'lucide-react';
import { getConfig, putConfig } from '@/lib/api';

export default function Config() {
  const [config, setConfig] = useState('');
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  useEffect(() => {
    getConfig()
      .then((data) => {
        // The API may return either a raw string or a JSON string
        setConfig(typeof data === 'string' ? data : JSON.stringify(data, null, 2));
      })
      .catch((err) => setError(err.message))
      .finally(() => setLoading(false));
  }, []);

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    setSuccess(null);
    try {
      await putConfig(config);
      setSuccess('Configuration saved successfully.');
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Failed to save configuration');
    } finally {
      setSaving(false);
    }
  };

  // Auto-dismiss success after 4 seconds
  useEffect(() => {
    if (!success) return;
    const timer = setTimeout(() => setSuccess(null), 4000);
    return () => clearTimeout(timer);
  }, [success]);

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
          <p className="nh-page-kicker">Trust envelope</p>
          <h2 className="nh-page-title">Configuration</h2>
          <p className="nh-page-subtitle">
            Shape the station contract, provider behavior, and local runtime settings behind the
            NeoHuman operator surface.
          </p>
        </div>
        <button
          onClick={handleSave}
          disabled={saving}
          className="nh-button-primary"
        >
          <Save className="h-4 w-4" />
          {saving ? 'Saving...' : 'Save'}
        </button>
      </section>

      <div className="nh-page-note nh-page-note-warn">
        <ShieldAlert className="h-5 w-5 text-yellow-400 flex-shrink-0 mt-0.5" />
        <div>
          <p className="text-sm text-yellow-300 font-medium">
            Sensitive fields are masked
          </p>
          <p className="text-sm text-yellow-400/70 mt-0.5">
            API keys, tokens, and passwords are hidden for security. To update a
            masked field, replace the entire masked value with your new value.
          </p>
        </div>
      </div>

      {/* Success message */}
      {success && (
        <div className="nh-page-note nh-page-note-success">
          <CheckCircle className="h-4 w-4 text-green-400 flex-shrink-0" />
          <span className="text-sm text-green-300">{success}</span>
        </div>
      )}

      {/* Error message */}
      {error && (
        <div className="nh-page-note nh-page-note-danger">
          <AlertTriangle className="h-4 w-4 text-red-400 flex-shrink-0" />
          <span className="text-sm text-red-300">{error}</span>
        </div>
      )}

      <div className="nh-panel overflow-hidden">
        <div className="flex items-center justify-between px-4 py-3 border-b border-white/8 bg-white/4">
          <span className="text-xs text-gray-400 font-medium uppercase tracking-wider">
            TOML Configuration
          </span>
          <span className="text-xs text-gray-500">
            {config.split('\n').length} lines
          </span>
        </div>
        <textarea
          value={config}
          onChange={(e) => setConfig(e.target.value)}
          spellCheck={false}
          className="nh-textarea min-h-[500px] rounded-none border-0 bg-black/30 p-4 font-mono text-sm text-gray-200 resize-y focus:ring-0"
          style={{ tabSize: 4 }}
        />
      </div>
    </div>
  );
}

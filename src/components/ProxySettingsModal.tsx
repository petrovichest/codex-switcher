import { useEffect, useState } from "react";
import type { ProxySettingsInfo } from "../types";
import { invokeBackend } from "../lib/platform";

interface ProxySettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  onToast: (message: string, isError?: boolean) => void;
}

export function ProxySettingsModal({
  isOpen,
  onClose,
  onToast,
}: ProxySettingsModalProps) {
  const [settings, setSettings] = useState<ProxySettingsInfo | null>(null);
  const [proxyInput, setProxyInput] = useState("");
  const [enabled, setEnabled] = useState(false);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState(false);
  const [clearing, setClearing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [testSuccess, setTestSuccess] = useState(false);

  useEffect(() => {
    if (!isOpen) return;

    let cancelled = false;
    const loadSettings = async () => {
      try {
        setLoading(true);
        setError(null);
        setTestSuccess(false);
        const next = await invokeBackend<ProxySettingsInfo>("get_proxy_settings");
        if (cancelled) return;
        setSettings(next);
        setEnabled(next.enabled);
        setProxyInput("");
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : String(err));
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    };

    void loadSettings();
    return () => {
      cancelled = true;
    };
  }, [isOpen]);

  if (!isOpen) return null;

  const currentProxyLabel =
    settings?.configured && settings.host && settings.port
      ? `${settings.host}:${settings.port}`
      : "Not configured";
  const currentAuthLabel = settings?.configured
    ? settings.username
      ? `${settings.username}${settings.has_password ? " / password saved" : ""}`
      : "No authentication"
    : "No authentication";

  const handleSave = async () => {
    try {
      setSaving(true);
      setError(null);
      setTestSuccess(false);
      const next = await invokeBackend<ProxySettingsInfo>("set_proxy_settings", {
        proxy: proxyInput.trim() || null,
        enabled,
      });
      setSettings(next);
      setEnabled(next.enabled);
      setProxyInput("");
      onToast(next.enabled ? "Proxy settings saved." : "Proxy disabled.");
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      onToast("Proxy save failed", true);
    } finally {
      setSaving(false);
    }
  };

  const handleTest = async () => {
    try {
      setTesting(true);
      setError(null);
      setTestSuccess(false);
      await invokeBackend("test_proxy_settings", {
        proxy: proxyInput.trim() || null,
      });
      setTestSuccess(true);
      onToast("Proxy test succeeded.");
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      onToast("Proxy test failed", true);
    } finally {
      setTesting(false);
    }
  };

  const handleClear = async () => {
    try {
      setClearing(true);
      setError(null);
      setTestSuccess(false);
      const next = await invokeBackend<ProxySettingsInfo>("clear_proxy_settings");
      setSettings(next);
      setEnabled(false);
      setProxyInput("");
      onToast("Proxy settings cleared.");
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      onToast("Proxy clear failed", true);
    } finally {
      setClearing(false);
    }
  };

  const busy = loading || saving || testing || clearing;

  return (
    <div className="fixed inset-0 bg-black/40 flex items-center justify-center z-50">
      <div className="bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-2xl w-full max-w-lg mx-4 shadow-xl">
        <div className="flex items-center justify-between p-5 border-b border-gray-100 dark:border-gray-800">
          <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
            Proxy Settings
          </h2>
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
            title="Close"
          >
            x
          </button>
        </div>

        <div className="p-5 space-y-4">
          <div className="rounded-lg border border-gray-200 bg-gray-50 p-3 text-sm dark:border-gray-700 dark:bg-gray-800">
            <div className="flex items-center justify-between gap-3">
              <span className="text-gray-500 dark:text-gray-400">Current proxy</span>
              <span className="font-medium text-gray-900 dark:text-gray-100">
                {loading ? "Loading..." : currentProxyLabel}
              </span>
            </div>
            <div className="mt-2 flex items-center justify-between gap-3">
              <span className="text-gray-500 dark:text-gray-400">Authentication</span>
              <span className="font-medium text-gray-900 dark:text-gray-100">
                {loading ? "Loading..." : currentAuthLabel}
              </span>
            </div>
          </div>

          <label className="flex items-center justify-between gap-3 rounded-lg border border-gray-200 px-3 py-2 dark:border-gray-700">
            <span className="text-sm font-medium text-gray-700 dark:text-gray-200">
              Enable proxy
            </span>
            <input
              type="checkbox"
              checked={enabled}
              onChange={(event) => setEnabled(event.target.checked)}
              className="h-4 w-4"
            />
          </label>

          <div>
            <label
              htmlFor="proxy-input"
              className="mb-2 block text-sm font-medium text-gray-700 dark:text-gray-200"
            >
              Proxy string
            </label>
            <input
              id="proxy-input"
              value={proxyInput}
              onChange={(event) => {
                setProxyInput(event.target.value);
                setTestSuccess(false);
              }}
              placeholder="host:port@user:password"
              className="w-full rounded-lg border border-gray-200 bg-gray-50 px-4 py-3 font-mono text-sm text-gray-800 placeholder-gray-400 focus:border-gray-400 focus:outline-none focus:ring-1 focus:ring-gray-400 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-100 dark:placeholder-gray-500 dark:focus:border-gray-500 dark:focus:ring-gray-500"
            />
            <p className="mt-2 text-xs text-gray-500 dark:text-gray-400">
              Example: host:port@username:password. Saved proxy settings are normalized.
            </p>
          </div>

          {testSuccess && (
            <div className="rounded-lg border border-green-200 bg-green-50 p-3 text-sm text-green-700 dark:border-green-700 dark:bg-green-900/20 dark:text-green-300">
              Proxy test succeeded.
            </div>
          )}

          {error && (
            <div className="rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-600 dark:border-red-700 dark:bg-red-900/20 dark:text-red-300">
              {error}
            </div>
          )}
        </div>

        <div className="flex flex-wrap gap-3 p-5 border-t border-gray-100 dark:border-gray-800">
          <button
            onClick={onClose}
            className="px-4 py-2.5 text-sm font-medium rounded-lg bg-gray-100 hover:bg-gray-200 dark:bg-gray-800 dark:hover:bg-gray-700 text-gray-700 dark:text-gray-200 transition-colors"
          >
            Close
          </button>
          <button
            onClick={handleTest}
            disabled={busy || (!proxyInput.trim() && !settings?.configured)}
            className="px-4 py-2.5 text-sm font-medium rounded-lg bg-gray-100 hover:bg-gray-200 disabled:opacity-50 dark:bg-gray-800 dark:hover:bg-gray-700 text-gray-700 dark:text-gray-200 transition-colors"
          >
            {testing ? "Testing..." : "Test"}
          </button>
          <button
            onClick={handleClear}
            disabled={busy || !settings?.configured}
            className="px-4 py-2.5 text-sm font-medium rounded-lg bg-red-50 hover:bg-red-100 disabled:opacity-50 dark:bg-red-900/20 dark:hover:bg-red-900/30 text-red-700 dark:text-red-300 transition-colors"
          >
            {clearing ? "Clearing..." : "Clear"}
          </button>
          <button
            onClick={handleSave}
            disabled={busy}
            className="px-4 py-2.5 text-sm font-medium rounded-lg bg-gray-900 hover:bg-gray-800 dark:bg-gray-100 dark:hover:bg-gray-200 text-white dark:text-gray-900 transition-colors disabled:opacity-50"
          >
            {saving ? "Saving..." : "Save"}
          </button>
        </div>
      </div>
    </div>
  );
}

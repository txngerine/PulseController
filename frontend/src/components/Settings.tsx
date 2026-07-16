import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Settings {
  general: {
    auto_launch: boolean;
    run_in_tray: boolean;
    minimize_to_tray: boolean;
    check_updates: boolean;
    update_channel: string;
  };
  network: {
    preferred_transport: string;
    udp_port: number;
    udp_bind_address: string;
    bluetooth_enabled: boolean;
    auto_discover: boolean;
  };
  input: {
    mouse_sensitivity: number;
    mouse_acceleration: boolean;
    keyboard_repeat_rate: number;
    keyboard_repeat_delay: number;
    default_deadzone: number;
  };
  appearance: {
    theme: string;
  };
}

export function Settings() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [saved, setSaved] = useState(false);

  const load = async () => {
    try {
      const str = await invoke<string>("get_settings");
      setSettings(JSON.parse(str));
    } catch (e) {
      console.error("load settings error:", e);
    }
  };

  const save = async () => {
    if (!settings) return;
    try {
      await invoke("update_settings", { settingsJson: JSON.stringify(settings) });
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      console.error("save error:", e);
    }
  };

  const reset = async () => {
    try {
      await invoke("reset_settings");
      load();
    } catch (e) {
      console.error("reset error:", e);
    }
  };

  useEffect(() => { load(); }, []);

  if (!settings) {
    return (
      <div className="page">
        <div className="page-header">
          <h1 className="page-title">Settings</h1>
        </div>
        <div className="empty-state">
          <div className="empty-title">Loading settings...</div>
        </div>
      </div>
    );
  }

  const update = (section: keyof Settings, key: string, value: unknown) => {
    setSettings((prev) => {
      if (!prev) return prev;
      return {
        ...prev,
        [section]: { ...prev[section], [key]: value },
      };
    });
  };

  return (
    <div className="page">
      <div className="page-header flex-between">
        <div>
          <h1 className="page-title">Settings</h1>
          <p className="page-subtitle">Configure PulsePad behavior</p>
        </div>
        <div className="btn-group">
          <button className="btn btn-primary" onClick={save}>
            {saved ? "Saved!" : "Save"}
          </button>
          <button className="btn" onClick={reset}>Reset</button>
        </div>
      </div>

      <div className="settings-grid">
        {/* General */}
        <div className="setting-group">
          <div className="setting-group-header">
            <div className="setting-group-title">General</div>
            <div className="setting-group-desc">Application startup and behavior</div>
          </div>
          <div className="setting-row">
            <div className="setting-label">Auto Launch<small>Start PulsePad on system boot</small></div>
            <input type="checkbox" checked={settings.general.auto_launch} onChange={(e) => update("general", "auto_launch", e.target.checked)} />
          </div>
          <div className="setting-row">
            <div className="setting-label">Run in Tray<small>Keep running when window is closed</small></div>
            <input type="checkbox" checked={settings.general.run_in_tray} onChange={(e) => update("general", "run_in_tray", e.target.checked)} />
          </div>
          <div className="setting-row">
            <div className="setting-label">Check for Updates<small>Automatically check for new versions</small></div>
            <input type="checkbox" checked={settings.general.check_updates} onChange={(e) => update("general", "check_updates", e.target.checked)} />
          </div>
          <div className="setting-row">
            <div className="setting-label">Update Channel</div>
            <select value={settings.general.update_channel} onChange={(e) => update("general", "update_channel", e.target.value)}>
              <option value="stable">Stable</option>
              <option value="beta">Beta</option>
              <option value="dev">Dev</option>
            </select>
          </div>
        </div>

        {/* Network */}
        <div className="setting-group">
          <div className="setting-group-header">
            <div className="setting-group-title">Network</div>
            <div className="setting-group-desc">Transport and discovery settings</div>
          </div>
          <div className="setting-row">
            <div className="setting-label">Preferred Transport</div>
            <select value={settings.network.preferred_transport} onChange={(e) => update("network", "preferred_transport", e.target.value)}>
              <option value="udp">UDP</option>
              <option value="bluetooth">Bluetooth</option>
            </select>
          </div>
          <div className="setting-row">
            <div className="setting-label">UDP Port<small>Listening port for device connections</small></div>
            <input type="number" value={settings.network.udp_port} onChange={(e) => update("network", "udp_port", parseInt(e.target.value) || 9876)} style={{ width: 80 }} />
          </div>
          <div className="setting-row">
            <div className="setting-label">Bluetooth<small>Enable Bluetooth LE transport</small></div>
            <input type="checkbox" checked={settings.network.bluetooth_enabled} onChange={(e) => update("network", "bluetooth_enabled", e.target.checked)} />
          </div>
          <div className="setting-row">
            <div className="setting-label">Auto Discover<small>Scan for devices on startup</small></div>
            <input type="checkbox" checked={settings.network.auto_discover} onChange={(e) => update("network", "auto_discover", e.target.checked)} />
          </div>
        </div>

        {/* Input */}
        <div className="setting-group">
          <div className="setting-group-header">
            <div className="setting-group-title">Input</div>
            <div className="setting-group-desc">Mouse, keyboard, and controller behavior</div>
          </div>
          <div className="setting-row">
            <div className="setting-label">Mouse Sensitivity<small>Controls cursor speed multiplier</small></div>
            <div className="setting-control">
              <input type="range" min="0.1" max="5" step="0.1" value={settings.input.mouse_sensitivity} onChange={(e) => update("input", "mouse_sensitivity", parseFloat(e.target.value))} style={{ width: 120 }} />
              <span className="value">{settings.input.mouse_sensitivity.toFixed(1)}x</span>
            </div>
          </div>
          <div className="setting-row">
            <div className="setting-label">Mouse Acceleration<small>Apply acceleration curve to mouse movement</small></div>
            <input type="checkbox" checked={settings.input.mouse_acceleration} onChange={(e) => update("input", "mouse_acceleration", e.target.checked)} />
          </div>
          <div className="setting-row">
            <div className="setting-label">Joystick Deadzone<small>Ignore small stick movements</small></div>
            <div className="setting-control">
              <input type="range" min="0" max="32767" step="100" value={settings.input.default_deadzone} onChange={(e) => update("input", "default_deadzone", parseInt(e.target.value))} style={{ width: 120 }} />
              <span className="value">{settings.input.default_deadzone}</span>
            </div>
          </div>
        </div>

        {/* Appearance */}
        <div className="setting-group">
          <div className="setting-group-header">
            <div className="setting-group-title">Appearance</div>
            <div className="setting-group-desc">Visual theme and display options</div>
          </div>
          <div className="setting-row">
            <div className="setting-label">Theme</div>
            <select value={settings.appearance.theme} onChange={(e) => update("appearance", "theme", e.target.value)}>
              <option value="dark">Dark</option>
              <option value="light">Light</option>
              <option value="auto">System</option>
            </select>
          </div>
        </div>
      </div>
    </div>
  );
}

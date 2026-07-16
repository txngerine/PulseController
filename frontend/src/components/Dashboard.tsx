import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Metrics {
  packet_rate: number;
  dropped_packets: number;
  total_packets: number;
  avg_latency_ms: number;
  min_latency_ms: number;
  max_latency_ms: number;
  connection_uptime_secs: number;
  memory_usage_mb: number;
  cpu_usage_percent: number;
}

export function Dashboard() {
  const [metrics, setMetrics] = useState<Metrics | null>(null);
  const [connected, setConnected] = useState(false);
  const [deviceName, setDeviceName] = useState<string | null>(null);

  useEffect(() => {
    const poll = async () => {
      try {
        const metricsStr = await invoke<string>("get_metrics");
        setMetrics(JSON.parse(metricsStr));
        const statusStr = await invoke<string>("get_connection_status");
        const status = JSON.parse(statusStr);
        setConnected(status.connected || false);
        setDeviceName(status.device_name);
      } catch (e) {
        console.error("poll error:", e);
      }
    };
    poll();
    const id = setInterval(poll, 1000);
    return () => clearInterval(id);
  }, []);

  const uptime = metrics?.connection_uptime_secs || 0;
  const uptimeStr = uptime > 0
    ? `${Math.floor(uptime / 3600)}h ${Math.floor((uptime % 3600) / 60)}m`
    : "--";

  return (
    <div className="page">
      <div className="page-header">
        <h1 className="page-title">Dashboard</h1>
        <p className="page-subtitle">Real-time connection overview</p>
      </div>

      <div className="connection-hero">
        <div className="hero-left">
          <div className={`hero-icon ${connected ? "connected" : "disconnected"}`}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M5 12.55a11 11 0 0 1 14.08 0" />
              <path d="M1.42 9a16 16 0 0 1 21.16 0" />
              <path d="M8.53 16.11a6 6 0 0 1 6.95 0" />
              <line x1="12" y1="20" x2="12.01" y2="20" />
            </svg>
          </div>
          <div className="hero-info">
            <h2>{connected ? deviceName || "Connected" : "No Device"}</h2>
            <p>{connected ? "Receiving input data" : "Connect a device to get started"}</p>
          </div>
        </div>
        <span className={`status-badge ${connected ? "connected" : "disconnected"}`}>
          <span className="dot" />
          {connected ? "Connected" : "Disconnected"}
        </span>
      </div>

      <div className="stats-grid">
        <div className="stat-card">
          <div className="stat-icon cyan">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <polyline points="22 12 18 12 15 21 9 3 6 12 2 12" />
            </svg>
          </div>
          <div className="stat-label">Latency</div>
          <div className="stat-value">
            {metrics?.avg_latency_ms.toFixed(1) || "0.0"}
            <span className="stat-unit">ms</span>
          </div>
        </div>

        <div className="stat-card">
          <div className="stat-icon green">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <polyline points="23 6 13.5 15.5 8.5 10.5 1 18" />
              <polyline points="17 6 23 6 23 12" />
            </svg>
          </div>
          <div className="stat-label">Packet Rate</div>
          <div className="stat-value">
            {metrics?.packet_rate.toFixed(0) || "0"}
            <span className="stat-unit">pps</span>
          </div>
        </div>

        <div className="stat-card">
          <div className="stat-icon purple">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <circle cx="12" cy="12" r="10" />
              <polyline points="12 6 12 12 16 14" />
            </svg>
          </div>
          <div className="stat-label">Uptime</div>
          <div className="stat-value">{uptimeStr}</div>
        </div>

        <div className="stat-card">
          <div className="stat-icon red">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
              <line x1="12" y1="9" x2="12" y2="13" />
              <line x1="12" y1="17" x2="12.01" y2="17" />
            </svg>
          </div>
          <div className="stat-label">Dropped</div>
          <div className="stat-value">{metrics?.dropped_packets || 0}</div>
        </div>

        <div className="stat-card">
          <div className="stat-icon amber">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
              <line x1="8" y1="21" x2="16" y2="21" />
              <line x1="12" y1="17" x2="12" y2="21" />
            </svg>
          </div>
          <div className="stat-label">Memory</div>
          <div className="stat-value">
            {metrics?.memory_usage_mb.toFixed(1) || "0.0"}
            <span className="stat-unit">MB</span>
          </div>
        </div>

        <div className="stat-card">
          <div className="stat-icon green">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <polyline points="20 6 9 17 4 12" />
            </svg>
          </div>
          <div className="stat-label">Total Packets</div>
          <div className="stat-value">{metrics?.total_packets || 0}</div>
        </div>
      </div>
    </div>
  );
}

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface DiscoveredDevice {
  id: string;
  name: string;
  address: string;
  port: number;
  transport: string;
  signal_strength: number | null;
  os: string | null;
  last_seen: number;
}

export function Devices() {
  const [devices, setDevices] = useState<DiscoveredDevice[]>([]);
  const [discovering, setDiscovering] = useState(false);
  const [ip, setIp] = useState("");
  const [port, setPort] = useState("9876");

  const toggleDiscovery = async () => {
    try {
      if (discovering) {
        await invoke("stop_discovery");
      } else {
        await invoke("start_discovery");
      }
      setDiscovering(!discovering);
    } catch (e) {
      console.error("discovery error:", e);
    }
  };

  const connectTo = async (address: string, devicePort: number) => {
    try {
      await invoke("connect_device", { address, port: devicePort.toString() });
    } catch (e) {
      console.error("connect error:", e);
    }
  };

  const disconnect = async () => {
    try {
      await invoke("disconnect_device");
    } catch (e) {
      console.error("disconnect error:", e);
    }
  };

  useEffect(() => {
    const poll = setInterval(async () => {
      try {
        const str = await invoke<string>("get_discovered_devices");
        setDevices(JSON.parse(str));
      } catch (e) {
        console.error("poll devices error:", e);
      }
    }, 2000);
    return () => clearInterval(poll);
  }, []);

  return (
    <div className="page">
      <div className="page-header flex-between">
        <div>
          <h1 className="page-title">Devices</h1>
          <p className="page-subtitle">Discover and connect to your phone</p>
        </div>
        <button className={`btn ${discovering ? "btn-danger" : "btn-primary"}`} onClick={toggleDiscovery}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            {discovering ? (
              <><line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" /></>
            ) : (
              <><circle cx="11" cy="11" r="8" /><line x1="21" y1="21" x2="16.65" y2="16.65" /></>
            )}
          </svg>
          {discovering ? "Stop Scanning" : "Start Scanning"}
        </button>
      </div>

      <div className="card mb-24">
        <div className="card-header">
          <span className="card-title">Manual Connection</span>
        </div>
        <div className="card-body" style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <input
            type="text"
            placeholder="IP Address"
            value={ip}
            onChange={(e) => setIp(e.target.value)}
            style={{ flex: 1 }}
          />
          <input
            type="text"
            placeholder="Port"
            value={port}
            onChange={(e) => setPort(e.target.value)}
            style={{ width: 80 }}
          />
          <button className="btn btn-primary" onClick={() => connectTo(ip, parseInt(port))}>
            Connect
          </button>
          <button className="btn btn-danger" onClick={disconnect}>
            Disconnect
          </button>
        </div>
      </div>

      <div className="card">
        <div className="card-header">
          <span className="card-title">Discovered Devices</span>
          <span style={{ fontSize: 12, color: "var(--text-muted)" }}>{devices.length} found</span>
        </div>
        <div className="card-body">
          {devices.length === 0 ? (
            <div className="empty-state">
              <div className="empty-icon">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <rect x="5" y="2" width="14" height="20" rx="2" ry="2" />
                  <line x1="12" y1="18" x2="12.01" y2="18" />
                </svg>
              </div>
              <div className="empty-title">No devices found</div>
              <div className="empty-desc">Start scanning or enter an IP manually</div>
            </div>
          ) : (
            <div className="item-list">
              {devices.map((d) => (
                <div key={d.id} className="item-card">
                  <div className="item-left">
                    <div className="item-icon">
                      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <rect x="5" y="2" width="14" height="20" rx="2" ry="2" />
                        <line x1="12" y1="18" x2="12.01" y2="18" />
                      </svg>
                    </div>
                    <div className="item-details">
                      <div className="item-name">{d.name}</div>
                      <div className="item-meta">{d.address}:{d.port} &middot; {d.transport}</div>
                    </div>
                  </div>
                  <div className="item-actions">
                    <button className="btn btn-sm btn-primary" onClick={() => connectTo(d.address, d.port)}>
                      Connect
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

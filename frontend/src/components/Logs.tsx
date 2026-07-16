import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

interface LogEntry {
  timestamp: string;
  level: string;
  message: string;
}

export function Logs() {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [filter, setFilter] = useState("all");
  const endRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const fetch = async () => {
      try {
        const f = filter === "all" ? null : filter;
        const str = await invoke<string>("get_logs", { filter: f });
        setLogs(JSON.parse(str));
      } catch (e) {
        console.error("fetch logs error:", e);
      }
    };
    fetch();
    const id = setInterval(fetch, 2000);
    return () => clearInterval(id);
  }, [filter]);

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  const clear = async () => {
    try {
      await invoke("clear_logs");
      setLogs([]);
    } catch (e) {
      console.error("clear error:", e);
    }
  };

  const copy = () => {
    const text = logs.map((l) => `[${l.timestamp}] [${l.level}] ${l.message}`).join("\n");
    navigator.clipboard.writeText(text);
  };

  return (
    <div className="page">
      <div className="page-header flex-between">
        <div>
          <h1 className="page-title">Logs</h1>
          <p className="page-subtitle">Application event log</p>
        </div>
        <div className="btn-group">
          <select value={filter} onChange={(e) => setFilter(e.target.value)}>
            <option value="all">All Levels</option>
            <option value="info">Info</option>
            <option value="warn">Warning</option>
            <option value="error">Error</option>
            <option value="debug">Debug</option>
          </select>
          <button className="btn btn-sm" onClick={copy}>Copy</button>
          <button className="btn btn-sm btn-danger" onClick={clear}>Clear</button>
        </div>
      </div>

      <div className="log-container">
        <div className="log-scroll">
          {logs.length === 0 ? (
            <div className="log-empty">No log entries</div>
          ) : (
            logs.map((log, i) => (
              <div key={i} className="log-entry">
                <span className="log-time">{log.timestamp}</span>
                <span className={`log-level ${log.level}`}>{log.level}</span>
                <span className="log-msg">{log.message}</span>
              </div>
            ))
          )}
          <div ref={endRef} />
        </div>
      </div>
    </div>
  );
}

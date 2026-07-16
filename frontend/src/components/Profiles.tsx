import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Profile {
  id: string;
  name: string;
  description: string;
  is_default: boolean;
}

export function Profiles() {
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [newName, setNewName] = useState("");
  const [newDesc, setNewDesc] = useState("");
  const [showCreate, setShowCreate] = useState(false);

  const load = async () => {
    try {
      const str = await invoke<string>("list_profiles");
      setProfiles(JSON.parse(str));
    } catch (e) {
      console.error("load profiles error:", e);
    }
  };

  const create = async () => {
    if (!newName.trim()) return;
    try {
      await invoke("create_profile", { name: newName, description: newDesc });
      setNewName("");
      setNewDesc("");
      setShowCreate(false);
      load();
    } catch (e) {
      console.error("create error:", e);
    }
  };

  const remove = async (id: string) => {
    try {
      await invoke("delete_profile", { id });
      load();
    } catch (e) {
      console.error("delete error:", e);
    }
  };

  const activate = async (id: string) => {
    try {
      await invoke("set_active_profile", { id });
      setActiveId(id);
    } catch (e) {
      console.error("activate error:", e);
    }
  };

  useEffect(() => { load(); }, []);

  return (
    <div className="page">
      <div className="page-header flex-between">
        <div>
          <h1 className="page-title">Profiles</h1>
          <p className="page-subtitle">Manage controller input mappings</p>
        </div>
        <button className="btn btn-primary" onClick={() => setShowCreate(!showCreate)}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <line x1="12" y1="5" x2="12" y2="19" />
            <line x1="5" y1="12" x2="19" y2="12" />
          </svg>
          New Profile
        </button>
      </div>

      {showCreate && (
        <div className="card mb-24">
          <div className="card-header">
            <span className="card-title">Create Profile</span>
          </div>
          <div className="card-body" style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            <input
              type="text"
              placeholder="Profile name"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
            />
            <input
              type="text"
              placeholder="Description (optional)"
              value={newDesc}
              onChange={(e) => setNewDesc(e.target.value)}
            />
            <div className="btn-group">
              <button className="btn btn-primary" onClick={create}>Create</button>
              <button className="btn" onClick={() => setShowCreate(false)}>Cancel</button>
            </div>
          </div>
        </div>
      )}

      <div className="card">
        <div className="card-header">
          <span className="card-title">Profiles</span>
          <span style={{ fontSize: 12, color: "var(--text-muted)" }}>{profiles.length} total</span>
        </div>
        <div className="card-body">
          {profiles.length === 0 ? (
            <div className="empty-state">
              <div className="empty-icon">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <circle cx="12" cy="12" r="3" />
                  <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06A1.65 1.65 0 0 0 15 19.4a1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
                </svg>
              </div>
              <div className="empty-title">No profiles yet</div>
              <div className="empty-desc">Create a profile to customize your controller mappings</div>
            </div>
          ) : (
            <div className="item-list">
              {profiles.map((p) => (
                <div key={p.id} className={`item-card ${activeId === p.id ? "active" : ""}`}>
                  <div className="item-left">
                    <div className="item-icon">
                      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <circle cx="12" cy="12" r="3" />
                        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06A1.65 1.65 0 0 0 15 19.4a1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4" />
                      </svg>
                    </div>
                    <div className="item-details">
                      <div className="item-name">
                        {p.name}
                        {p.is_default && <span style={{ fontSize: 10, color: "var(--amber)", marginLeft: 6, fontWeight: 500 }}>DEFAULT</span>}
                      </div>
                      <div className="item-meta">{p.description || "No description"}</div>
                    </div>
                  </div>
                  <div className="item-actions">
                    <button
                      className={`btn btn-sm ${activeId === p.id ? "btn-success" : ""}`}
                      onClick={() => activate(p.id)}
                    >
                      {activeId === p.id ? "Active" : "Activate"}
                    </button>
                    <button className="btn btn-sm btn-danger" onClick={() => remove(p.id)}>
                      Delete
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

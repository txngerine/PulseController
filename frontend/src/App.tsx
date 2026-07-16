import { useState } from "react";
import { Sidebar } from "./components/Sidebar";
import { Dashboard } from "./components/Dashboard";
import { Devices } from "./components/Devices";
import { Profiles } from "./components/Profiles";
import { Settings } from "./components/Settings";
import { Logs } from "./components/Logs";

type Page = "dashboard" | "devices" | "profiles" | "settings" | "logs";

function App() {
  const [currentPage, setCurrentPage] = useState<Page>("dashboard");
  const [collapsed, setCollapsed] = useState(false);

  const renderPage = () => {
    switch (currentPage) {
      case "dashboard": return <Dashboard />;
      case "devices": return <Devices />;
      case "profiles": return <Profiles />;
      case "settings": return <Settings />;
      case "logs": return <Logs />;
      default: return <Dashboard />;
    }
  };

  return (
    <div className="app">
      <Sidebar
        currentPage={currentPage}
        onNavigate={setCurrentPage}
        collapsed={collapsed}
        onToggleCollapse={() => setCollapsed(!collapsed)}
      />
      <main className="main-content">
        {renderPage()}
      </main>
    </div>
  );
}

export default App;

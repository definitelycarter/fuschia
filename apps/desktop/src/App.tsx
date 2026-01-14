import { useState, useCallback, useMemo } from "react";
import { WorkflowCanvas } from "./components/WorkflowCanvas";
import { Drawer } from "./components/Drawer";
import { WorkflowContext } from "./components/WorkflowContext";

function App() {
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [sourceNodeId, setSourceNodeId] = useState<string | null>(null);

  const onAddNode = useCallback((nodeId: string) => {
    setSourceNodeId(nodeId);
    setDrawerOpen(true);
  }, []);

  const contextValue = useMemo(() => ({ onAddNode }), [onAddNode]);

  return (
    <WorkflowContext.Provider value={contextValue}>
      <div className="h-full flex flex-col bg-[var(--color-bg-base)]">
        {/* Header */}
        <header className="flex items-center justify-between px-4 py-3 border-b border-[var(--color-border-default)] bg-[var(--color-bg-surface)]">
          <h1 className="text-lg font-semibold text-[var(--color-text-primary)]">
            Fuchsia
          </h1>
          <div className="flex gap-2">
            <button className="px-3 py-1.5 text-sm rounded-md bg-primary-500 text-white hover:bg-primary-600 transition-colors">
              New Workflow
            </button>
          </div>
        </header>

        {/* Main content */}
        <div className="flex flex-1 overflow-hidden relative">
          {/* Sidebar */}
          <aside className="w-56 border-r border-[var(--color-border-default)] bg-[var(--color-bg-surface)] p-4">
            <nav className="space-y-1">
              <a
                href="#"
                className="flex items-center gap-2 px-3 py-2 text-sm rounded-md bg-primary-500/10 text-primary-500"
              >
                Workflows
              </a>
              <a
                href="#"
                className="flex items-center gap-2 px-3 py-2 text-sm rounded-md text-[var(--color-text-secondary)] hover:bg-[var(--color-bg-elevated)]"
              >
                Components
              </a>
              <a
                href="#"
                className="flex items-center gap-2 px-3 py-2 text-sm rounded-md text-[var(--color-text-secondary)] hover:bg-[var(--color-bg-elevated)]"
              >
                Executions
              </a>
            </nav>
          </aside>

          {/* Canvas area */}
          <main className="flex-1 bg-[var(--color-bg-base)]">
            <WorkflowCanvas />
          </main>

          {/* Right drawer */}
          <Drawer
            open={drawerOpen}
            onClose={() => setDrawerOpen(false)}
            title="Add Node"
          >
            <div className="space-y-4">
              <p className="text-sm text-[var(--color-text-secondary)]">
                Adding node after:{" "}
                <span className="font-mono text-[var(--color-text-primary)]">
                  {sourceNodeId}
                </span>
              </p>
              <div className="space-y-2">
                <div className="p-3 rounded-md border border-[var(--color-border-default)] hover:bg-[var(--color-bg-elevated)] cursor-pointer transition-colors">
                  <p className="text-sm font-medium text-[var(--color-text-primary)]">
                    Google Sheets
                  </p>
                  <p className="text-xs text-[var(--color-text-muted)]">
                    Read and write spreadsheet data
                  </p>
                </div>
                <div className="p-3 rounded-md border border-[var(--color-border-default)] hover:bg-[var(--color-bg-elevated)] cursor-pointer transition-colors">
                  <p className="text-sm font-medium text-[var(--color-text-primary)]">
                    Email
                  </p>
                  <p className="text-xs text-[var(--color-text-muted)]">
                    Send emails via SMTP
                  </p>
                </div>
                <div className="p-3 rounded-md border border-[var(--color-border-default)] hover:bg-[var(--color-bg-elevated)] cursor-pointer transition-colors">
                  <p className="text-sm font-medium text-[var(--color-text-primary)]">
                    HTTP Request
                  </p>
                  <p className="text-xs text-[var(--color-text-muted)]">
                    Make HTTP requests to any API
                  </p>
                </div>
              </div>
            </div>
          </Drawer>
        </div>

        {/* Status bar */}
        <footer className="flex items-center justify-between px-4 py-1.5 text-xs border-t border-[var(--color-border-default)] bg-[var(--color-bg-surface)] text-[var(--color-text-muted)]">
          <span>Ready</span>
          <span>v0.1.0</span>
        </footer>
      </div>
    </WorkflowContext.Provider>
  );
}

export default App;

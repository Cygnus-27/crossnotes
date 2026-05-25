import "./index.css";
import { AppShell } from "./components/layout/AppShell";
import { useShortcuts } from "./hooks/useShortcuts";
import { useVault } from "./hooks/useVault";
import { useNoteStore } from "./store/noteStore";
import { Editor } from "./components/editor/Editor";
import { CommandPalette } from "./components/palette/CommandPalette";
import { QuickOpen } from "./components/palette/QuickOpen";
import { GlobalSearch } from "./components/palette/GlobalSearch";
import { useCommands } from "./hooks/useCommands";
import { TabBar } from "./components/layout/TabBar";
import { Header } from "./components/layout/Header";
import { ShortcutCheatSheet } from "./components/ui/ShortcutCheatSheet";
import { useUIStore } from "./store/uiStore";

function App() {
  const { openVault, vaultPath } = useVault();
  const activeNote = useNoteStore((state) => state.activeNote);
  const {
    quickOpenOpen,
    setQuickOpenOpen,
    helpOpen,
    setHelpOpen,
    globalSearchOpen,
    setGlobalSearchOpen,
  } = useUIStore();

  useCommands();
  useShortcuts({ onOpenVault: openVault });

  return (
    <AppShell>
      <CommandPalette />
      <QuickOpen isOpen={quickOpenOpen} setIsOpen={setQuickOpenOpen} />
      <GlobalSearch isOpen={globalSearchOpen} setIsOpen={setGlobalSearchOpen} />
      <ShortcutCheatSheet
        isOpen={helpOpen}
        onClose={() => setHelpOpen(false)}
      />
      <Header />
      <TabBar />
      {activeNote ? (
        <div
          style={{
            flex: 1,
            minHeight: 0,
            display: "flex",
            flexDirection: "column",
          }}
        >
          <Editor />
        </div>
      ) : (
        <div className="empty-state">
          <div className="empty-card">
            <span className="empty-badge">Local-first workspace</span>
            <div
              style={{ display: "flex", flexDirection: "column", gap: "10px" }}
            >
              <h1 className="empty-title">Write faster, sync smarter.</h1>
              <p className="empty-subtitle">
                CrossNotes keeps your markdown notes local, keyboard-first, and
                ready for future LAN sync between desktop and mobile.
              </p>
            </div>
            <div className="empty-actions">
              {!vaultPath && (
                <button
                  type="button"
                  className="primary-button"
                  onClick={openVault}
                >
                  Open Vault
                </button>
              )}
              <button
                type="button"
                className="secondary-button"
                onClick={() => setHelpOpen(true)}
              >
                View shortcuts
              </button>
            </div>
            <div className="empty-shortcuts">
              {!vaultPath && (
                <div className="empty-shortcut">
                  <span className="keyboard-hint">Ctrl + O</span>
                  <span>Open a vault</span>
                </div>
              )}
              <div className="empty-shortcut">
                <span className="keyboard-hint">Ctrl + K</span>
                <span>Run commands</span>
              </div>
              <div className="empty-shortcut">
                <span className="keyboard-hint">Ctrl + ?</span>
                <span>Open help</span>
              </div>
            </div>
          </div>
        </div>
      )}
    </AppShell>
  );
}

export default App;

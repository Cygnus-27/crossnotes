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
  const { openVault, vaultPath, createNote } = useVault();
  const activeNote = useNoteStore((state) => state.activeNote);
  const notes = useNoteStore((state) => state.notes);
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

  const showLanding = !activeNote;
  const hasVault = Boolean(vaultPath);
  const hasNotes = notes.length > 0;

  const promptForNewNote = () => {
    const name = prompt("Note name:");
    if (name?.trim()) createNote(name.trim());
  };

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
      {!showLanding ? (
        <div className="editor-stage">
          <Editor />
        </div>
      ) : (
        <div className="empty-state">
          <section className="empty-page">
            <div className="empty-kicker">CrossNotes</div>
            <h1 className="empty-title">
              {hasVault && !hasNotes
                ? "A quiet vault, waiting for its first note."
                : "A quiet workspace for markdown notes."}
            </h1>
            <p className="empty-subtitle">
              {hasVault && !hasNotes
                ? "This folder is connected, but it does not contain any .md files yet. Add a markdown file here or switch to a vault that already has notes."
                : "Open a folder of markdown files and keep your writing local. The sync layer is being designed around deliberate device-to-device transfer, not a cloud account."}
            </p>

            <div className="empty-actions">
              <button
                type="button"
                className="primary-button"
                onClick={hasVault && !hasNotes ? promptForNewNote : openVault}
              >
                {hasVault && !hasNotes
                  ? "Create first note"
                  : hasVault
                    ? "Choose another vault"
                    : "Open vault"}
              </button>
              {hasVault && !hasNotes && (
                <button
                  type="button"
                  className="secondary-button"
                  onClick={openVault}
                >
                  Choose another vault
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

            <div className="empty-grid">
              <article className="empty-panel">
                <span className="panel-index">01</span>
                <h2>Local first</h2>
                <p>
                  Your vault is just a folder. Notes stay readable as plain
                  markdown files.
                </p>
              </article>
              <article className="empty-panel">
                <span className="panel-index">02</span>
                <h2>Keyboard ready</h2>
                <p>
                  Use quick open, global search, command palette, and Vim
                  editing without leaving the flow.
                </p>
              </article>
              <article className="empty-panel">
                <span className="panel-index">03</span>
                <h2>Sync direction</h2>
                <p>
                  Next up: pair nearby devices and exchange changes directly
                  over your local network.
                </p>
              </article>
            </div>

            <div className="empty-shortcuts">
              <div className="empty-shortcut">
                <span className="keyboard-hint">Ctrl + O</span>
                <span>{hasVault ? "Switch vault" : "Open a vault"}</span>
              </div>
              <div className="empty-shortcut">
                <span className="keyboard-hint">Ctrl + P</span>
                <span>Quick open</span>
              </div>
              <div className="empty-shortcut">
                <span className="keyboard-hint">Ctrl + K</span>
                <span>Commands</span>
              </div>
            </div>
          </section>
        </div>
      )}
    </AppShell>
  );
}

export default App;

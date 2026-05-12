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
  const { quickOpenOpen, setQuickOpenOpen, helpOpen, setHelpOpen, globalSearchOpen, setGlobalSearchOpen } = useUIStore();
  
  useCommands();
  useShortcuts({ onOpenVault: openVault });

  return (
    <AppShell>
      <CommandPalette />
      <QuickOpen isOpen={quickOpenOpen} setIsOpen={setQuickOpenOpen} />
      <GlobalSearch isOpen={globalSearchOpen} setIsOpen={setGlobalSearchOpen} />
      <ShortcutCheatSheet isOpen={helpOpen} onClose={() => setHelpOpen(false)} />
      <Header />
      <TabBar />
      {activeNote ? (
        <div style={{ flex: 1, height: '100%', display: 'flex', flexDirection: 'column' }}>
          <Editor />
        </div>
      ) : (
        <div style={{ 
          flex: 1, 
          display: 'flex', 
          alignItems: 'center', 
          justifyContent: 'center',
          flexDirection: 'column',
          gap: '24px'
        }}>
          <h1 style={{ fontSize: '24px', opacity: 0.5, fontWeight: 300 }}>CrossNotes</h1>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '12px', alignItems: 'center' }}>
             {!vaultPath && (
               <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
                 <span className="keyboard-hint">Ctrl + O</span>
                 <span style={{ fontSize: '12px', color: 'var(--text-secondary)' }}>to open a vault</span>
               </div>
             )}
            <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
              <span className="keyboard-hint">Ctrl + K</span>
              <span style={{ fontSize: '12px', color: 'var(--text-secondary)' }}>for commands</span>
            </div>
          </div>
        </div>
      )}
    </AppShell>
  );
}

export default App;

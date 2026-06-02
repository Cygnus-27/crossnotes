import React from "react";
import { invoke } from "@tauri-apps/api/core";
import { useNoteStore } from "../../store/noteStore";
import { useUIStore } from "../../store/uiStore";
import { useVaultStore } from "../../store/vaultStore";

const shortVaultPath = (vaultPath: string | null) => {
  if (!vaultPath) return "No vault selected";
  const parts = vaultPath.split(/[\\/]/).filter(Boolean);
  return parts.slice(-2).join(" / ") || vaultPath;
};

export const Header: React.FC = () => {
  const { activeNote, isDirty } = useNoteStore();
  const { setHelpOpen, toggleTheme, theme } = useUIStore();
  const vaultPath = useVaultStore((state) => state.vaultPath);

  const openVaultFolder = async () => {
    if (!vaultPath) return;

    try {
      await invoke("open_in_file_manager", { path: vaultPath });
    } catch (err) {
      console.error("Failed to open vault folder:", err);
    }
  };

  return (
    <header className="app-header">
      <div className="header-main">
        <div className="brand-block">
          <div className="brand-title">
            {activeNote ? activeNote.name : "CrossNotes"}
            {isDirty && <span className="dirty-dot" title="Unsaved changes" />}
          </div>
          {activeNote ? (
            <button
              type="button"
              className="brand-path-button desktop-only"
              onClick={openVaultFolder}
              disabled={!vaultPath}
              title={vaultPath ? `Open ${vaultPath}` : "No vault selected"}
            >
              {shortVaultPath(vaultPath)}
            </button>
          ) : (
            <div className="brand-subtitle">Local markdown workspace</div>
          )}
        </div>
      </div>

      <div className="header-toolbar">
        <button
          type="button"
          className="header-action-button header-icon-button"
          onClick={toggleTheme}
          title={`Switch to ${theme === "dark" ? "light" : "dark"} mode`}
          aria-label={`Switch to ${theme === "dark" ? "light" : "dark"} mode`}
        >
          {theme === "dark" ? "☼" : "☾"}
        </button>
        <button
          type="button"
          className="header-action-button"
          onClick={() => setHelpOpen(true)}
          title="Open keyboard shortcuts"
        >
          ? <span className="keyboard-hint">Ctrl + ?</span>
        </button>
      </div>
    </header>
  );
};

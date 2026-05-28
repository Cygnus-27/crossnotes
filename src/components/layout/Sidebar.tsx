import React from "react";
import { useUIStore } from "../../store/uiStore";
import { useVault } from "../../hooks/useVault";
import { NoteTree } from "../notes/NoteTree";
import { useNoteStore } from "../../store/noteStore";

export const Sidebar: React.FC = () => {
  const collapsed = useUIStore((state) => state.sidebarCollapsed);
  const { vaultPath, openVault } = useVault();
  const notes = useNoteStore((state) => state.notes);
  const hasVault = Boolean(vaultPath);
  const hasNotes = notes.length > 0;

  return (
    <aside className={`sidebar ${collapsed ? "collapsed" : ""}`}>
      <div className="sidebar-inner">
        <div className="sidebar-header-row">
          <div>
            <h2 className="section-label">{hasVault ? "Vault" : "Welcome"}</h2>
          </div>
          {hasVault && (
            <button
              type="button"
              className="secondary-button"
              onClick={openVault}
            >
              Change
            </button>
          )}
        </div>

        <div style={{ flex: 1, overflowY: "auto" }}>
          {hasVault && hasNotes ? (
            <NoteTree />
          ) : (
            <div
              className="sidebar-card"
              style={{ display: "flex", flexDirection: "column", gap: "14px" }}
            >
              <p className="sidebar-copy">
                {hasVault
                  ? "This folder is connected, but there are no markdown notes in it yet."
                  : "Choose a folder of markdown notes to turn CrossNotes into your focused local workspace."}
              </p>
              <button
                type="button"
                className="primary-button"
                onClick={openVault}
              >
                {hasVault ? "Choose another vault" : "Open Vault"}
              </button>
            </div>
          )}
        </div>

        <div className="sidebar-footer">
          <span className="keyboard-hint">Ctrl + \</span>
          <span>Hide sidebar</span>
        </div>
      </div>
    </aside>
  );
};

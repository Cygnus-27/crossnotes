import React from "react";
import { useUIStore } from "../../store/uiStore";
import { useVault } from "../../hooks/useVault";
import { NoteTree } from "../notes/NoteTree";
import { FavoritesSection } from "../notes/FavoritesSection";
import { useNoteStore } from "../../store/noteStore";
import { SyncPanel } from "../sync/SyncPanel";

export const Sidebar: React.FC = () => {
  const collapsed = useUIStore((state) => state.sidebarCollapsed);
  const { vaultPath, openVault, createVault, useDefaultVault } = useVault();
  const notes = useNoteStore((state) => state.notes);
  const hasVault = Boolean(vaultPath);
  const hasNotes = notes.length > 0;

  return (
    <aside className={`sidebar ${collapsed ? "collapsed" : ""}`}>
      <div className="sidebar-inner">
        <div className="sidebar-section">
          <h2 className="section-label">{hasVault ? "Vault" : "Welcome"}</h2>
          <div className="button-row">
            {hasVault && (
              <button
                type="button"
                className="secondary-button desktop-only"
                onClick={openVault}
              >
                Change
              </button>
            )}
            <button
              type="button"
              className="secondary-button"
              onClick={useDefaultVault}
            >
              App Vault
            </button>
            <button
              type="button"
              className="secondary-button desktop-only"
              onClick={createVault}
            >
              New
            </button>
          </div>
        </div>

        <div style={{ flex: 1, minHeight: 0, overflowY: "auto" }}>
          <FavoritesSection />
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
              <div className="button-stack">
                <button
                  type="button"
                  className="primary-button desktop-only"
                  onClick={openVault}
                >
                  {hasVault ? "Choose another vault" : "Open Vault"}
                </button>
                <button
                  type="button"
                  className="secondary-button"
                  onClick={useDefaultVault}
                >
                  Use App Vault
                </button>
                <button
                  type="button"
                  className="secondary-button desktop-only"
                  onClick={createVault}
                >
                  Create New Vault
                </button>
              </div>
            </div>
          )}
        </div>

        {hasVault && <SyncPanel />}

        <div className="sidebar-footer">
          <span className="keyboard-hint">Ctrl + \</span>
          <span>Hide sidebar</span>
        </div>
      </div>
    </aside>
  );
};

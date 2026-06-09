import React, { useState } from "react";
import { useSync } from "../../hooks/useSync";
import { useVault } from "../../hooks/useVault";
import { useVaultStore } from "../../store/vaultStore";
import { useUIStore } from "../../store/uiStore";
import { CrossOsVaultSection } from "./CrossOsVaultSection";

export const SyncPanel: React.FC = () => {
  const vaultPath = useVaultStore((state) => state.vaultPath);
  const setSyncStageOpen = useUIStore((state) => state.setSyncStageOpen);
  const { loadNotes } = useVault();
  const {
    selectedCount,
    isSyncing,
    isImporting,
    lastExportPath,
    syncStatus,
    deviceIdentity,
    error,
    lastResult,
    lastImportResult,
    lastLanSendResult,
    triggerSync,
    importSyncPackage,
    startSync,
    sendLatestSyncPackage,
  } = useSync();
  const [expanded, setExpanded] = useState(false);

  if (!vaultPath) return null;

  const openStage = async () => {
    await startSync();
    setSyncStageOpen(true);
  };

  const sendToPeer = async () => {
    const peerHost = window.prompt("Peer device IP address (same Wi-Fi)");
    if (!peerHost?.trim()) return;
    await sendLatestSyncPackage(peerHost.trim());
  };

  const handleImport = async () => {
    const result = await importSyncPackage();
    if (result) await loadNotes(vaultPath);
  };

  return (
    <section className={`sync-panel ${expanded ? "is-open" : ""}`}>
      <button
        type="button"
        className="sync-panel-toggle"
        onClick={() => setExpanded((value) => !value)}
        aria-expanded={expanded}
      >
        <span className="section-label">LAN Sync</span>
        <span className="sync-panel-meta">
          {selectedCount > 0 && (
            <span className="sync-count-badge">{selectedCount}</span>
          )}
          {syncStatus && (
            <span
              className="sync-live-dot"
              title={`Online · port ${syncStatus.port}`}
            />
          )}
          <span className="sync-panel-chevron">{expanded ? "▾" : "▸"}</span>
        </span>
      </button>

      {expanded && (
        <div className="sync-panel-body">
          <p className="sidebar-copy">
            Mark notes with ○ in the list, create a package, then send it to a
            device receiving on the same network.
          </p>

          <div className="sync-action-group">
            <span className="sync-group-label">This device</span>
            <div className="button-row">
              <button
                type="button"
                className="primary-button"
                onClick={triggerSync}
                disabled={isSyncing || selectedCount === 0}
              >
                {isSyncing ? "Packaging…" : `Create package (${selectedCount})`}
              </button>
            </div>
          </div>

          <div className="sync-action-group">
            <span className="sync-group-label">Devices</span>
            <div className="button-row">
              <button
                type="button"
                className="primary-button"
                onClick={openStage}
              >
                {syncStatus ? "Open radar" : "Find devices"}
              </button>
            </div>
          </div>

          <div className="sync-action-group">
            <span className="sync-group-label">Manual transfer</span>
            <div className="button-row">
              <button
                type="button"
                className="secondary-button"
                onClick={sendToPeer}
                disabled={!lastExportPath}
              >
                Send to IP…
              </button>
              <button
                type="button"
                className="secondary-button"
                onClick={handleImport}
                disabled={isImporting}
              >
                {isImporting ? "Importing…" : "Import folder…"}
              </button>
            </div>
          </div>

          <CrossOsVaultSection />

          <div className="sync-panel-status">
            {deviceIdentity && (
              <p className="sync-panel-footnote">
                This device: {deviceIdentity.deviceName}
                {syncStatus ? " · online" : ""}
              </p>
            )}
            {lastResult && (
              <p className="sync-panel-footnote">
                Packaged {lastResult.exportedCount} note(s).
              </p>
            )}
            {lastLanSendResult && (
              <p className="sync-panel-footnote">
                Sent {lastLanSendResult.sentCount} file(s) to{" "}
                {lastLanSendResult.peerAddr}.
              </p>
            )}
            {lastImportResult && (
              <p className="sync-panel-footnote">
                Imported {lastImportResult.importedCount}·{" "}
                {lastImportResult.conflictCount} conflict(s).
              </p>
            )}
            {error && <p className="sidebar-error">{error}</p>}
          </div>
        </div>
      )}
    </section>
  );
};

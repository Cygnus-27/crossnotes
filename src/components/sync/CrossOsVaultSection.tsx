import { open } from "@tauri-apps/plugin-dialog";
import React, { useEffect, useState } from "react";
import { useSync } from "../../hooks/useSync";
import { useVault } from "../../hooks/useVault";
import { useVaultStore } from "../../store/vaultStore";
import { BridgeCandidate } from "../../store/syncStore";

/**
 * Cross-OS vault: snapshot-courier sync through a folder both operating
 * systems can read (e.g. the Windows NTFS partition from a dual boot).
 * Desktop-only — mounted inside the LAN sync panel.
 */
export const CrossOsVaultSection: React.FC = () => {
  const vaultPath = useVaultStore((state) => state.vaultPath);
  const { loadNotes } = useVault();
  const {
    bridge,
    bridgeStatus,
    detectBridges,
    selectBridge,
    clearBridge,
    setBridgeWholeVault,
    pushToBridge,
    pullFromBridge,
  } = useSync();

  const [candidates, setCandidates] = useState<BridgeCandidate[]>([]);
  const [busy, setBusy] = useState<"push" | "pull" | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    if (!bridge) detectBridges().then(setCandidates);
  }, [bridge, detectBridges]);

  const browse = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Choose the shared (cross-OS) folder",
    });
    const dir = Array.isArray(selected) ? selected[0] : selected;
    if (typeof dir === "string") await selectBridge(dir.replace(/\\/g, "/"));
  };

  const doPush = async () => {
    setBusy("push");
    setMessage(null);
    const result = await pushToBridge();
    setBusy(null);
    if (result) setMessage(`Pushed ${result.exportedCount} note(s) to the bridge.`);
  };

  const doPull = async () => {
    setBusy("pull");
    setMessage(null);
    const result = await pullFromBridge();
    setBusy(null);
    if (!result) return;
    if (vaultPath) await loadNotes(vaultPath);
    setMessage(
      result.importedCount > 0
        ? `Pulled ${result.importedCount} note(s)${
            result.conflictCount > 0
              ? `, ${result.conflictCount} conflict(s)`
              : ""
          }.`
        : "Already up to date.",
    );
  };

  const suggestions = candidates.filter((candidate) => candidate.hasExistingBridge);

  return (
    <div className="sync-action-group desktop-only">
      <span className="sync-group-label">Cross-OS vault</span>

      {bridge ? (
        <>
          <p className="sync-panel-footnote">{bridge.label} — {bridge.path}</p>
          {bridgeStatus && (
            <p
              className={
                bridgeStatus.writable
                  ? "sync-panel-footnote"
                  : "sidebar-error"
              }
            >
              {bridgeStatus.message}
            </p>
          )}

          <label className="sync-toggle-row">
            <input
              type="checkbox"
              checked={bridge.wholeVault}
              onChange={(event) => setBridgeWholeVault(event.target.checked)}
            />
            <span>Sync the whole vault (off = selected notes only)</span>
          </label>

          <div className="sidebar-card-actions">
            <button
              type="button"
              className="primary-button"
              onClick={doPush}
              disabled={busy !== null || !bridgeStatus?.writable}
            >
              {busy === "push" ? "Pushing…" : "Push"}
            </button>
            <button
              type="button"
              className="secondary-button"
              onClick={doPull}
              disabled={busy !== null}
            >
              {busy === "pull" ? "Pulling…" : "Pull"}
            </button>
            <button type="button" className="secondary-button" onClick={browse}>
              Change…
            </button>
            <button type="button" className="secondary-button" onClick={clearBridge}>
              Remove
            </button>
          </div>
          {message && <p className="sync-panel-footnote">{message}</p>}
        </>
      ) : (
        <>
          <p className="sidebar-copy">
            Sync between two OSes on one machine (e.g. Windows + Linux dual
            boot) through a folder both can read.
          </p>
          {suggestions.length > 0 && (
            <div className="sync-suggestion">
              <span className="sync-panel-footnote">
                Found an existing cross-OS vault:
              </span>
              {suggestions.map((candidate) => (
                <button
                  key={candidate.path}
                  type="button"
                  className="secondary-button"
                  onClick={() => selectBridge(candidate.path)}
                >
                  Use {candidate.label}
                </button>
              ))}
            </div>
          )}
          <div className="sidebar-card-actions">
            <button type="button" className="secondary-button" onClick={browse}>
              Choose shared folder…
            </button>
          </div>
        </>
      )}
    </div>
  );
};

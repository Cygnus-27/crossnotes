import React, { useEffect, useState } from "react";
import { useUIStore } from "../../store/uiStore";
import { useSync } from "../../hooks/useSync";
import { SyncPeer } from "../../store/syncStore";

/** Position a peer orb around the radar centre (upper arc, away from self node). */
const peerPosition = (index: number, total: number) => {
  const spread = Math.min(total, 6);
  const slot = index % spread;
  const angle = (-150 + (slot / Math.max(spread - 1, 1)) * 120) * (Math.PI / 180);
  const radius = 34 + (Math.floor(index / spread) % 2) * 12;
  return {
    left: `${50 + Math.cos(angle) * radius}%`,
    top: `${50 + Math.sin(angle) * radius}%`,
  };
};

export const SyncStage: React.FC = () => {
  const open = useUIStore((state) => state.syncStageOpen);
  const setOpen = useUIStore((state) => state.setSyncStageOpen);
  const {
    peers,
    deviceIdentity,
    syncStatus,
    selectedCount,
    lastExportPath,
    triggerSync,
    sendLatestSyncPackage,
    startSync,
    beginPairing,
    cancelPairing,
    pairWithCode,
  } = useSync();

  const [pairing, setPairing] = useState(false);
  const [pairingCode, setPairingCode] = useState<string | null>(null);
  const [codeInput, setCodeInput] = useState("");
  const [pairBusy, setPairBusy] = useState(false);
  const [pairMessage, setPairMessage] = useState<string | null>(null);

  // Make sure the receiver + discovery are running whenever the stage is open.
  useEffect(() => {
    if (open) startSync();
  }, [open, startSync]);

  // Ask the backend for a pairing code when entering pair mode; release it on exit.
  useEffect(() => {
    if (pairing) {
      let active = true;
      beginPairing().then((code) => {
        if (active) setPairingCode(code);
      });
      return () => {
        active = false;
        cancelPairing();
      };
    }
    setPairingCode(null);
    return undefined;
  }, [pairing, beginPairing, cancelPairing]);

  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        if (pairing) setPairing(false);
        else setOpen(false);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, pairing, setOpen]);

  if (!open) return null;

  const sendToPeer = async (peer: SyncPeer) => {
    if (!lastExportPath) {
      await triggerSync();
    }
    await sendLatestSyncPackage(peer.host);
  };

  const submitCode = async () => {
    setPairBusy(true);
    setPairMessage(null);
    const result = await pairWithCode(codeInput);
    setPairBusy(false);
    if (result) {
      setPairMessage(`Paired with ${result.deviceName}.`);
      setCodeInput("");
      setPairing(false);
    } else {
      setPairMessage("Could not pair — check the code and try again.");
    }
  };

  return (
    <div
      className="modal-backdrop sync-stage-backdrop"
      onClick={() => setOpen(false)}
    >
      <div
        className="modal-panel sync-stage-panel"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="sync-stage-header">
          <div>
            <h2 className="sync-stage-title">Sync over your network</h2>
            <p className="sync-stage-subtitle">
              {selectedCount > 0
                ? `${selectedCount} note(s) ready · tap a device to send`
                : "Mark notes with ○ in the sidebar, then tap a device to send"}
            </p>
          </div>
          <button
            type="button"
            className="icon-button"
            onClick={() => setOpen(false)}
            aria-label="Close sync"
          >
            ×
          </button>
        </div>

        {pairing ? (
          <div className="sync-pair-view">
            <div className="sync-pair-code">
              {(pairingCode ?? "·····").split("").map((char, index) => (
                <span key={index} className="sync-pair-code-char">
                  {char}
                </span>
              ))}
            </div>
            <p className="sync-stage-subtitle">
              Enter this code on your other device — or type its code below to
              pair.
            </p>
            <div className="sync-pair-or">PAIR WITH CODE</div>
            <input
              className="sync-pair-input"
              value={codeInput}
              maxLength={5}
              placeholder="• • • • •"
              onChange={(event) =>
                setCodeInput(event.target.value.toUpperCase().slice(0, 5))
              }
            />
            {pairMessage && (
              <p className="sync-stage-subtitle">{pairMessage}</p>
            )}
            <div className="sync-pair-actions">
              <button
                type="button"
                className="secondary-button"
                onClick={() => setPairing(false)}
              >
                Back
              </button>
              <button
                type="button"
                className="primary-button"
                disabled={codeInput.length < 5 || pairBusy}
                onClick={submitCode}
              >
                {pairBusy ? "Pairing…" : "Pair"}
              </button>
            </div>
          </div>
        ) : (
          <div className="sync-radar">
            <span className="sync-radar-ring" />
            <span className="sync-radar-ring" />
            <span className="sync-radar-ring" />
            <span className="sync-radar-ring" />

            {peers.length === 0 ? (
              <div className="sync-radar-empty">
                <p className="sync-radar-empty-title">
                  Open CrossNotes on another device
                </p>
                <p className="sync-stage-subtitle">
                  Devices on the same network appear here automatically.
                </p>
              </div>
            ) : (
              peers.map((peer, index) => (
                <button
                  key={peer.deviceId}
                  type="button"
                  className={`sync-peer ${peer.paired ? "is-paired" : ""}`}
                  style={peerPosition(index, peers.length)}
                  onClick={() => sendToPeer(peer)}
                  title={
                    peer.paired
                      ? `Send to ${peer.deviceName}`
                      : `${peer.deviceName} — pair first to send`
                  }
                >
                  <span className="sync-peer-dot" />
                  <span className="sync-peer-name">{peer.deviceName}</span>
                </button>
              ))
            )}

            <div className="sync-self">
              <span className="sync-self-pulse" />
              <span className="sync-self-name">
                You are known as{" "}
                <strong>{deviceIdentity?.deviceName ?? "this device"}</strong>
              </span>
              <span className="sync-discoverable">
                {syncStatus
                  ? "Discoverable · on this network"
                  : "Starting discovery…"}
              </span>
            </div>
          </div>
        )}

        <div className="sync-stage-footer">
          <button
            type="button"
            className="secondary-button"
            onClick={() => setPairing((value) => !value)}
          >
            {pairing ? "Show devices" : "Pair a device"}
          </button>
        </div>
      </div>
    </div>
  );
};

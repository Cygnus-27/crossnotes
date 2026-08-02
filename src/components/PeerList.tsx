import { useState } from "react";

import * as api from "../lib/api";
import { useAppStore } from "../store/appStore";

export function PeerList() {
  const peers = useAppStore((state) => state.peers);
  const trusted = useAppStore((state) => state.trusted);
  const staged = useAppStore((state) => state.staged);
  const clearStaged = useAppStore((state) => state.clearStaged);
  const setError = useAppStore((state) => state.setError);

  const [manualAddress, setManualAddress] = useState("");
  const [showManual, setShowManual] = useState(false);

  const trustedIds = new Set(trusted.map((peer) => peer.deviceId));
  const canSend = staged.length > 0;

  async function send(action: () => Promise<string>) {
    if (!canSend) return;
    try {
      await action();
      clearStaged();
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <div className="panel">
      <div className="panel__head">
        <h2>Devices on this network</h2>
        <button
          type="button"
          className="link-button"
          onClick={() => setShowManual((value) => !value)}
        >
          {showManual ? "Hide" : "Enter address"}
        </button>
      </div>

      {showManual && (
        <form
          className="manual"
          onSubmit={(e) => {
            e.preventDefault();
            const address = manualAddress.trim();
            if (!address) return;
            void send(() => api.sendToAddress(address, staged)).then(() =>
              setManualAddress(""),
            );
          }}
        >
          <input
            value={manualAddress}
            placeholder="192.168.1.42"
            onChange={(e) => setManualAddress(e.target.value)}
          />
          <button type="submit" disabled={!canSend || !manualAddress.trim()}>
            Send
          </button>
          <p className="muted small">
            Use this when discovery is blocked. Some networks isolate clients from each
            other, which stops devices finding one another automatically.
          </p>
        </form>
      )}

      {peers.length === 0 ? (
        <p className="muted empty">
          Looking for devices… Open fluqsr on another machine on the same network.
        </p>
      ) : (
        <ul className="peers">
          {peers.map((peer) => (
            <li key={peer.deviceId}>
              <button
                type="button"
                className="peer"
                disabled={!canSend}
                title={canSend ? `Send ${staged.length} item(s)` : "Add files first"}
                onClick={() => void send(() => api.sendToPeer(peer.deviceId, staged))}
              >
                <span className="peer__name">{peer.deviceName}</span>
                <span className="peer__meta">
                  {api.platformLabel[peer.platform] ?? peer.platform} · {peer.address}
                  {trustedIds.has(peer.deviceId) && (
                    <span className="badge" title="Already paired with this device">
                      paired
                    </span>
                  )}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

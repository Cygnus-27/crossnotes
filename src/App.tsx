import { useEffect } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";

import * as api from "./lib/api";
import { useAppStore } from "./store/appStore";
import { Header } from "./components/Header";
import { DropZone } from "./components/DropZone";
import { PeerList } from "./components/PeerList";
import { TransferList } from "./components/TransferList";
import { OfferDialog } from "./components/OfferDialog";
import { PairingDialog } from "./components/PairingDialog";
import { IdentityAlert } from "./components/IdentityAlert";

export default function App() {
  const store = useAppStore();
  const { offers, pairings, identityWarnings, error } = store;

  useEffect(() => {
    let cancelled = false;
    const unlisteners: Array<() => void> = [];

    async function boot() {
      try {
        const [self, peers, trusted, transfers] = await Promise.all([
          api.getSelf(),
          api.listPeers(),
          api.listTrusted(),
          api.listTransfers(),
        ]);
        if (cancelled) return;

        store.setSelf(self);
        store.setPeers(peers);
        store.setTransfers(transfers);
        useAppStore.setState({ trusted });
      } catch (err) {
        if (!cancelled) store.setError(String(err));
      }

      // Subscribe after the initial snapshot so nothing is missed in between.
      unlisteners.push(
        await api.onPeersUpdated((peers) => useAppStore.getState().setPeers(peers)),
        await api.onProgress((progress) =>
          useAppStore.getState().upsertTransfer(progress),
        ),
        await api.onOffer((offer) => useAppStore.getState().pushOffer(offer)),
        await api.onPairing((request) => useAppStore.getState().pushPairing(request)),
        await api.onIdentityWarning((warning) =>
          useAppStore.getState().pushIdentityWarning(warning),
        ),
      );

      // Tauri's own drag-drop event carries real filesystem paths; the HTML5
      // one does not, which is why this does not use onDrop.
      const webview = await getCurrentWebview();
      unlisteners.push(
        await webview.onDragDropEvent((event) => {
          if (event.payload.type === "drop") {
            useAppStore.getState().stage(event.payload.paths);
          }
        }),
      );
    }

    void boot();

    return () => {
      cancelled = true;
      unlisteners.forEach((off) => off());
    };
    // Runs once: the store is read through getState inside the callbacks.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="app">
      <Header />

      {identityWarnings.map((warning) => (
        <IdentityAlert key={warning.peerDeviceId} warning={warning} />
      ))}

      {error && (
        <div className="banner banner--error" role="alert">
          <span>{error}</span>
          <button type="button" onClick={() => store.setError(null)}>
            Dismiss
          </button>
        </div>
      )}

      <main className="layout">
        <section className="column">
          <DropZone />
          <PeerList />
        </section>
        <section className="column">
          <TransferList />
        </section>
      </main>

      {/* One modal at a time, oldest first, so a queue of prompts is answered
          in the order the requests arrived. */}
      {pairings.length > 0 ? (
        <PairingDialog request={pairings[0]} />
      ) : offers.length > 0 ? (
        <OfferDialog offer={offers[0]} />
      ) : null}
    </div>
  );
}

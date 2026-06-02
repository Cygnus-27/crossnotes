import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import { useVault } from "./useVault";
import { useVaultStore } from "../store/vaultStore";
import {
  DeviceIdentity,
  SyncImportResult,
  SyncPeer,
  useSyncStore,
} from "../store/syncStore";

/**
 * Subscribes to backend sync events (discovery, incoming transfers, pairing)
 * and reflects them into the store / note list. Mount once at the app root.
 */
export const useSyncListeners = () => {
  const { loadNotes } = useVault();

  useEffect(() => {
    const subscriptions = [
      listen<SyncPeer[]>("sync://peers", (event) => {
        useSyncStore.getState().setPeers(event.payload);
      }),
      listen<SyncImportResult>("sync://received", (event) => {
        useSyncStore.getState().setLastImportResult(event.payload);
        const vaultPath = useVaultStore.getState().vaultPath;
        if (vaultPath) loadNotes(vaultPath);
      }),
      listen<DeviceIdentity>("sync://paired", () => {
        useSyncStore.getState().setError(null);
      }),
      listen<string>("sync://error", (event) => {
        useSyncStore.getState().setError(event.payload);
      }),
    ];

    return () => {
      subscriptions.forEach((subscription) =>
        subscription.then((unlisten) => unlisten()),
      );
    };
  }, [loadNotes]);
};

import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect } from "react";
import {
  DeviceIdentity,
  LanSendResult,
  notePathToVaultRelativePath,
  SyncImportResult,
  SyncManifest,
  SyncStartResult,
  SyncTriggerResult,
  useSyncStore,
} from "../store/syncStore";
import { useVaultStore } from "../store/vaultStore";

const errorMessage = (err: unknown) =>
  err instanceof Error ? err.message : String(err);

export const useSync = () => {
  const vaultPath = useVaultStore((state) => state.vaultPath);
  const selectedNotes = useSyncStore((state) => state.selectedNotes);
  const isLoading = useSyncStore((state) => state.isLoading);
  const isSyncing = useSyncStore((state) => state.isSyncing);
  const isImporting = useSyncStore((state) => state.isImporting);
  const error = useSyncStore((state) => state.error);
  const deviceIdentity = useSyncStore((state) => state.deviceIdentity);
  const lastResult = useSyncStore((state) => state.lastResult);
  const lastImportResult = useSyncStore((state) => state.lastImportResult);
  const syncStatus = useSyncStore((state) => state.syncStatus);
  const lastLanSendResult = useSyncStore((state) => state.lastLanSendResult);
  const peers = useSyncStore((state) => state.peers);
  const lastTriggeredAt = useSyncStore((state) => state.lastTriggeredAt);
  const lastExportPath = useSyncStore((state) => state.lastExportPath);

  const loadSyncManifest = useCallback(async () => {
    if (!vaultPath) {
      useSyncStore.getState().resetSync();
      return;
    }

    useSyncStore.getState().setLoading(true);
    try {
      const [manifest, identity] = await Promise.all([
        invoke<SyncManifest>("get_sync_manifest", { vaultPath }),
        invoke<DeviceIdentity>("get_device_identity", { vaultPath }),
      ]);
      useSyncStore.getState().setManifest(manifest);
      useSyncStore.getState().setDeviceIdentity(identity);
      // Keep the receiver pointed at the current vault even if it changes.
      invoke("set_active_vault", { vaultPath }).catch(() => undefined);
    } catch (err) {
      useSyncStore.getState().setError(errorMessage(err));
    } finally {
      useSyncStore.getState().setLoading(false);
    }
  }, [vaultPath]);

  const isNoteSelected = useCallback(
    (notePath: string) => {
      if (!vaultPath) return false;
      const relativePath = notePathToVaultRelativePath(vaultPath, notePath);
      return selectedNotes.includes(relativePath);
    },
    [selectedNotes, vaultPath],
  );

  const setNoteSyncEnabled = useCallback(
    async (notePath: string, enabled: boolean) => {
      if (!vaultPath) return;

      try {
        const manifest = await invoke<SyncManifest>("set_note_sync_enabled", {
          vaultPath,
          notePath,
          enabled,
        });
        useSyncStore.getState().setManifest(manifest);
      } catch (err) {
        useSyncStore.getState().setError(errorMessage(err));
      }
    },
    [vaultPath],
  );

  const importSyncPackage = useCallback(async () => {
    if (!vaultPath) return null;

    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select CrossNotes sync package",
    });

    let packagePath: string | null = null;
    if (Array.isArray(selected)) {
      packagePath = selected[0] ?? null;
    } else if (typeof selected === "string") {
      packagePath = selected;
    }

    if (!packagePath) return null;

    useSyncStore.getState().setImporting(true);
    useSyncStore.getState().setError(null);
    try {
      const result = await invoke<SyncImportResult>("import_sync_package", {
        vaultPath,
        packagePath: packagePath.replace(/\\/g, "/"),
      });
      useSyncStore.getState().setLastImportResult(result);
      await loadSyncManifest();
      return result;
    } catch (err) {
      useSyncStore.getState().setError(errorMessage(err));
      return null;
    } finally {
      useSyncStore.getState().setImporting(false);
    }
  }, [loadSyncManifest, vaultPath]);

  const startSync = useCallback(async () => {
    if (!vaultPath) return null;

    try {
      const status = await invoke<SyncStartResult>("start_sync", {
        vaultPath,
        port: 37642,
      });
      useSyncStore.getState().setSyncStatus(status);
      return status;
    } catch (err) {
      useSyncStore.getState().setError(errorMessage(err));
      return null;
    }
  }, [vaultPath]);

  const beginPairing = useCallback(async () => {
    try {
      return await invoke<string>("begin_pairing");
    } catch (err) {
      useSyncStore.getState().setError(errorMessage(err));
      return null;
    }
  }, []);

  const cancelPairing = useCallback(async () => {
    try {
      await invoke("cancel_pairing");
    } catch {
      /* best effort */
    }
  }, []);

  const pairWithCode = useCallback(async (code: string) => {
    useSyncStore.getState().setError(null);
    try {
      return await invoke<DeviceIdentity>("pair_with_code", { code });
    } catch (err) {
      useSyncStore.getState().setError(errorMessage(err));
      return null;
    }
  }, []);

  const sendLatestSyncPackage = useCallback(
    async (peerHost: string) => {
      if (!lastExportPath) {
        useSyncStore
          .getState()
          .setError("Create a sync package before sending over LAN.");
        return null;
      }

      try {
        const result = await invoke<LanSendResult>("send_lan_sync_package", {
          peerHost,
          port: 37642,
          packagePath: lastExportPath,
        });
        useSyncStore.getState().setLastLanSendResult(result);
        return result;
      } catch (err) {
        useSyncStore.getState().setError(errorMessage(err));
        return null;
      }
    },
    [lastExportPath],
  );

  const triggerSync = useCallback(async () => {
    if (!vaultPath) return null;

    useSyncStore.getState().setSyncing(true);
    useSyncStore.getState().setError(null);
    try {
      const result = await invoke<SyncTriggerResult>("trigger_sync", {
        vaultPath,
      });
      useSyncStore.getState().setLastResult(result);
      await loadSyncManifest();
      return result;
    } catch (err) {
      useSyncStore.getState().setError(errorMessage(err));
      return null;
    } finally {
      useSyncStore.getState().setSyncing(false);
    }
  }, [loadSyncManifest, vaultPath]);

  useEffect(() => {
    loadSyncManifest();
  }, [loadSyncManifest]);

  return {
    selectedNotes,
    selectedCount: selectedNotes.length,
    isLoading,
    isSyncing,
    isImporting,
    error,
    deviceIdentity,
    lastResult,
    lastImportResult,
    syncStatus,
    lastLanSendResult,
    peers,
    lastTriggeredAt,
    lastExportPath,
    loadSyncManifest,
    isNoteSelected,
    setNoteSyncEnabled,
    triggerSync,
    importSyncPackage,
    startSync,
    beginPairing,
    cancelPairing,
    pairWithCode,
    sendLatestSyncPackage,
  };
};

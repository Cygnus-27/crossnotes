import { create } from "zustand";

export interface SyncManifest {
  selectedNotes: string[];
  lastTriggeredAt: number | null;
  lastExportPath: string | null;
}

export interface SyncTriggerResult {
  selectedCount: number;
  exportedCount: number;
  exportPath: string;
  manifestPath: string;
}

export interface DeviceIdentity {
  deviceId: string;
  deviceName: string;
  createdAt: number;
}

export interface SyncImportResult {
  sourceDeviceId: string;
  importedCount: number;
  skippedCount: number;
  conflictCount: number;
  conflicts: string[];
}

export interface SyncStartResult {
  port: number;
  deviceId: string;
  deviceName: string;
}

export interface LanSendResult {
  sentCount: number;
  peerAddr: string;
}

export interface SyncPeer {
  deviceId: string;
  deviceName: string;
  host: string;
  port: number;
  paired: boolean;
}

export interface BridgeConfig {
  path: string;
  label: string;
  createdAt: number;
  wholeVault: boolean;
  lastImportedAt: Record<string, number>;
}

export interface BridgeCandidate {
  path: string;
  label: string;
  hasExistingBridge: boolean;
}

export interface BridgeStatus {
  configured: boolean;
  path: string | null;
  writable: boolean;
  peerCount: number;
  message: string;
}

export interface BridgePushResult {
  deviceId: string;
  exportedCount: number;
  path: string;
}

export interface BridgePullResult {
  importedCount: number;
  skippedCount: number;
  conflictCount: number;
  conflicts: string[];
  fromDevices: string[];
}

interface SyncState extends SyncManifest {
  isLoading: boolean;
  isSyncing: boolean;
  isImporting: boolean;
  error: string | null;
  deviceIdentity: DeviceIdentity | null;
  lastResult: SyncTriggerResult | null;
  lastImportResult: SyncImportResult | null;
  syncStatus: SyncStartResult | null;
  lastLanSendResult: LanSendResult | null;
  peers: SyncPeer[];
  bridge: BridgeConfig | null;
  bridgeStatus: BridgeStatus | null;
  setPeers: (peers: SyncPeer[]) => void;
  setBridge: (bridge: BridgeConfig | null) => void;
  setBridgeStatus: (bridgeStatus: BridgeStatus | null) => void;
  setManifest: (manifest: SyncManifest) => void;
  setLoading: (isLoading: boolean) => void;
  setSyncing: (isSyncing: boolean) => void;
  setImporting: (isImporting: boolean) => void;
  setError: (error: string | null) => void;
  setDeviceIdentity: (deviceIdentity: DeviceIdentity | null) => void;
  setLastResult: (lastResult: SyncTriggerResult | null) => void;
  setLastImportResult: (lastImportResult: SyncImportResult | null) => void;
  setSyncStatus: (syncStatus: SyncStartResult | null) => void;
  setLastLanSendResult: (lastLanSendResult: LanSendResult | null) => void;
  resetSync: () => void;
}

const emptyManifest: SyncManifest = {
  selectedNotes: [],
  lastTriggeredAt: null,
  lastExportPath: null,
};

export const notePathToVaultRelativePath = (
  vaultPath: string,
  notePath: string,
) => {
  const normalizedVault = vaultPath.replace(/\\/g, "/").replace(/\/+$/, "");
  const normalizedNote = notePath.replace(/\\/g, "/");
  const prefix = `${normalizedVault}/`;

  return normalizedNote.startsWith(prefix)
    ? normalizedNote.slice(prefix.length)
    : normalizedNote;
};

export const useSyncStore = create<SyncState>((set) => ({
  ...emptyManifest,
  isLoading: false,
  isSyncing: false,
  isImporting: false,
  error: null,
  deviceIdentity: null,
  lastResult: null,
  lastImportResult: null,
  syncStatus: null,
  lastLanSendResult: null,
  peers: [],
  bridge: null,
  bridgeStatus: null,
  setPeers: (peers) => set({ peers }),
  setBridge: (bridge) => set({ bridge }),
  setBridgeStatus: (bridgeStatus) => set({ bridgeStatus }),
  setManifest: (manifest) => set({ ...manifest, error: null }),
  setLoading: (isLoading) => set({ isLoading }),
  setSyncing: (isSyncing) => set({ isSyncing }),
  setImporting: (isImporting) => set({ isImporting }),
  setError: (error) => set({ error }),
  setDeviceIdentity: (deviceIdentity) => set({ deviceIdentity }),
  setLastResult: (lastResult) => set({ lastResult }),
  setLastImportResult: (lastImportResult) => set({ lastImportResult }),
  setSyncStatus: (syncStatus) => set({ syncStatus }),
  setLastLanSendResult: (lastLanSendResult) => set({ lastLanSendResult }),
  resetSync: () =>
    set({
      ...emptyManifest,
      isLoading: false,
      isSyncing: false,
      isImporting: false,
      error: null,
      deviceIdentity: null,
      lastResult: null,
      lastImportResult: null,
      syncStatus: null,
      lastLanSendResult: null,
      peers: [],
    }),
}));

// Typed wrappers over the Rust command surface, plus the event channels the
// backend pushes on. Everything the UI knows about the backend goes through
// this file.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Platform = "windows" | "macos" | "linux" | "android" | "ios" | "unknown";

export interface DeviceInfo {
  deviceId: string;
  deviceName: string;
  platform: Platform;
  fingerprint: string;
}

export interface Settings {
  receiveDir: string;
  transferPort: number;
  deviceName: string;
}

export interface SelfView {
  device: DeviceInfo;
  shortFingerprint: string;
  settings: Settings;
}

export interface Peer {
  deviceId: string;
  deviceName: string;
  platform: Platform;
  address: string;
  port: number;
  source: "beacon" | "mdns" | "manual";
  lastSeenSecs: number;
  fingerprintHint: string | null;
}

export interface TrustedPeer {
  deviceId: string;
  deviceName: string;
  fingerprint: string;
  pairedAt: number;
  autoAccept: boolean;
}

export type Direction = "send" | "receive";

export type TransferStatus =
  | "connecting"
  | "awaitingApproval"
  | "active"
  | "completed"
  | "declined"
  | "cancelled"
  | "failed";

export interface TransferProgress {
  transferId: string;
  direction: Direction;
  peerDeviceId: string;
  peerName: string;
  status: TransferStatus;
  totalBytes: number;
  transferredBytes: number;
  fileCount: number;
  filesCompleted: number;
  currentFile: string | null;
  bytesPerSecond: number;
  error: string | null;
}

export interface IncomingOffer {
  transferId: string;
  peerDeviceId: string;
  peerName: string;
  peerFingerprint: string;
  fileCount: number;
  totalBytes: number;
  preview: string[];
}

export interface PairingRequest {
  requestId: string;
  peerDeviceId: string;
  peerName: string;
  peerFingerprint: string;
  pairingCode: string;
  direction: Direction;
}

export interface IdentityWarning {
  peerDeviceId: string;
  peerName: string;
  expectedFingerprint: string;
  presentedFingerprint: string;
}

// --- Commands --------------------------------------------------------------

export const getSelf = () => invoke<SelfView>("get_self");
export const listPeers = () => invoke<Peer[]>("list_peers");
export const listTrusted = () => invoke<TrustedPeer[]>("list_trusted");
export const listTransfers = () => invoke<TransferProgress[]>("list_transfers");
export const clearFinishedTransfers = () => invoke<void>("clear_finished_transfers");

export const sendToPeer = (deviceId: string, paths: string[]) =>
  invoke<string>("send_to_peer", { deviceId, paths });

export const sendToAddress = (address: string, paths: string[]) =>
  invoke<string>("send_to_address", { address, paths });

export const respondToOffer = (transferId: string, accept: boolean) =>
  invoke<void>("respond_to_offer", { transferId, accept });

export const respondToPairing = (requestId: string, confirmed: boolean) =>
  invoke<void>("respond_to_pairing", { requestId, confirmed });

export const cancelTransfer = (transferId: string) =>
  invoke<void>("cancel_transfer", { transferId });

export const forgetPeer = (deviceId: string) => invoke<void>("forget_peer", { deviceId });

export const setAutoAccept = (deviceId: string, enabled: boolean) =>
  invoke<void>("set_auto_accept", { deviceId, enabled });

export const setReceiveDir = (path: string) => invoke<Settings>("set_receive_dir", { path });

export const setDeviceName = (name: string) => invoke<Settings>("set_device_name", { name });

export const getReceiveDir = () => invoke<string>("get_receive_dir");

// --- Events ----------------------------------------------------------------

export const onPeersUpdated = (fn: (peers: Peer[]) => void): Promise<UnlistenFn> =>
  listen<Peer[]>("peers://updated", (event) => fn(event.payload));

export const onProgress = (fn: (progress: TransferProgress) => void): Promise<UnlistenFn> =>
  listen<TransferProgress>("transfer://progress", (event) => fn(event.payload));

export const onOffer = (fn: (offer: IncomingOffer) => void): Promise<UnlistenFn> =>
  listen<IncomingOffer>("transfer://offer", (event) => fn(event.payload));

export const onPairing = (fn: (request: PairingRequest) => void): Promise<UnlistenFn> =>
  listen<PairingRequest>("transfer://pairing", (event) => fn(event.payload));

export const onIdentityWarning = (
  fn: (warning: IdentityWarning) => void,
): Promise<UnlistenFn> =>
  listen<IdentityWarning>("security://identity-mismatch", (event) => fn(event.payload));

// --- Formatting ------------------------------------------------------------

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value < 10 ? value.toFixed(1) : Math.round(value)} ${units[unit]}`;
}

export function formatRate(bytesPerSecond: number): string {
  if (bytesPerSecond <= 0) return "";
  return `${formatBytes(bytesPerSecond)}/s`;
}

export function formatEta(progress: TransferProgress): string {
  if (progress.bytesPerSecond <= 0 || progress.status !== "active") return "";
  const remaining = progress.totalBytes - progress.transferredBytes;
  if (remaining <= 0) return "";

  const seconds = Math.round(remaining / progress.bytesPerSecond);
  if (seconds < 60) return `${seconds}s left`;
  if (seconds < 3600) return `${Math.round(seconds / 60)}m left`;
  return `${(seconds / 3600).toFixed(1)}h left`;
}

/// Split a fingerprint into readable groups. Used when the user needs to
/// compare it against another screen.
export function groupFingerprint(hex: string): string {
  return (hex.match(/.{1,4}/g) ?? []).join(" ");
}

export const platformLabel: Record<Platform, string> = {
  windows: "Windows",
  macos: "macOS",
  linux: "Linux",
  android: "Android",
  ios: "iOS",
  unknown: "Unknown",
};

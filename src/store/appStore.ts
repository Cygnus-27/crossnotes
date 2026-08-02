// Single store for everything the UI renders. The backend is the source of
// truth; this holds the latest snapshot it has pushed.

import { create } from "zustand";

import * as api from "../lib/api";

interface AppState {
  self: api.SelfView | null;
  peers: api.Peer[];
  trusted: api.TrustedPeer[];
  transfers: api.TransferProgress[];

  /// Queued because two devices can ask at once, and answering one should not
  /// discard the other.
  offers: api.IncomingOffer[];
  pairings: api.PairingRequest[];
  identityWarnings: api.IdentityWarning[];

  /// Files staged in the drop zone, waiting for a destination to be picked.
  staged: string[];
  error: string | null;

  setSelf: (value: api.SelfView) => void;
  setPeers: (peers: api.Peer[]) => void;
  refreshTrusted: () => Promise<void>;
  upsertTransfer: (progress: api.TransferProgress) => void;
  setTransfers: (transfers: api.TransferProgress[]) => void;

  pushOffer: (offer: api.IncomingOffer) => void;
  resolveOffer: (transferId: string, accept: boolean) => Promise<void>;
  pushPairing: (request: api.PairingRequest) => void;
  resolvePairing: (requestId: string, confirmed: boolean) => Promise<void>;
  pushIdentityWarning: (warning: api.IdentityWarning) => void;
  dismissIdentityWarning: (deviceId: string) => void;

  stage: (paths: string[]) => void;
  clearStaged: () => void;
  setError: (message: string | null) => void;
}

export const useAppStore = create<AppState>((set, get) => ({
  self: null,
  peers: [],
  trusted: [],
  transfers: [],
  offers: [],
  pairings: [],
  identityWarnings: [],
  staged: [],
  error: null,

  setSelf: (value) => set({ self: value }),
  setPeers: (peers) => set({ peers }),

  refreshTrusted: async () => {
    try {
      set({ trusted: await api.listTrusted() });
    } catch (err) {
      set({ error: String(err) });
    }
  },

  upsertTransfer: (progress) =>
    set((state) => {
      const index = state.transfers.findIndex((t) => t.transferId === progress.transferId);
      if (index === -1) {
        // Newest first: an active transfer should not be pushed off-screen by
        // older completed ones.
        return { transfers: [progress, ...state.transfers] };
      }
      const next = [...state.transfers];
      next[index] = progress;
      return { transfers: next };
    }),

  setTransfers: (transfers) => set({ transfers }),

  pushOffer: (offer) =>
    set((state) => ({
      offers: state.offers.some((o) => o.transferId === offer.transferId)
        ? state.offers
        : [...state.offers, offer],
    })),

  resolveOffer: async (transferId, accept) => {
    // Dismiss first so the dialog cannot be double-submitted while the command
    // is in flight.
    set((state) => ({ offers: state.offers.filter((o) => o.transferId !== transferId) }));
    try {
      await api.respondToOffer(transferId, accept);
    } catch (err) {
      set({ error: String(err) });
    }
  },

  pushPairing: (request) =>
    set((state) => ({
      pairings: state.pairings.some((p) => p.requestId === request.requestId)
        ? state.pairings
        : [...state.pairings, request],
    })),

  resolvePairing: async (requestId, confirmed) => {
    set((state) => ({ pairings: state.pairings.filter((p) => p.requestId !== requestId) }));
    try {
      await api.respondToPairing(requestId, confirmed);
      if (confirmed) await get().refreshTrusted();
    } catch (err) {
      set({ error: String(err) });
    }
  },

  pushIdentityWarning: (warning) =>
    set((state) => ({
      identityWarnings: [
        ...state.identityWarnings.filter((w) => w.peerDeviceId !== warning.peerDeviceId),
        warning,
      ],
    })),

  dismissIdentityWarning: (deviceId) =>
    set((state) => ({
      identityWarnings: state.identityWarnings.filter((w) => w.peerDeviceId !== deviceId),
    })),

  stage: (paths) =>
    set((state) => ({
      // De-duplicate: dropping the same file twice should not send it twice.
      staged: [...new Set([...state.staged, ...paths])],
    })),

  clearStaged: () => set({ staged: [] }),
  setError: (message) => set({ error: message }),
}));

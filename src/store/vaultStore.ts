import { create } from 'zustand';

interface VaultState {
  vaultPath: string | null;
  setVaultPath: (path: string | null) => void;
}

export const useVaultStore = create<VaultState>((set) => ({
  vaultPath: localStorage.getItem('vaultPath'),
  setVaultPath: (path) => {
    if (path) localStorage.setItem('vaultPath', path);
    else localStorage.removeItem('vaultPath');
    set({ vaultPath: path });
  },
}));

import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface VaultState {
  vaultPath: string | null;
  setVaultPath: (path: string | null) => void;
}

export const useVaultStore = create<VaultState>()(
  persist(
    (set) => ({
      vaultPath: null,
      setVaultPath: (vaultPath) => set({ vaultPath }),
    }),
    {
      name: 'crossnotes-vault-storage',
    }
  )
);

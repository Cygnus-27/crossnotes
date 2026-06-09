import { create } from "zustand";
import { persist } from "zustand/middleware";

export interface FavoriteNote {
  path: string;
  name: string;
  /** The vault this note belongs to, so favorites can span vaults. */
  vaultPath: string;
}

interface FavoritesState {
  favorites: FavoriteNote[];
  /** When on, opening a favorite from another vault switches to that vault. */
  switchVaultOnOpen: boolean;
  toggleFavorite: (note: FavoriteNote) => void;
  removeFavorite: (path: string) => void;
  setSwitchVaultOnOpen: (value: boolean) => void;
}

export const useFavoritesStore = create<FavoritesState>()(
  persist(
    (set) => ({
      favorites: [],
      switchVaultOnOpen: true,
      toggleFavorite: (note) =>
        set((state) => ({
          favorites: state.favorites.some((fav) => fav.path === note.path)
            ? state.favorites.filter((fav) => fav.path !== note.path)
            : [...state.favorites, note],
        })),
      removeFavorite: (path) =>
        set((state) => ({
          favorites: state.favorites.filter((fav) => fav.path !== path),
        })),
      setSwitchVaultOnOpen: (switchVaultOnOpen) => set({ switchVaultOnOpen }),
    }),
    { name: "crossnotes-favorites" },
  ),
);

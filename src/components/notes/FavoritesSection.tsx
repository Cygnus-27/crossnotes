import React from "react";
import { useFavoritesStore } from "../../store/favoritesStore";
import { useVault } from "../../hooks/useVault";
import { useVaultStore } from "../../store/vaultStore";
import { useNoteStore } from "../../store/noteStore";

export const FavoritesSection: React.FC = () => {
  const favorites = useFavoritesStore((state) => state.favorites);
  const switchVaultOnOpen = useFavoritesStore((state) => state.switchVaultOnOpen);
  const setSwitchVaultOnOpen = useFavoritesStore(
    (state) => state.setSwitchVaultOnOpen,
  );
  const removeFavorite = useFavoritesStore((state) => state.removeFavorite);
  const { openFavorite } = useVault();
  const vaultPath = useVaultStore((state) => state.vaultPath);
  const activePath = useNoteStore((state) => state.activeNote?.path);

  if (favorites.length === 0) return null;

  return (
    <div className="favorites-section">
      <div className="favorites-header">
        <span className="section-label">Favorites</span>
        <label
          className="favorites-switch"
          title="When on, opening a favorite that lives in another vault switches to that vault"
        >
          <input
            type="checkbox"
            checked={switchVaultOnOpen}
            onChange={(event) => setSwitchVaultOnOpen(event.target.checked)}
          />
          <span>Follow vault</span>
        </label>
      </div>

      <div className="favorites-list">
        {favorites.map((favorite) => {
          const inOtherVault = favorite.vaultPath !== vaultPath;
          const disabled = inOtherVault && !switchVaultOnOpen;
          const isActive = favorite.path === activePath;
          return (
            <div
              key={favorite.path}
              className={[
                "favorite-item",
                isActive ? "is-active" : "",
                disabled ? "is-disabled" : "",
              ]
                .filter(Boolean)
                .join(" ")}
              onClick={() => {
                if (!disabled) openFavorite(favorite);
              }}
              title={
                inOtherVault
                  ? `In another vault: ${favorite.vaultPath}${
                      disabled ? " — enable “Follow vault” to open" : ""
                    }`
                  : favorite.name
              }
            >
              <span className="favorite-star">★</span>
              <span className="favorite-name">{favorite.name}</span>
              {inOtherVault && (
                <span className="favorite-vault-badge" aria-hidden="true">
                  ↗
                </span>
              )}
              <button
                type="button"
                className="favorite-remove"
                aria-label={`Remove ${favorite.name} from favorites`}
                onClick={(event) => {
                  event.stopPropagation();
                  removeFavorite(favorite.path);
                }}
              >
                ×
              </button>
            </div>
          );
        })}
      </div>
    </div>
  );
};

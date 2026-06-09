import React, { useState } from "react";
import { useNoteStore } from "../../store/noteStore";
import { useVault } from "../../hooks/useVault";
import { useUIStore } from "../../store/uiStore";
import {
  noteMarkerOptions,
  useNoteVisualStore,
} from "../../store/noteVisualStore";
import { useSync } from "../../hooks/useSync";
import { useFavoritesStore } from "../../store/favoritesStore";
import { useVaultStore } from "../../store/vaultStore";

export const NoteTree: React.FC = () => {
  const {
    notes,
    activeNote,
    selectedNoteIndex,
    setSelectedNoteIndex,
    isDirty,
  } = useNoteStore();
  const { openNote } = useVault();
  const { focusedElement } = useUIStore();
  const { getMarker, setMarker } = useNoteVisualStore();
  const { isNoteSelected, setNoteSyncEnabled } = useSync();
  const vaultPath = useVaultStore((state) => state.vaultPath);
  const favorites = useFavoritesStore((state) => state.favorites);
  const toggleFavorite = useFavoritesStore((state) => state.toggleFavorite);
  const [openMarkerFor, setOpenMarkerFor] = useState<string | null>(null);

  return (
    <div className="note-list">
      {notes.map((note, index) => {
        const isActive = activeNote?.path === note.path;
        const isSelected =
          index === selectedNoteIndex && focusedElement === "sidebar";
        const showDirty = isActive && isDirty;
        const marker = getMarker(note.path);
        const syncSelected = isNoteSelected(note.path);
        const isFavorite = favorites.some((fav) => fav.path === note.path);
        const className = [
          "note-item",
          isActive ? "is-active" : "",
          isSelected ? "is-selected" : "",
        ]
          .filter(Boolean)
          .join(" ");

        return (
          <div
            key={note.path}
            className={className}
            onClick={() => {
              openNote(note);
              setSelectedNoteIndex(index);
            }}
            onMouseEnter={() => setSelectedNoteIndex(index)}
          >
            <div
              className="note-marker-wrap"
              onClick={(event) => event.stopPropagation()}
            >
              <button
                type="button"
                className="note-marker-button"
                title="Set note marker"
                aria-label={`Set marker for ${note.name}`}
                onClick={() =>
                  setOpenMarkerFor(
                    openMarkerFor === note.path ? null : note.path,
                  )
                }
              >
                {marker}
              </button>
              {openMarkerFor === note.path && (
                <div className="note-marker-menu">
                  {noteMarkerOptions.map((option) => (
                    <button
                      key={option.value}
                      type="button"
                      className={`note-marker-option ${option.value === marker ? "is-selected" : ""}`}
                      onClick={() => {
                        setMarker(note.path, option.value);
                        setOpenMarkerFor(null);
                      }}
                    >
                      <span className="note-marker-option-symbol">
                        {option.value}
                      </span>
                      <span>{option.label}</span>
                    </button>
                  ))}
                </div>
              )}
            </div>
            <span className="note-item-name">{note.name}</span>
            {showDirty && <span className="note-item-dirty" />}
            <button
              type="button"
              className={`note-fav-button ${isFavorite ? "is-favorite" : ""}`}
              title={
                isFavorite ? "Remove from favorites" : "Add to favorites"
              }
              aria-label={
                isFavorite
                  ? `Remove ${note.name} from favorites`
                  : `Add ${note.name} to favorites`
              }
              onClick={(event) => {
                event.stopPropagation();
                if (!vaultPath) return;
                toggleFavorite({
                  path: note.path,
                  name: note.name,
                  vaultPath,
                });
              }}
            >
              {isFavorite ? "★" : "☆"}
            </button>
            <button
              type="button"
              className={`note-sync-button ${syncSelected ? "is-selected" : ""}`}
              title={
                syncSelected
                  ? "Remove this note from sync selection"
                  : "Select this note for sync"
              }
              aria-label={
                syncSelected
                  ? `Remove ${note.name} from sync selection`
                  : `Select ${note.name} for sync`
              }
              onClick={(event) => {
                event.stopPropagation();
                setNoteSyncEnabled(note.path, !syncSelected);
              }}
            >
              {syncSelected ? "↻" : "○"}
            </button>
          </div>
        );
      })}
    </div>
  );
};

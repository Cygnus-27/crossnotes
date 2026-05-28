import React, { useState } from "react";
import { useNoteStore } from "../../store/noteStore";
import { useVault } from "../../hooks/useVault";
import { useUIStore } from "../../store/uiStore";
import {
  noteMarkerOptions,
  useNoteVisualStore,
} from "../../store/noteVisualStore";

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
  const [openMarkerFor, setOpenMarkerFor] = useState<string | null>(null);

  return (
    <div className="note-list">
      {notes.map((note, index) => {
        const isActive = activeNote?.path === note.path;
        const isSelected =
          index === selectedNoteIndex && focusedElement === "sidebar";
        const showDirty = isActive && isDirty;
        const marker = getMarker(note.path);
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
          </div>
        );
      })}
    </div>
  );
};

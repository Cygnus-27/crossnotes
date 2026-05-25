import React from "react";
import { useNoteStore } from "../../store/noteStore";

export const TabBar: React.FC = () => {
  const { openNotes, activeNote, setActiveNote, closeNote, layoutMode } =
    useNoteStore();

  if (layoutMode !== "tabs" || openNotes.length === 0) return null;

  return (
    <div className="tab-bar">
      {openNotes.map((note) => {
        const isActive = activeNote?.path === note.path;

        return (
          <div
            key={note.path}
            className={`tab-item ${isActive ? "is-active" : ""}`}
            onClick={() => setActiveNote(note)}
          >
            {note.name}
            <span
              className="tab-close"
              onClick={(e) => {
                e.stopPropagation();
                closeNote(note.path);
              }}
            >
              ×
            </span>
          </div>
        );
      })}
    </div>
  );
};

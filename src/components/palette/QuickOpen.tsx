import React, { useState, useEffect, useRef } from "react";
import Fuse from "fuse.js";
import { useNoteStore } from "../../store/noteStore";
import { useVault } from "../../hooks/useVault";

interface QuickOpenProps {
  isOpen: boolean;
  setIsOpen: (open: boolean) => void;
}

export const QuickOpen: React.FC<QuickOpenProps> = ({ isOpen, setIsOpen }) => {
  const { notes } = useNoteStore();
  const { openNote } = useVault();
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const fuse = new Fuse(notes, {
    keys: ["name"],
    threshold: 0.4,
  });

  const results = query ? fuse.search(query).map((r) => r.item) : notes;

  useEffect(() => {
    if (isOpen) {
      setTimeout(() => inputRef.current?.focus(), 10);
      setSelectedIndex(0);
      setQuery("");
    }
  }, [isOpen]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!isOpen) return;

      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedIndex((prev) => (prev + 1) % Math.max(results.length, 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedIndex(
          (prev) =>
            (prev - 1 + Math.max(results.length, 1)) %
            Math.max(results.length, 1),
        );
      } else if (e.key === "Enter") {
        e.preventDefault();
        if (results[selectedIndex]) {
          openNote(results[selectedIndex]);
          setIsOpen(false);
        }
      } else if (e.key === "Escape") {
        setIsOpen(false);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, results, selectedIndex, openNote, setIsOpen]);

  if (!isOpen) return null;

  return (
    <div className="modal-backdrop" onClick={() => setIsOpen(false)}>
      <div
        className="modal-panel palette-panel"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="palette-header">Quick open</div>
        <input
          ref={inputRef}
          className="palette-input"
          placeholder="Search notes..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <div className="palette-results">
          {results.length === 0 ? (
            <div className="palette-item">
              <div className="palette-title">No notes found</div>
            </div>
          ) : (
            results.map((note, index) => (
              <div
                key={note.path}
                className={`palette-item ${index === selectedIndex ? "is-selected" : ""}`}
                onClick={() => {
                  openNote(note);
                  setIsOpen(false);
                }}
                onMouseEnter={() => setSelectedIndex(index)}
              >
                <div>
                  <div className="palette-title">{note.name}</div>
                  <div className="palette-subtitle">Markdown note</div>
                </div>
                <span className="keyboard-hint">.md</span>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
};

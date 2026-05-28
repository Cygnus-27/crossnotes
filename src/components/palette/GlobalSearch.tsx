import React, { useState, useEffect, useRef } from "react";
import Fuse from "fuse.js";
import { useNoteStore } from "../../store/noteStore";
import { useVault } from "../../hooks/useVault";

interface GlobalSearchProps {
  isOpen: boolean;
  setIsOpen: (open: boolean) => void;
}

export const GlobalSearch: React.FC<GlobalSearchProps> = ({
  isOpen,
  setIsOpen,
}) => {
  const { notes } = useNoteStore();
  const { openNote } = useVault();
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const fuse = new Fuse(notes, {
    keys: ["name", "content"],
    threshold: 0.4,
    includeMatches: true,
    minMatchCharLength: 2,
  });

  const results = query ? fuse.search(query) : [];

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
          openNote(results[selectedIndex].item);
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
        style={{ width: "min(860px, 94vw)", maxHeight: "72vh" }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="palette-header">Global search</div>
        <input
          ref={inputRef}
          className="palette-input"
          placeholder="Search note contents..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <div className="palette-results" style={{ flex: 1 }}>
          {query && results.length === 0 ? (
            <div className="palette-item">
              <div className="palette-title">
                No matches found for “{query}”
              </div>
            </div>
          ) : query === "" ? (
            <div className="palette-item">
              <div className="palette-title">
                Type to search through all notes
              </div>
            </div>
          ) : (
            results.map((result, index) => {
              const { item } = result;
              const contentMatch = result.matches?.find(
                (match) => match.key === "content",
              );
              let snippet = "";

              if (contentMatch && contentMatch.value) {
                const matchIndex = contentMatch.indices[0][0];
                const start = Math.max(0, matchIndex - 40);
                const end = Math.min(
                  contentMatch.value.length,
                  matchIndex + 90,
                );
                snippet = `${start > 0 ? "…" : ""}${contentMatch.value.substring(start, end)}${end < contentMatch.value.length ? "…" : ""}`;
              } else {
                snippet = `${item.content.substring(0, 100)}...`;
              }

              return (
                <div
                  key={item.path}
                  className={`palette-item ${index === selectedIndex ? "is-selected" : ""}`}
                  onClick={() => {
                    openNote(item);
                    setIsOpen(false);
                  }}
                  onMouseEnter={() => setSelectedIndex(index)}
                  style={{ alignItems: "flex-start" }}
                >
                  <div>
                    <div className="palette-title">{item.name}</div>
                    <div
                      className="palette-subtitle"
                      style={{
                        marginTop: "8px",
                        fontFamily: "var(--font-mono)",
                        lineHeight: 1.45,
                      }}
                    >
                      {snippet}
                    </div>
                  </div>
                </div>
              );
            })
          )}
        </div>
        <div className="palette-footer">
          <span>
            <span className="keyboard-hint">Enter</span> open
          </span>
          <span>
            <span className="keyboard-hint">Esc</span> close
          </span>
        </div>
      </div>
    </div>
  );
};

import React, { useState, useEffect, useRef } from "react";
import Fuse from "fuse.js";
import { useCommandStore } from "../../store/commandStore";

export const CommandPalette: React.FC = () => {
  const { isOpen, commands, setIsOpen } = useCommandStore();
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const fuse = new Fuse(commands, {
    keys: ["name", "category", "description"],
    threshold: 0.4,
  });

  const results = query ? fuse.search(query).map((r) => r.item) : commands;

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
          results[selectedIndex].action();
          setIsOpen(false);
        }
      } else if (e.key === "Escape") {
        setIsOpen(false);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, results, selectedIndex, setIsOpen]);

  if (!isOpen) return null;

  return (
    <div className="modal-backdrop" onClick={() => setIsOpen(false)}>
      <div
        className="modal-panel palette-panel"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="palette-header">Command palette</div>
        <input
          ref={inputRef}
          className="palette-input"
          placeholder="Type a command..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <div className="palette-results">
          {results.length === 0 ? (
            <div className="palette-item">
              <div className="palette-title">No commands found</div>
            </div>
          ) : (
            results.map((cmd, index) => (
              <div
                key={cmd.id}
                className={`palette-item ${index === selectedIndex ? "is-selected" : ""}`}
                onClick={() => {
                  cmd.action();
                  setIsOpen(false);
                }}
                onMouseEnter={() => setSelectedIndex(index)}
              >
                <div>
                  <div className="palette-title">{cmd.name}</div>
                  {cmd.description && (
                    <div className="palette-subtitle">{cmd.description}</div>
                  )}
                </div>
                {cmd.shortcut && (
                  <span className="keyboard-hint">{cmd.shortcut}</span>
                )}
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
};

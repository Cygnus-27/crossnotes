import React from "react";
import { useCommandStore } from "../../store/commandStore";

export const ShortcutCheatSheet: React.FC<{
  isOpen: boolean;
  onClose: () => void;
}> = ({ isOpen, onClose }) => {
  const commands = useCommandStore((state) => state.commands);

  if (!isOpen) return null;

  return (
    <div className="modal-backdrop" style={{ zIndex: 2000 }} onClick={onClose}>
      <div
        className="modal-panel help-panel"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="help-header">
          <div>
            <h2
              style={{
                fontSize: "22px",
                color: "var(--text-primary)",
                marginBottom: "4px",
              }}
            >
              Keyboard Shortcuts
            </h2>
            <p style={{ fontSize: "13px", color: "var(--text-secondary)" }}>
              The hands-never-leave-the-keyboard guide
            </p>
          </div>
          <button type="button" className="icon-button" onClick={onClose}>
            ×
          </button>
        </div>

        <div className="help-body">
          <section>
            <h3 className="help-section-title">Application</h3>
            <div className="help-grid">
              <div className="help-row">
                <span style={{ color: "var(--text-secondary)" }}>
                  Global Search
                </span>
                <span className="keyboard-hint">Ctrl + Shift + F</span>
              </div>
              <div className="help-row">
                <span style={{ color: "var(--text-secondary)" }}>
                  Quick Open
                </span>
                <span className="keyboard-hint">Ctrl + P</span>
              </div>
              <div className="help-row">
                <span style={{ color: "var(--text-secondary)" }}>
                  Command Palette
                </span>
                <span className="keyboard-hint">Ctrl + K</span>
              </div>
              <div className="help-row">
                <span style={{ color: "var(--text-secondary)" }}>
                  Switch Note (Up)
                </span>
                <span className="keyboard-hint">Ctrl + Up</span>
              </div>
              <div className="help-row">
                <span style={{ color: "var(--text-secondary)" }}>
                  Switch Note (Down)
                </span>
                <span className="keyboard-hint">Ctrl + Down</span>
              </div>
              {commands
                .filter(
                  (command) =>
                    ![
                      "global-search",
                      "global-search-open",
                      "quick-open",
                      "show-palette",
                    ].includes(command.id),
                )
                .map((command) => (
                  <div key={command.id} className="help-row">
                    <span style={{ color: "var(--text-secondary)" }}>
                      {command.name}
                    </span>
                    <span className="keyboard-hint">
                      {command.shortcut || "---"}
                    </span>
                  </div>
                ))}
            </div>
          </section>

          <section>
            <h3 className="help-section-title">Vim Mode</h3>
            <div className="help-grid">
              <div className="help-row">
                <span style={{ color: "var(--text-secondary)" }}>
                  Normal Mode
                </span>
                <span className="keyboard-hint">Esc</span>
              </div>
              <div className="help-row">
                <span style={{ color: "var(--text-secondary)" }}>
                  Insert Mode
                </span>
                <span className="keyboard-hint">i</span>
              </div>
              <div className="help-row">
                <span style={{ color: "var(--text-secondary)" }}>
                  Delete Line
                </span>
                <span className="keyboard-hint">dd</span>
              </div>
              <div className="help-row">
                <span style={{ color: "var(--text-secondary)" }}>
                  Copy Line
                </span>
                <span className="keyboard-hint">yy</span>
              </div>
              <div className="help-row">
                <span style={{ color: "var(--text-secondary)" }}>Paste</span>
                <span className="keyboard-hint">p</span>
              </div>
              <div className="help-row">
                <span style={{ color: "var(--text-secondary)" }}>Undo</span>
                <span className="keyboard-hint">u</span>
              </div>
            </div>
          </section>
        </div>

        <div className="help-footer">
          Tip: Press <span className="keyboard-hint">Ctrl + K</span> to search
          for any command instantly.
        </div>
      </div>
    </div>
  );
};

import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

import * as api from "../lib/api";
import { useAppStore } from "../store/appStore";

export function Header() {
  const self = useAppStore((state) => state.self);
  const setSelf = useAppStore((state) => state.setSelf);
  const setError = useAppStore((state) => state.setError);

  const [editingName, setEditingName] = useState(false);
  const [draftName, setDraftName] = useState("");

  if (!self) {
    return (
      <header className="header">
        <span className="muted">Starting…</span>
      </header>
    );
  }

  async function commitName() {
    setEditingName(false);
    const trimmed = draftName.trim();
    if (!trimmed || !self || trimmed === self.settings.deviceName) return;

    try {
      const settings = await api.setDeviceName(trimmed);
      setSelf({ ...self, settings, device: { ...self.device, deviceName: settings.deviceName } });
    } catch (err) {
      setError(String(err));
    }
  }

  async function chooseReceiveDir() {
    if (!self) return;
    try {
      const picked = await open({ directory: true, multiple: false });
      if (typeof picked !== "string") return;
      const settings = await api.setReceiveDir(picked);
      setSelf({ ...self, settings });
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <header className="header">
      <div className="header__identity">
        <span className="logo">fluqsr</span>

        {editingName ? (
          <input
            className="name-input"
            autoFocus
            value={draftName}
            maxLength={64}
            onChange={(e) => setDraftName(e.target.value)}
            onBlur={commitName}
            onKeyDown={(e) => {
              if (e.key === "Enter") void commitName();
              if (e.key === "Escape") setEditingName(false);
            }}
          />
        ) : (
          <button
            type="button"
            className="name-button"
            title="Rename this device"
            onClick={() => {
              setDraftName(self.settings.deviceName);
              setEditingName(true);
            }}
          >
            {self.settings.deviceName}
          </button>
        )}

        <code
          className="fingerprint"
          title={`Full fingerprint:\n${api.groupFingerprint(self.device.fingerprint)}`}
        >
          {self.shortFingerprint}
        </code>
      </div>

      <div className="header__actions">
        <button
          type="button"
          className="link-button"
          title={self.settings.receiveDir}
          onClick={() => void revealItemInDir(self.settings.receiveDir)}
        >
          Open received files
        </button>
        <button type="button" className="link-button" onClick={chooseReceiveDir}>
          Change folder
        </button>
      </div>
    </header>
  );
}

import { open } from "@tauri-apps/plugin-dialog";

import { useAppStore } from "../store/appStore";

/// Where files are staged before a destination is chosen. Dropping is wired up
/// in App.tsx through Tauri's own drag-drop event, because the HTML5 one does
/// not expose real filesystem paths.
export function DropZone() {
  const staged = useAppStore((state) => state.staged);
  const stage = useAppStore((state) => state.stage);
  const clearStaged = useAppStore((state) => state.clearStaged);
  const setError = useAppStore((state) => state.setError);

  async function pick(directory: boolean) {
    try {
      const picked = await open({ multiple: true, directory });
      if (!picked) return;
      stage(Array.isArray(picked) ? picked : [picked]);
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <div className={`dropzone ${staged.length > 0 ? "dropzone--filled" : ""}`}>
      {staged.length === 0 ? (
        <>
          <p className="dropzone__hint">Drop files or folders here</p>
          <div className="dropzone__buttons">
            <button type="button" onClick={() => void pick(false)}>
              Choose files
            </button>
            <button type="button" onClick={() => void pick(true)}>
              Choose folders
            </button>
          </div>
        </>
      ) : (
        <>
          <p className="dropzone__count">
            {staged.length} item{staged.length === 1 ? "" : "s"} ready
          </p>
          <ul className="dropzone__list">
            {staged.slice(0, 5).map((path) => (
              <li key={path} title={path}>
                {basename(path)}
              </li>
            ))}
            {staged.length > 5 && <li className="muted">and {staged.length - 5} more…</li>}
          </ul>
          <div className="dropzone__buttons">
            <button type="button" onClick={() => void pick(false)}>
              Add more
            </button>
            <button type="button" className="ghost" onClick={clearStaged}>
              Clear
            </button>
          </div>
          <p className="dropzone__next">Pick a device below to send to.</p>
        </>
      )}
    </div>
  );
}

function basename(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] || path;
}

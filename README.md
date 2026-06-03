# CrossNotes

crossnotes is a work-in-progress keyboard-first cross-platform markdown note-taking application built for deep focus and high-performance workflows. Eliminating the friction between your thoughts and the screen, CrossNotes allows you to manage your entire vault without ever touching your mouse. 

---

## Key Features

- **Keyboard-First UX**: Navigate, create, search, and manage notes using intuitive hotkeys.
- **Built-in Vim Mode**: High-performance editing with dynamic mode indicators (Normal, Insert, Visual).
- **Zero-Config Persistence**: Automatically remembers your last opened vault and secures filesystem permissions.
- **Privacy First**: Your notes stay on your machine. CrossNotes is a local-first application that works with any folder of markdown files.
- **Local Network Sync**: Discover your other devices on the same Wi-Fi (mDNS), pair them once, and send selected notes directly — no cloud, no account.
- **Cross-OS Vault**: On a dual-boot machine (e.g. Windows + Linux), sync notes between operating systems through a folder both can read.
- **Premium Aesthetics**: A sophisticated "One Dark" inspired interface with charcoal and burnished gold accents.

---

## Essential Shortcuts

| Action | Shortcut |
| :--- | :--- |
| **Command Palette** | `Ctrl + K` |
| **Quick Open Note** | `Ctrl + P` |
| **Global Search** | `Ctrl + Shift + F` |
| **Save Note** | `Ctrl + S` (or Auto-save) |
| **Next / Prev Note** | `Ctrl + Down` / `Ctrl + Up` |
| **Toggle Sidebar** | `Ctrl + \` |
| **Show Cheat Sheet**| `Ctrl + ?` |

---

## Installation and Setup

### Prerequisites (all platforms)

- **Node.js** 18+ and npm
- **Rust** (stable) via [rustup](https://rustup.rs)
- Platform build tooling for Tauri 2:
  - **Linux**: `webkit2gtk4.1`, `libappindicator`/`ayatana-appindicator`, `librsvg`, `base-devel`/`build-essential` (see the [Tauri Linux prerequisites](https://v2.tauri.app/start/prerequisites/)).
  - **Windows**: **Microsoft C++ Build Tools** ("Desktop development with C++") and the **WebView2 Runtime**.
  - **macOS**: **Xcode Command Line Tools** (`xcode-select --install`).

### Run in development

```bash
git clone https://github.com/Cygnus-27/crossnotes
cd crossnotes
npm install
npm run tauri dev
```

`npm run tauri dev` launches the app with hot-reload — use this while testing sync. `npm run tauri build` produces an optimized binary/installer (paths below).

### Linux (Arch / CachyOS / Fedora / Ubuntu)

#### 1. Binary Execution (Fastest)
The optimized release binary is located at `./src-tauri/target/release/crossnotes`.

#### 2. Native Build
1. Clone the repository: `git clone https://github.com/Cygnus-27/crossnotes`
2. Install dependencies: `npm install`
3. Build the binary: `npm run tauri build`
4. Run: `./src-tauri/target/release/crossnotes`

**Desktop Integration:**

For **fish** shell:
```fish
printf "[Desktop Entry]\nName=CrossNotes\nExec=%s\nIcon=%s\nType=Application\nCategories=Utility;" (realpath ./src-tauri/target/release/crossnotes) (realpath ./src-tauri/icons/128x128.png) > ~/.local/share/applications/crossnotes.desktop
```

For **bash/zsh**:
```bash
printf "[Desktop Entry]\nName=CrossNotes\nExec=%s\nIcon=%s\nType=Application\nCategories=Utility;" "$(readlink -f ./src-tauri/target/release/crossnotes)" "$(readlink -f ./src-tauri/icons/128x128.png)" > ~/.local/share/applications/crossnotes.desktop
```

### Windows
1. Install the [prerequisites](#prerequisites-all-platforms): Node.js, Rust (rustup, MSVC toolchain), and the **Microsoft C++ Build Tools**.
2. Open the project in PowerShell: `git clone https://github.com/Cygnus-27/crossnotes; cd crossnotes`.
3. Install dependencies: `npm install`.
4. Develop: `npm run tauri dev` — or build the installer: `npm run tauri build`.
5. The installer is generated at `src-tauri/target/release/bundle/msi/` (and NSIS `.exe` under `bundle/nsis/`).

> **Debloated Windows (e.g. AtlasOS):** CrossNotes renders through the **WebView2 Runtime**. Debloated builds often strip Edge/WebView2, which makes the window open blank. If that happens, install the Evergreen [WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/) and relaunch.

### macOS
1. Open the project in Terminal.
2. Install dependencies: `npm install`.
3. Build the bundle: `npm run tauri build`.
4. Drag the generated `.app` from `src-tauri/target/release/bundle/macos/` to your `Applications`.

---

## Sync

All sync is local and direct — no cloud, no account. Open the **LAN Sync** panel in the sidebar. Mark notes for sync with the ○ toggle next to each note in the list.

### Over the local network (two devices, same Wi-Fi)

For two devices that are powered on at the same time (PC ↔ PC, PC ↔ Mac, PC ↔ phone).

1. On both devices, open **LAN Sync → Find devices** to open the radar (this also makes the device discoverable).
2. **Pair once:** on one device choose *Pair a device* to show a 5-character code; on the other, enter that code. Paired devices light up in the radar.
3. Tap a paired device to send your selected notes. Referenced attachments travel with the notes.

> **macOS** prompts to allow local-network access the first time — allow it. All devices must be on the same network segment (guest/isolated Wi-Fi blocks discovery), and the sync port (TCP 37642) plus mDNS (UDP 5353) must be allowed through the firewall.

### Across a dual-boot machine (Cross-OS Vault)

Two operating systems on one machine are never running at the same time, so network sync can't apply. Instead, CrossNotes exchanges **snapshots through a folder both OSes can read** (e.g. the Windows NTFS partition, or a shared exFAT/NTFS data partition).

1. **One-time (Windows):** disable **Fast Startup / hibernation** so Linux can mount the Windows partition read-write safely. CrossNotes refuses to write to a read-only/dirty location rather than risk corruption.
2. In **LAN Sync → Cross-OS vault**, click **Choose shared folder…** and pick a folder both OSes can see (the *same physical folder*, even though its path differs per OS — e.g. `D:\Notes` on Windows vs `/run/media/<you>/Data/Notes` on Linux). If CrossNotes detects an existing cross-OS vault, it suggests it inline.
3. Toggle **Sync the whole vault** on for everything, or leave it off to sync only the ○-selected notes.
4. **Push** writes a snapshot of this OS's notes to the shared folder. Reboot into the other OS and **Pull** to import anything newer. Each OS keeps its own independent local vault; the shared folder is only a courier.

---

## 📜 License
GNU AGPLv3 © 2026 Cygnus

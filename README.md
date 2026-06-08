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

> ## ⚠️ Read this before creating the shared partition (dual-boot)
>
> **Do not use Windows Disk Management to create or resize the shared partition on a dual-boot disk.** It has been observed to corrupt the **primary GPT header**, which drops Linux into **emergency mode** on the next boot.
>
> **Create the shared partition from Linux with [GParted](https://gparted.org/)** (or `gdisk`) instead — from a live USB if you're shrinking a partition that's currently in use. exFAT or NTFS both work as the shared filesystem.
>
> **After creating it, verify GPT health from Linux** (read-only — these never modify the disk; confirm the device name with `lsblk` first).
>
> Always available (util-linux):
> ```bash
> sudo sfdisk --verify /dev/nvme0n1     # read-only: "looks OK" if healthy
> sudo fdisk  -l       /dev/nvme0n1     # warns explicitly if the primary GPT is corrupt
> ```
> For an explicit "Main/Backup GPT header: OK" report, install `gptfdisk` (Arch/CachyOS: `sudo pacman -S gptfdisk`, Debian/Ubuntu: `sudo apt install gdisk`) and run:
> ```bash
> sudo sgdisk --verify /dev/nvme0n1
> sudo gdisk  -l       /dev/nvme0n1
> ```
>
> **If Linux already dropped to emergency mode (corrupt primary GPT):** when the primary header is damaged but the backup is intact, rebuild it from the backup with `gdisk` (install `gptfdisk` first). `gdisk` only writes to the disk on `w`; everything before it is inspection.
> ```bash
> sudo gdisk /dev/nvme0n1
>   r      # recovery & transformation menu
>   b      # rebuild the main GPT header from the backup
>   w      # write the corrected table to disk, then confirm: y
> ```
> Then reboot. (Always double-check the device with `lsblk` — writing to the wrong disk is destructive.)

**Steps**

1. **One-time (Windows):** disable **Fast Startup / hibernation** so Linux can mount the Windows partition read-write safely. CrossNotes refuses to write to a read-only/dirty location rather than risk corruption.
2. In **LAN Sync → Cross-OS vault**, click **Choose shared folder…** and pick a folder both OSes can see (the *same physical folder*, even though its path differs per OS — e.g. a partition labeled `PORTAL` is `Z:\Notes` on Windows vs `/run/media/<you>/PORTAL/Notes` on Linux). If CrossNotes finds an existing cross-OS vault (even one folder deep), it suggests it inline.
3. Toggle **Sync the whole vault** on for everything, or leave it off to sync only the ○-selected notes.
4. **Push** writes a snapshot of this OS's notes to the shared folder. Reboot into the other OS and **Pull** to import anything newer. Each OS keeps its own independent local vault; the shared folder is only a courier.

**Optional — auto-mount the shared partition on Linux.** If you add an `/etc/fstab` entry so the partition mounts at a fixed path, **always include `nofail`** so a missing or unmountable partition can never block boot (and a short device timeout so boot isn't delayed). Get the UUID with `sudo blkid /dev/nvme0n1p5`:
```fstab
# nofail = never block boot if the partition is absent/unmountable
UUID=<PORTAL-UUID>  /mnt/PORTAL  ntfs3  defaults,nofail,x-systemd.device-timeout=5s,uid=1000,gid=1000  0 0
```

---

## 📜 License
GNU AGPLv3 © 2026 Cygnus

# CrossNotes

CrossNotes is a work-in-progress keyboard-first cross-platform markdown note-taking application built for deep focus and high-performance workflows. Eliminating the friction between your thoughts and the screen, CrossNotes allows you to manage your entire vault without ever touching your mouse. 

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

- **Node.js** 20.19+ or 22.12+ and npm (required by Vite 7)
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

1. `git clone https://github.com/Cygnus-27/crossnotes && cd crossnotes`
2. `npm install`
3. `npm run tauri build`
4. Run the optimized binary at `./src-tauri/target/release/crossnotes` — or use `npm run tauri dev` while developing.

> The repository does not ship prebuilt binaries; the binary above is produced by step 3.

**Desktop Integration** (after building):

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

Setup is a one-time thing per machine. After that, daily use is just **Push** before you reboot out and **Pull** after you reboot in.

#### 1. Create the shared partition (once, from Linux)

Boot Linux (or a [GParted](https://gparted.org/) live USB if you need to shrink the in-use system partition), then in GParted:

1. Shrink an existing partition to free up space (a few hundred MB is plenty for notes).
2. Create a new partition in the free space, formatted **NTFS** or **exFAT** (both are readable by Windows *and* Linux), and set its label to `PORTAL`.
3. Apply, then **verify GPT health** using the read-only commands in the box above. Both headers should report OK before you continue.

#### 2. Make PORTAL mountable on each OS

- **Windows:** it auto-assigns a drive letter (e.g. `Z:`). Turn **Fast Startup off** (Control Panel → Power Options → *Choose what the power buttons do*) and run `powercfg /h off` so Linux can mount it read-write. CrossNotes refuses to write to a read-only/dirty volume rather than risk corruption, so this step matters.
- **Linux — quick:** open PORTAL once in your file manager; it mounts at `/run/media/<you>/PORTAL`.
- **Linux — smoothest (auto-mount at every boot):** add an `/etc/fstab` entry with **`nofail`** (get the UUID from `sudo blkid /dev/nvme0n1p5`). `nofail` guarantees a missing or unmountable PORTAL can **never** block boot:
  ```fstab
  UUID=<PORTAL-UUID>  /mnt/PORTAL  ntfs3  defaults,nofail,x-systemd.device-timeout=5s,uid=1000,gid=1000  0 0
  ```

#### 3. Point CrossNotes at the shared folder (once per OS)

1. In the sidebar, open **LAN Sync → Cross-OS vault → Choose shared folder…** and pick the **same physical folder** on each OS — e.g. `Z:\Notes` on Windows and `/run/media/<you>/PORTAL/Notes` (or `/mnt/PORTAL/Notes`) on Linux.
2. After the first OS pushes a snapshot, the *other* OS **auto-suggests that folder** ("PORTAL/Notes", detected even one level deep) — just click the suggestion.
3. Optionally toggle **Sync the whole vault** (off = only the ○-selected notes are synced).

#### 4. Everyday sync

- **Before rebooting out** of an OS: click **Push** — it writes a snapshot of this OS's notes (and any referenced attachments) to PORTAL.
- **After rebooting into** the other OS: click **Pull** — it imports anything newer.

Each OS keeps its own independent local vault; PORTAL is only the courier. A **Pull** with nothing new reports *"Already up to date."* If you edit the *same* note on both OSes before syncing, the incoming copy is saved alongside as `…conflict-….md` rather than overwriting your local version — no edit is ever lost.

---

## 📜 License
GNU AGPLv3 © 2026 Cygnus

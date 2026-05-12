# CrossNotes

crossnotes is a work-in-progress keyboard-first markdown note-taking application built for deep focus and high-performance workflows. Eliminating the friction between your thoughts and the screen, CrossNotes allows you to manage your entire vault without ever touching your mouse. 

---

## Key Features

- **Keyboard-First UX**: Navigate, create, search, and manage notes using intuitive hotkeys.
- **Built-in Vim Mode**: High-performance editing with dynamic mode indicators (Normal, Insert, Visual).
- **Zero-Config Persistence**: Automatically remembers your last opened vault and secures filesystem permissions.
- **Privacy First**: Your notes stay on your machine. CrossNotes is a local-first application that works with any folder of markdown files.
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
1. Open the project in PowerShell.
2. Install dependencies: `npm install`.
3. Build the installer: `npm run tauri build`.
4. Run the generated `.msi` in `src-tauri/target/release/bundle/msi/`.

### macOS
1. Open the project in Terminal.
2. Install dependencies: `npm install`.
3. Build the bundle: `npm run tauri build`.
4. Drag the generated `.app` from `src-tauri/target/release/bundle/macos/` to your `Applications`.

---

## 📜 License
GNU AGPLv3 © 2026 Cygnus

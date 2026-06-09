import React, { useCallback, useEffect, useRef, useState } from "react";
import { EditorState, StateField } from "@codemirror/state";
import {
  EditorView,
  keymap,
  lineNumbers,
  highlightActiveLine,
  highlightActiveLineGutter,
  Decoration,
  DecorationSet,
  WidgetType,
} from "@codemirror/view";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
import { languages } from "@codemirror/language-data";
import { open } from "@tauri-apps/plugin-dialog";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { vim, Vim } from "@replit/codemirror-vim";
import { useNoteStore } from "../../store/noteStore";
import { useEditorStore } from "../../store/editorStore";
import { useVault } from "../../hooks/useVault";
import { useVaultStore } from "../../store/vaultStore";

type AttachmentInsertMode =
  | "auto"
  | "image"
  | "image-small"
  | "image-medium"
  | "file"
  | "reference";

interface AttachedFile {
  fileName: string;
  relativePath: string;
  isImage: boolean;
}

interface InsertMenuPosition {
  top: number;
  left: number;
}

const imageFilters = [
  {
    name: "Images",
    extensions: ["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "avif"],
  },
];

const getAltText = (fileName: string) =>
  fileName.replace(/\.[^/.]+$/, "").replace(/[-_]+/g, " ");

const currentVaultPath = { value: null as string | null };

const normalizeMarkdownAssetPath = (
  src: string,
  options = { relative: true },
) => {
  const withoutTitle = src.trim().replace(/\s+["'][^"']*["']$/, "");
  const withoutQuery = withoutTitle.split(/[?#]/, 1)[0];
  const path = options.relative
    ? withoutQuery.replace(/^\.\//, "").replace(/^\/+/, "")
    : withoutQuery;

  try {
    return path
      .split("/")
      .map((segment) => decodeURIComponent(segment))
      .join("/");
  } catch {
    return path;
  }
};

const joinVaultPath = (vaultPath: string, relativePath: string) => {
  const normalizedVault = vaultPath.replace(/\\/g, "/").replace(/\/$/, "");
  return `${normalizedVault}/${relativePath}`;
};

const resolveVaultAssetPath = (src: string) => {
  if (!src) return src;

  const isWindowsAbsolutePath = /^[a-z]:[\\/]/i.test(src);
  if (!isWindowsAbsolutePath && src.match(/^[a-z][a-z0-9+.-]*:/i)) {
    return src;
  }

  if (src.startsWith("/") || isWindowsAbsolutePath) {
    return convertFileSrc(normalizeMarkdownAssetPath(src, { relative: false }));
  }

  const normalizedPath = normalizeMarkdownAssetPath(src);

  if (!currentVaultPath.value) {
    return src;
  }

  return convertFileSrc(joinVaultPath(currentVaultPath.value, normalizedPath));
};

class ImagePreviewWidget extends WidgetType {
  constructor(
    private readonly src: string,
    private readonly alt: string,
    private readonly width?: string,
  ) {
    super();
  }

  eq(other: ImagePreviewWidget) {
    return (
      this.src === other.src &&
      this.alt === other.alt &&
      this.width === other.width
    );
  }

  toDOM() {
    const wrap = document.createElement("div");
    wrap.className = "cm-image-preview";

    const img = document.createElement("img");
    img.src = resolveVaultAssetPath(this.src);
    img.alt = this.alt;
    img.loading = "lazy";
    if (this.width) img.style.maxWidth = `${this.width}px`;

    const fallback = document.createElement("div");
    fallback.className = "cm-image-preview-error";
    fallback.textContent = `Could not load image: ${this.src}`;

    img.addEventListener("error", () => {
      img.remove();
      wrap.appendChild(fallback);
    });

    wrap.appendChild(img);

    return wrap;
  }
}

const findImagePreviewDecorations = (state: EditorState) => {
  const decorations = [];
  const markdownImageRegex = /!\[([^\]]*)\]\(([^)]+)\)/g;
  const htmlImageRegex = /<img\s+[^>]*src=["']([^"']+)["'][^>]*>/gi;
  const widthRegex = /width=["']?(\d+)["']?/i;
  const altRegex = /alt=["']([^"']*)["']/i;

  for (let lineNumber = 1; lineNumber <= state.doc.lines; lineNumber += 1) {
    const line = state.doc.line(lineNumber);
    const text = line.text;
    markdownImageRegex.lastIndex = 0;
    htmlImageRegex.lastIndex = 0;

    for (const match of text.matchAll(markdownImageRegex)) {
      const alt = match[1] || "image";
      const src = match[2];
      decorations.push(
        Decoration.widget({
          widget: new ImagePreviewWidget(src, alt),
          block: true,
          side: 1,
        }).range(line.to),
      );
    }

    for (const match of text.matchAll(htmlImageRegex)) {
      const fullMatch = match[0];
      const src = match[1];
      const width = fullMatch.match(widthRegex)?.[1];
      const alt = fullMatch.match(altRegex)?.[1] || "image";
      decorations.push(
        Decoration.widget({
          widget: new ImagePreviewWidget(src, alt, width),
          block: true,
          side: 1,
        }).range(line.to),
      );
    }
  }

  return Decoration.set(decorations, true);
};

const imagePreviewField = StateField.define<DecorationSet>({
  create(state) {
    return findImagePreviewDecorations(state);
  },
  update(decorations, transaction) {
    if (transaction.docChanged) {
      return findImagePreviewDecorations(transaction.state);
    }

    return decorations.map(transaction.changes);
  },
  provide: (field) => EditorView.decorations.from(field),
});

const createMarkdownForAttachment = (
  attachment: AttachedFile,
  mode: AttachmentInsertMode,
) => {
  const alt = getAltText(attachment.fileName);
  const encodedPath = attachment.relativePath
    .split("/")
    .map(encodeURIComponent)
    .join("/");

  if (
    (mode === "image-small" || mode === "image-medium") &&
    attachment.isImage
  ) {
    const width = mode === "image-small" ? 360 : 640;
    return `<img src="./${encodedPath}" width="${width}" alt="${alt}" />`;
  }

  if ((mode === "image" || mode === "auto") && attachment.isImage) {
    return `![${alt}](./${encodedPath})`;
  }

  if (mode === "reference") {
    return `[${alt}](./${encodedPath})`;
  }

  return `[${attachment.fileName}](./${encodedPath})`;
};

export const Editor: React.FC = () => {
  const editorRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const openInsertMenuRef = useRef<() => boolean>(() => false);
  const attachAndInsertRef = useRef<
    (paths: string[], mode: AttachmentInsertMode) => Promise<void>
  >(async () => {});
  const activeNote = useNoteStore((state) => state.activeNote);
  const setEditorInfo = useEditorStore((state) => state.setEditorInfo);
  const vaultPath = useVaultStore((state) => state.vaultPath);
  currentVaultPath.value = vaultPath;
  const { saveNote } = useVault();
  const [insertMenuPosition, setInsertMenuPosition] =
    useState<InsertMenuPosition | null>(null);
  const autoSaveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const saveNoteRef = useRef(saveNote);
  saveNoteRef.current = saveNote;

  const activeNoteRef = useRef(activeNote);
  activeNoteRef.current = activeNote;

  const hideInsertMenu = useCallback(() => {
    setInsertMenuPosition(null);
  }, []);

  const insertAtCursor = useCallback((text: string) => {
    const view = viewRef.current;
    if (!view) return;

    const selection = view.state.selection.main;
    const prefix =
      selection.from > 0 &&
      view.state.doc.sliceString(selection.from - 1, selection.from) !== "\n"
        ? "\n"
        : "";
    const suffix =
      selection.to < view.state.doc.length &&
      view.state.doc.sliceString(selection.to, selection.to + 1) !== "\n"
        ? "\n"
        : "";
    const insert = `${prefix}${text}${suffix}`;

    view.dispatch({
      changes: { from: selection.from, to: selection.to, insert },
      selection: { anchor: selection.from + insert.length },
      scrollIntoView: true,
    });
    view.focus();
  }, []);

  const attachAndInsert = useCallback(
    async (sourcePaths: string[], mode: AttachmentInsertMode) => {
      if (!vaultPath) {
        alert("Open a vault before inserting attachments.");
        return;
      }

      const markdownBlocks: string[] = [];

      try {
        for (const sourcePath of sourcePaths) {
          const attachment = await invoke<AttachedFile>(
            "attach_file_to_vault",
            {
              sourcePath,
              vaultPath,
            },
          );
          markdownBlocks.push(createMarkdownForAttachment(attachment, mode));
        }

        if (markdownBlocks.length > 0) {
          insertAtCursor(markdownBlocks.join("\n"));
        }
      } catch (err) {
        console.error("Failed to insert attachment:", err);
        alert(`Failed to insert attachment.\n${err}`);
      }
    },
    [insertAtCursor, vaultPath],
  );

  attachAndInsertRef.current = attachAndInsert;

  const chooseAndInsert = useCallback(
    async (mode: AttachmentInsertMode) => {
      hideInsertMenu();

      try {
        const selected = await open({
          multiple: mode === "file" || mode === "auto",
          directory: false,
          title: mode.startsWith("image")
            ? "Choose image"
            : "Choose attachment",
          filters: mode.startsWith("image") ? imageFilters : undefined,
        });

        const paths = Array.isArray(selected)
          ? selected
          : selected
            ? [selected]
            : [];
        if (paths.length > 0) {
          await attachAndInsert(paths, mode);
        }
      } catch (err) {
        console.error("Failed to choose attachment:", err);
        alert(`Failed to choose attachment.\n${err}`);
      }
    },
    [attachAndInsert, hideInsertMenu],
  );

  const insertReferencePlaceholder = useCallback(() => {
    hideInsertMenu();
    insertAtCursor("[Reference title](./Attachments/reference.pdf)");
  }, [hideInsertMenu, insertAtCursor]);

  const openInsertMenuAtCursor = useCallback(() => {
    const view = viewRef.current;
    if (!view) return false;

    const cursor = view.state.selection.main.head;
    const coords = view.coordsAtPos(cursor);
    const editorRect = editorRef.current?.getBoundingClientRect();

    setInsertMenuPosition({
      top: (coords?.bottom ?? editorRect?.top ?? 0) + 8,
      left: Math.min(
        coords?.left ?? editorRect?.left ?? 0,
        window.innerWidth - 220,
      ),
    });
    return true;
  }, []);

  openInsertMenuRef.current = openInsertMenuAtCursor;

  useEffect(() => {
    if (!editorRef.current) return;

    const startState = EditorState.create({
      doc: activeNote?.content || "",
      extensions: [
        keymap.of([
          {
            key: "Mod-s",
            run: () => {
              const text = viewRef.current?.state.doc.toString() || "";
              if (activeNoteRef.current) {
                saveNoteRef.current(activeNoteRef.current, text);
              }
              return true;
            },
          },
          {
            key: "Ctrl-Space",
            run: () => openInsertMenuRef.current(),
          },
          {
            key: "Mod-Shift-a",
            run: () => openInsertMenuRef.current(),
          },
          ...defaultKeymap,
          ...historyKeymap,
        ]),
        vim(),
        EditorView.lineWrapping,
        lineNumbers(),
        highlightActiveLineGutter(),
        history(),
        highlightActiveLine(),
        markdown({ base: markdownLanguage, codeLanguages: languages }),
        imagePreviewField,
        EditorView.domEventHandlers({
          drop: (event) => {
            const files = Array.from(event.dataTransfer?.files ?? []);
            const paths = files
              .map((file) => (file as File & { path?: string }).path)
              .filter((path): path is string => Boolean(path));

            if (paths.length === 0) return false;

            event.preventDefault();
            hideInsertMenu();
            attachAndInsertRef.current(paths, "auto").catch((err) => {
              console.error("Failed to insert dropped attachment:", err);
              alert(`Failed to insert dropped attachment.\n${err}`);
            });
            return true;
          },
        }),
        EditorView.theme(
          {
            "&": {
              height: "100%",
              fontSize: "14px",
              backgroundColor: "var(--bg-base)",
              color: "var(--text-primary)",
            },
            "&.cm-focused": {
              outline: "none",
            },
            ".cm-scroller": {
              fontFamily: "var(--font-mono)",
              lineHeight: "1.72",
              padding: "28px 0 40px",
              backgroundColor: "var(--bg-base)",
            },
            ".cm-content": {
              maxWidth: "920px",
              minHeight: "100%",
              padding: "0 48px 32px 40px",
              caretColor: "var(--accent)",
            },
            ".cm-line": {
              padding: "0 2px",
            },
            ".cm-gutters": {
              backgroundColor: "var(--bg-base)",
              color: "var(--text-faint)",
              borderRight: "1px solid var(--border)",
            },
            ".cm-lineNumbers .cm-gutterElement": {
              minWidth: "36px",
              padding: "0 12px 0 14px",
              fontSize: "12px",
            },
            ".cm-activeLine": {
              backgroundColor: "var(--bg-surface)",
            },
            ".cm-activeLineGutter": {
              backgroundColor: "var(--bg-surface)",
              color: "var(--text-secondary)",
            },
            ".cm-cursor": {
              borderLeftColor: "var(--accent)",
              borderLeftWidth: "2px",
            },
            ".cm-selectionBackground, &.cm-focused .cm-selectionBackground, ::selection":
              {
                backgroundColor: "var(--accent-muted)",
              },
            ".cm-matchingBracket, .cm-nonmatchingBracket": {
              backgroundColor: "var(--bg-hover)",
              outline: "1px solid var(--border-strong)",
            },
            ".cm-panels": {
              backgroundColor: "var(--bg-surface)",
              color: "var(--text-primary)",
            },
            ".cm-tooltip": {
              backgroundColor: "var(--bg-surface)",
              border: "1px solid var(--border-strong)",
              color: "var(--text-primary)",
            },
          },
          { dark: true },
        ),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) hideInsertMenu();

          if (update.selectionSet || update.docChanged) {
            const state = update.state;
            const pos = state.selection.main.head;
            const line = state.doc.lineAt(pos);
            const col = pos - line.from + 1;

            setEditorInfo({
              line: line.number,
              col,
              wordCount: state.doc.toString().trim()
                ? state.doc.toString().trim().split(/\s+/).length
                : 0,
            });

            if (update.docChanged && activeNoteRef.current) {
              const { setIsDirty } = useNoteStore.getState();
              setIsDirty(true);
              const currentNote = activeNoteRef.current;
              const textToSave = update.state.doc.toString();

              if (autoSaveTimerRef.current)
                clearTimeout(autoSaveTimerRef.current);
              autoSaveTimerRef.current = setTimeout(() => {
                saveNoteRef.current(currentNote, textToSave);
              }, 500);
            }
          }
        }),
      ],
    });

    const view = new EditorView({
      state: startState,
      parent: editorRef.current,
    });

    viewRef.current = view;

    if (Vim && (Vim as any).on) {
      try {
        (Vim as any).on("mode-change", (data: any) => {
          const mode = data.mode;
          let label = "NORMAL";
          if (mode === "insert") label = "INSERT";
          else if (mode === "visual") label = "VISUAL";
          else if (mode === "replace") label = "REPLACE";
          setEditorInfo({ vimMode: label });
        });
      } catch (err) {
        console.error("Failed to attach Vim listener:", err);
      }
    }

    return () => {
      if (autoSaveTimerRef.current) clearTimeout(autoSaveTimerRef.current);
      if (Vim && (Vim as any).off) {
        try {
          (Vim as any).off("mode-change", () => {});
        } catch {}
      }
      view.destroy();
    };
  }, []);

  useEffect(() => {
    if (viewRef.current && activeNote) {
      const currentDoc = viewRef.current.state.doc.toString();
      if (currentDoc !== activeNote.content) {
        viewRef.current.dispatch({
          changes: {
            from: 0,
            to: currentDoc.length,
            insert: activeNote.content || "",
          },
        });
      }
    }
  }, [activeNote?.path, activeNote?.content]);

  return (
    <div ref={editorRef} className="editor-root">
      {insertMenuPosition && (
        <div
          className="insert-menu"
          style={{ top: insertMenuPosition.top, left: insertMenuPosition.left }}
          onMouseDown={(event) => event.preventDefault()}
        >
          <div className="insert-menu-title">Insert</div>
          <button
            type="button"
            className="insert-menu-item"
            onClick={() => chooseAndInsert("image")}
          >
            Image
          </button>
          <button
            type="button"
            className="insert-menu-item"
            onClick={() => chooseAndInsert("image-small")}
          >
            Image · small
          </button>
          <button
            type="button"
            className="insert-menu-item"
            onClick={() => chooseAndInsert("image-medium")}
          >
            Image · medium
          </button>
          <button
            type="button"
            className="insert-menu-item"
            onClick={() => chooseAndInsert("file")}
          >
            File attachment
          </button>
          <button
            type="button"
            className="insert-menu-item"
            onClick={insertReferencePlaceholder}
          >
            Reference link
          </button>
          <div className="insert-menu-hint">Ctrl + Space</div>
        </div>
      )}
    </div>
  );
};

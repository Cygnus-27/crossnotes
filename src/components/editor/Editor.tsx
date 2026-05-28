import React, { useEffect, useRef } from "react";
import { EditorState } from "@codemirror/state";
import {
  EditorView,
  keymap,
  lineNumbers,
  highlightActiveLine,
  highlightActiveLineGutter,
} from "@codemirror/view";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
import { languages } from "@codemirror/language-data";

import { vim, Vim } from "@replit/codemirror-vim";
import { useNoteStore } from "../../store/noteStore";
import { useEditorStore } from "../../store/editorStore";
import { useVault } from "../../hooks/useVault";

export const Editor: React.FC = () => {
  const editorRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const activeNote = useNoteStore((state) => state.activeNote);
  const setEditorInfo = useEditorStore((state) => state.setEditorInfo);
  const { saveNote } = useVault();
  const autoSaveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const saveNoteRef = useRef(saveNote);
  saveNoteRef.current = saveNote;

  const activeNoteRef = useRef(activeNote);
  activeNoteRef.current = activeNote;

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
          ...defaultKeymap,
          ...historyKeymap,
        ]),
        vim(),
        lineNumbers(),
        highlightActiveLineGutter(),
        history(),
        highlightActiveLine(),
        markdown({ base: markdownLanguage, codeLanguages: languages }),
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

  return <div ref={editorRef} style={{ height: "100%", width: "100%" }} />;
};

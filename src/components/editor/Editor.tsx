import React, { useEffect, useRef } from 'react';
import { EditorState } from '@codemirror/state';
import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter } from '@codemirror/view';
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
import { languages } from '@codemirror/language-data';
import { oneDark } from '@codemirror/theme-one-dark';
import { vim } from '@replit/codemirror-vim';
import { useNoteStore } from '../../store/noteStore';
import { useEditorStore } from '../../store/editorStore';
import { useVault } from '../../hooks/useVault';

export const Editor: React.FC = () => {
  const editorRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const activeNote = useNoteStore((state) => state.activeNote);
  const setEditorInfo = useEditorStore((state) => state.setEditorInfo);
  const { saveNote } = useVault();
  const autoSaveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (!editorRef.current) return;

    const startState = EditorState.create({
      doc: activeNote?.content || '',
      extensions: [
        vim(),
        lineNumbers(),
        highlightActiveLineGutter(),
        history(),
        highlightActiveLine(),
        markdown({ base: markdownLanguage, codeLanguages: languages }),
        oneDark,
        keymap.of([
          ...defaultKeymap,
          ...historyKeymap,
        ]),
        EditorView.theme({
          "&": { height: "100%", fontSize: "14px" },
          ".cm-scroller": { fontFamily: "var(--font-mono)" },
          ".cm-activeLine": { backgroundColor: "rgba(255, 255, 255, 0.05)" },
        }),
        EditorView.updateListener.of((update) => {
          if (update.selectionSet || update.docChanged) {
            const state = update.state;
            const pos = state.selection.main.head;
            const line = state.doc.lineAt(pos);
            const col = pos - line.from + 1;
            
            const text = state.doc.toString();
            const wordCount = text.trim() ? text.trim().split(/\s+/).length : 0;
            
            setEditorInfo({ line: line.number, col, wordCount });

            if (update.docChanged && activeNote) {
               if (autoSaveTimerRef.current) clearTimeout(autoSaveTimerRef.current);
               autoSaveTimerRef.current = setTimeout(() => {
                 saveNote(activeNote, text);
               }, 500); // 500ms debounce
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

    return () => {
      if (autoSaveTimerRef.current) clearTimeout(autoSaveTimerRef.current);
      view.destroy();
    };
  }, []);

  // Update content when activeNote changes (if it's a different note)
  useEffect(() => {
    if (viewRef.current && activeNote) {
      // Check if we need to reload the editor content
      // We use a property to check if it's the same note to avoid overwriting current edits
      if (viewRef.current.state.doc.toString() !== activeNote.content) {
         // Only replace if the content is significantly different (init load)
         // or if we have a better way to track "current note path"
      }
    }
  }, [activeNote]);

  return <div ref={editorRef} style={{ height: '100%', width: '100%' }} />;
};

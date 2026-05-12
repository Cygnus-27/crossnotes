import { create } from 'zustand';

interface EditorState {
  line: number;
  col: number;
  wordCount: number;
  setEditorInfo: (info: { line: number; col: number; wordCount: number }) => void;
}

export const useEditorStore = create<EditorState>((set) => ({
  line: 1,
  col: 1,
  wordCount: 0,
  setEditorInfo: (info) => set(info),
}));

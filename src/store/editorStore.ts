import { create } from 'zustand';

interface EditorState {
  line: number;
  col: number;
  wordCount: number;
  vimMode: string;
  setEditorInfo: (info: Partial<{ line: number; col: number; wordCount: number; vimMode: string }>) => void;
}

export const useEditorStore = create<EditorState>((set) => ({
  line: 1,
  col: 1,
  wordCount: 0,
  vimMode: 'NORMAL',
  setEditorInfo: (info) => set((state) => ({ ...state, ...info })),
}));

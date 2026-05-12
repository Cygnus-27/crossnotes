import { create } from 'zustand';

export interface Note {
  path: string;
  name: string;
  content: string;
  lastModified?: number;
}

interface NoteState {
  notes: Note[];
  activeNote: Note | null;
  openNotes: Note[]; // For tabs mode
  layoutMode: 'tabs' | 'history';
  selectedNoteIndex: number;
  isDirty: boolean;
  setNotes: (notes: Note[]) => void;
  setActiveNote: (note: Note | null) => void;
  setSelectedNoteIndex: (index: number) => void;
  setLayoutMode: (mode: 'tabs' | 'history') => void;
  setIsDirty: (dirty: boolean) => void;
  closeNote: (path: string) => void;
}

export const useNoteStore = create<NoteState>((set) => ({
  notes: [],
  activeNote: null,
  openNotes: [],
  layoutMode: 'history',
  selectedNoteIndex: 0,
  isDirty: false,
  setNotes: (notes) => set({ notes, selectedNoteIndex: 0 }),
  setIsDirty: (isDirty) => set({ isDirty }),
  setActiveNote: (note) => set((state) => {
    if (!note) return { activeNote: null };
    
    const index = state.notes.findIndex(n => n.path === note.path);
    const isAlreadyOpen = state.openNotes.find(n => n.path === note.path);
    const newOpenNotes = isAlreadyOpen 
      ? state.openNotes 
      : [...state.openNotes, note];
      
    return { 
      activeNote: note,
      openNotes: newOpenNotes,
      selectedNoteIndex: index >= 0 ? index : state.selectedNoteIndex
    };
  }),
  setSelectedNoteIndex: (selectedNoteIndex) => set({ selectedNoteIndex }),
  setLayoutMode: (layoutMode) => set({ layoutMode }),
  closeNote: (path) => set((state) => {
    const newOpenNotes = state.openNotes.filter(n => n.path !== path);
    let newActiveNote = state.activeNote;
    
    if (state.activeNote?.path === path) {
      newActiveNote = newOpenNotes[newOpenNotes.length - 1] || null;
    }
    
    return { openNotes: newOpenNotes, activeNote: newActiveNote };
  }),
}));

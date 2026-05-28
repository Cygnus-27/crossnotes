import { create } from "zustand";

export interface Note {
  path: string;
  name: string;
  content: string;
  lastModified?: number;
}

interface NoteState {
  notes: Note[];
  activeNote: Note | null;
  openNotes: Note[];
  layoutMode: "tabs" | "history";
  selectedNoteIndex: number;
  isDirty: boolean;
  setNotes: (notes: Note[]) => void;
  setActiveNote: (note: Note | null) => void;
  setSelectedNoteIndex: (index: number) => void;
  setLayoutMode: (mode: "tabs" | "history") => void;
  setIsDirty: (dirty: boolean) => void;
  closeNote: (path: string) => void;
  resetWorkspace: () => void;
}

export const useNoteStore = create<NoteState>((set) => ({
  notes: [],
  activeNote: null,
  openNotes: [],
  layoutMode: "history",
  selectedNoteIndex: 0,
  isDirty: false,
  setNotes: (notes) =>
    set((state) => {
      const activeNoteStillExists =
        state.activeNote &&
        notes.some((note) => note.path === state.activeNote?.path);
      const openNotes = state.openNotes.filter((openNote) =>
        notes.some((note) => note.path === openNote.path),
      );

      return {
        notes,
        openNotes,
        activeNote: activeNoteStillExists ? state.activeNote : null,
        selectedNoteIndex:
          notes.length > 0
            ? Math.min(state.selectedNoteIndex, notes.length - 1)
            : 0,
        isDirty: activeNoteStillExists ? state.isDirty : false,
      };
    }),
  setIsDirty: (isDirty) => set({ isDirty }),
  setActiveNote: (note) =>
    set((state) => {
      if (!note) {
        return {
          activeNote: null,
          openNotes: [],
          isDirty: false,
          selectedNoteIndex: 0,
        };
      }

      const index = state.notes.findIndex((n) => n.path === note.path);
      const existingOpenNotes = state.openNotes.filter((openNote) =>
        state.notes.some((n) => n.path === openNote.path),
      );
      const alreadyOpen = existingOpenNotes.some((n) => n.path === note.path);
      const openNotes = alreadyOpen
        ? existingOpenNotes
        : [...existingOpenNotes, note];

      return {
        activeNote: note,
        openNotes,
        selectedNoteIndex: index >= 0 ? index : state.selectedNoteIndex,
      };
    }),
  setSelectedNoteIndex: (selectedNoteIndex) => set({ selectedNoteIndex }),
  setLayoutMode: (layoutMode) => set({ layoutMode }),
  closeNote: (path) =>
    set((state) => {
      const openNotes = state.openNotes.filter((note) => note.path !== path);
      const activeNote =
        state.activeNote?.path === path
          ? openNotes[openNotes.length - 1] || null
          : state.activeNote;

      return {
        openNotes,
        activeNote,
        isDirty: state.activeNote?.path === path ? false : state.isDirty,
      };
    }),
  resetWorkspace: () =>
    set({
      notes: [],
      activeNote: null,
      openNotes: [],
      selectedNoteIndex: 0,
      isDirty: false,
    }),
}));

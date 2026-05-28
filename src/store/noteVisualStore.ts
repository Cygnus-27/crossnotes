import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface NoteVisualState {
  markers: Record<string, string>;
  setMarker: (notePath: string, marker: string) => void;
  getMarker: (notePath: string) => string;
}

export const defaultNoteMarker = '◇';

export const noteMarkerOptions = [
  { value: '◇', label: 'General' },
  { value: '📘', label: 'Study' },
  { value: '🧪', label: 'Research' },
  { value: '>/', label: 'Code' },
  { value: '✎', label: 'Writing' },
  { value: '✓', label: 'Tasks' },
  { value: '💡', label: 'Ideas' },
  { value: '⌁', label: 'Reference' },
];

export const useNoteVisualStore = create<NoteVisualState>()(
  persist(
    (set, get) => ({
      markers: {},
      setMarker: (notePath, marker) => set((state) => ({
        markers: {
          ...state.markers,
          [notePath]: marker,
        },
      })),
      getMarker: (notePath) => get().markers[notePath] ?? defaultNoteMarker,
    }),
    {
      name: 'crossnotes-note-visuals',
    }
  )
);

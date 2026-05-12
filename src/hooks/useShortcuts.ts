import { useEffect } from 'react';
import { useUIStore } from '../store/uiStore';
import { useCommandStore } from '../store/commandStore';
import { useNoteStore } from '../store/noteStore';
import { useVault } from './useVault';

interface ShortcutProps {
  onOpenVault: () => void;
}

export const useShortcuts = ({ onOpenVault }: ShortcutProps) => {
  const { toggleSidebar, toggleTheme, setQuickOpenOpen, setHelpOpen, setGlobalSearchOpen, focusedElement, setFocusedElement } = useUIStore();
  const setPaletteOpen = useCommandStore((state) => state.setIsOpen);
  const { activeNote, notes, openNotes, setActiveNote, closeNote, selectedNoteIndex, setSelectedNoteIndex } = useNoteStore();
  const { openNote, createNote, renameNote, deleteNote } = useVault();

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const isMod = e.ctrlKey || e.metaKey;

      // Global Esc: Close palettes or focus editor
      if (e.key === 'Escape') {
        setPaletteOpen(false);
        setQuickOpenOpen(false);
        setHelpOpen(false);
        setGlobalSearchOpen(false);
        setFocusedElement('editor');
        return;
      }
      
      // ... later ...
      // Global Search: Ctrl + Shift + F
      if (isMod && e.shiftKey && (e.key === 'F' || e.key === 'f')) {
        e.preventDefault();
        setGlobalSearchOpen(true);
      }

      // If a modal is open, don't trigger other shortcuts
      const isModalOpen = useCommandStore.getState().isOpen || useUIStore.getState().quickOpenOpen || useUIStore.getState().helpOpen;
      if (isModalOpen) return;

      // Focus Sidebar: Alt + 1 or Ctrl + \ (when sidebar is hidden, show it)
      if (isMod && e.key === '1') {
        e.preventDefault();
        setFocusedElement('sidebar');
      }

      // Focus Editor: Alt + 2
      if (isMod && e.key === '2') {
        e.preventDefault();
        setFocusedElement('editor');
      }

      // New Note: Ctrl + N
      if (isMod && e.key === 'n') {
        e.preventDefault();
        const name = prompt('Note name:');
        if (name) createNote(name);
      }

      // Save Note: Ctrl + S (Force save)
      if (isMod && e.key === 's') {
        e.preventDefault();
        if (activeNote) {
          // Manual save logic if needed, though auto-save is on
        }
      }

      // Navigation (Tier 1) - Only when sidebar is focused
      if (focusedElement === 'sidebar') {
        if (e.key === 'j') {
          e.preventDefault();
          setSelectedNoteIndex((selectedNoteIndex + 1) % notes.length);
        } else if (e.key === 'k') {
          e.preventDefault();
          setSelectedNoteIndex((selectedNoteIndex - 1 + notes.length) % notes.length);
        } else if (e.key === 'Enter') {
          e.preventDefault();
          const note = notes[selectedNoteIndex];
          if (note) {
            openNote(note);
            setFocusedElement('editor');
          }
        } else if (e.key === 'o') {
          e.preventDefault();
          const name = prompt('New note:');
          if (name) createNote(name);
        } else if (e.key === 'r') {
          e.preventDefault();
          const note = notes[selectedNoteIndex];
          if (note) {
            const name = prompt('Rename to:', note.name);
            if (name) renameNote(note, name);
          }
        } else if (e.key === 'd') {
          e.preventDefault();
          const note = notes[selectedNoteIndex];
          if (note && confirm(`Delete "${note.name}"?`)) {
            deleteNote(note);
          }
        } else if (e.key === '/') {
          e.preventDefault();
          setQuickOpenOpen(true);
        }
      }

      // Toggle Sidebar: Ctrl + \
      if (isMod && e.key === '\\') {
        e.preventDefault();
        toggleSidebar();
      }

      // Open Vault: Ctrl + O
      if (isMod && e.key === 'o') {
        e.preventDefault();
        onOpenVault();
      }

      // Help / Shortcuts: Ctrl + ? or Ctrl + /
      if (isMod && (e.key === '?' || e.key === '/')) {
        e.preventDefault();
        setHelpOpen(true);
      }

      // Quick Open: Ctrl + P
      if (isMod && e.key === 'p') {
        e.preventDefault();
        setQuickOpenOpen(true);
      }

      // Close Note/Tab: Ctrl + W
      if (isMod && e.key === 'w') {
        e.preventDefault();
        if (activeNote) closeNote(activeNote.path);
      }

      // Switch Tabs: Ctrl (+ Shift) + Tab
      if (isMod && e.key === 'Tab') {
        e.preventDefault();
        if (openNotes.length > 1) {
          const currentIndex = openNotes.findIndex(n => n.path === activeNote?.path);
          const nextIndex = e.shiftKey 
            ? (currentIndex - 1 + openNotes.length) % openNotes.length
            : (currentIndex + 1) % openNotes.length;
          setActiveNote(openNotes[nextIndex]);
        }
      }

      // Open Command Palette: Ctrl + K
      if (isMod && e.key === 'k') {
        e.preventDefault();
        setPaletteOpen(true);
      }

      // Quick Open: Ctrl + P
      if (isMod && e.key === 'p') {
        e.preventDefault();
        console.log('Open Quick Open');
      }

      // New Note: Ctrl + N
      if (isMod && e.key === 'n') {
        e.preventDefault();
        console.log('New Note');
      }
      
      // Toggle Theme: Ctrl + Shift + L
      if (isMod && e.shiftKey && e.key === 'L') {
        e.preventDefault();
        toggleTheme();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [toggleSidebar, toggleTheme, onOpenVault]);
};

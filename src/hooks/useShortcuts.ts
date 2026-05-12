import { useEffect } from 'react';
import { useUIStore } from '../store/uiStore';
import { useCommandStore } from '../store/commandStore';
import { useNoteStore } from '../store/noteStore';
import { useVault } from './useVault';

interface ShortcutProps {
  onOpenVault: () => void;
}

export const useShortcuts = ({ onOpenVault }: ShortcutProps) => {
  const { openNote, createNote, renameNote, deleteNote } = useVault();

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const isMod = e.ctrlKey || e.metaKey;
      const { focusedElement, setFocusedElement, toggleSidebar, toggleTheme, setQuickOpenOpen, setHelpOpen, setGlobalSearchOpen } = useUIStore.getState();
      const { setIsOpen: setPaletteOpen } = useCommandStore.getState();
      const { notes, activeNote, selectedNoteIndex, setSelectedNoteIndex, closeNote } = useNoteStore.getState();

      // Global Esc: Close palettes or focus editor
      if (e.key === 'Escape') {
        const isAnyPaletteOpen = useCommandStore.getState().isOpen || useUIStore.getState().quickOpenOpen || useUIStore.getState().helpOpen || useUIStore.getState().globalSearchOpen;
        
        if (isAnyPaletteOpen) {
          setPaletteOpen(false);
          setQuickOpenOpen(false);
          setHelpOpen(false);
          setGlobalSearchOpen(false);
        } else {
          setFocusedElement('editor');
        }
        return;
      }

      // Global Search: Ctrl + Shift + F
      if (isMod && e.shiftKey && (e.key === 'F' || e.key === 'f')) {
        e.preventDefault();
        setGlobalSearchOpen(true);
        return;
      }

      // If a modal is open, don't trigger other shortcuts
      const isModalOpen = useCommandStore.getState().isOpen || useUIStore.getState().quickOpenOpen || useUIStore.getState().helpOpen || useUIStore.getState().globalSearchOpen;
      if (isModalOpen) return;

      // Focus Sidebar: Alt + 1
      if (e.altKey && e.key === '1') {
        e.preventDefault();
        setFocusedElement('sidebar');
      }

      // Focus Editor: Alt + 2
      if (e.altKey && e.key === '2') {
        e.preventDefault();
        setFocusedElement('editor');
      }

      // Sidebar Specific Navigation (Tier 1) - Vim keys
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
        } else if (e.key === 'o' && !isMod) {
          e.preventDefault();
          const name = prompt('New note:');
          if (name) createNote(name);
        } else if (e.key === 'r' && !isMod) {
          e.preventDefault();
          const note = notes[selectedNoteIndex];
          if (note) {
            const name = prompt('Rename to:', note.name);
            if (name) renameNote(note, name);
          }
        } else if (e.key === 'd' && !isMod) {
          e.preventDefault();
          const note = notes[selectedNoteIndex];
          if (note && confirm(`Delete "${note.name}"?`)) {
            deleteNote(note);
          }
        } else if (e.key === '/' && !isMod) {
          e.preventDefault();
          setQuickOpenOpen(true);
        }
      }

      // Global Shortcuts (Working even when Editor is focused)
      
      // New Note: Ctrl + N
      if (isMod && (e.key === 'n' || e.key === 'N')) {
        e.preventDefault();
        const name = prompt('Note name:');
        if (name) createNote(name);
      }

      // Open Vault: Ctrl + O
      if (isMod && (e.key === 'o' || e.key === 'O')) {
        e.preventDefault();
        onOpenVault();
      }

      // Toggle Sidebar: Ctrl + \
      if (isMod && e.key === '\\') {
        e.preventDefault();
        toggleSidebar();
      }

      // Help / Shortcuts: Ctrl + ?
      if (isMod && e.key === '?') {
        e.preventDefault();
        setHelpOpen(true);
      }

      // Quick Open: Ctrl + P
      if (isMod && (e.key === 'p' || e.key === 'P')) {
        e.preventDefault();
        setQuickOpenOpen(true);
      }

      // Command Palette: Ctrl + K
      if (isMod && (e.key === 'k' || e.key === 'K')) {
        e.preventDefault();
        setPaletteOpen(true);
      }

      // Toggle Theme: Ctrl + Shift + L
      if (isMod && e.shiftKey && (e.key === 'L' || e.key === 'l')) {
        e.preventDefault();
        toggleTheme();
      }

      // Rename Note: Ctrl + Shift + R
      if (isMod && e.shiftKey && (e.key === 'R' || e.key === 'r')) {
        e.preventDefault();
        const { notes, activeNote, selectedNoteIndex } = useNoteStore.getState();
        const note = activeNote || notes[selectedNoteIndex];
        if (note) {
          const name = prompt('New name:', note.name);
          if (name) renameNote(note, name);
        }
      }

      // Delete Note: Ctrl + Shift + D
      if (isMod && e.shiftKey && (e.key === 'D' || e.key === 'd')) {
        e.preventDefault();
        const { notes, activeNote, selectedNoteIndex } = useNoteStore.getState();
        const note = activeNote || notes[selectedNoteIndex];
        if (note && confirm(`Delete "${note.name}"?`)) {
          deleteNote(note);
        }
      }

      // Switch to Tabs: Ctrl + Shift + T
      if (isMod && e.shiftKey && (e.key === 'T' || e.key === 't')) {
        e.preventDefault();
        useNoteStore.getState().setLayoutMode('tabs');
      }

      // Switch to History: Ctrl + Shift + H
      if (isMod && e.shiftKey && (e.key === 'H' || e.key === 'h')) {
        e.preventDefault();
        useNoteStore.getState().setLayoutMode('history');
      }

      // Save Note: Ctrl + S
      if (isMod && (e.key === 's' || e.key === 'S')) {
        e.preventDefault();
        // The actual save is handled by CodeMirror keymap for accuracy,
        // but we prevent browser default here.
      }

      // Tab Management
      if (isMod && (e.key === 'w' || e.key === 'W')) {
        e.preventDefault();
        if (activeNote) closeNote(activeNote.path);
      }

      // Switch Note: Ctrl + Up/Down
      if (isMod && e.key === 'ArrowUp') {
        e.preventDefault();
        const { notes, activeNote, setSelectedNoteIndex } = useNoteStore.getState();
        if (notes.length <= 1) return;
        const currentIndex = notes.findIndex(n => n.path === activeNote?.path);
        const prevIndex = (currentIndex - 1 + notes.length) % notes.length;
        const note = notes[prevIndex];
        if (note) {
          openNote(note);
          setSelectedNoteIndex(prevIndex);
        }
      }
      if (isMod && e.key === 'ArrowDown') {
        e.preventDefault();
        const { notes, activeNote, setSelectedNoteIndex } = useNoteStore.getState();
        if (notes.length <= 1) return;
        const currentIndex = notes.findIndex(n => n.path === activeNote?.path);
        const nextIndex = (currentIndex + 1) % notes.length;
        const note = notes[nextIndex];
        if (note) {
          openNote(note);
          setSelectedNoteIndex(nextIndex);
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown, true); // Use capture phase to beat CodeMirror
    return () => window.removeEventListener('keydown', handleKeyDown, true);
  }, [onOpenVault, openNote, createNote, renameNote, deleteNote]);
};

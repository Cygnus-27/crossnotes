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

      // Tab Management
      if (isMod && (e.key === 'w' || e.key === 'W')) {
        e.preventDefault();
        if (activeNote) closeNote(activeNote.path);
      }
    };

    window.addEventListener('keydown', handleKeyDown, true); // Use capture phase to beat CodeMirror
    return () => window.removeEventListener('keydown', handleKeyDown, true);
  }, [onOpenVault, openNote, createNote, renameNote, deleteNote]);
};

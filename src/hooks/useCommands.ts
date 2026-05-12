import { useEffect } from 'react';
import { useCommandStore } from '../store/commandStore';
import { useUIStore } from '../store/uiStore';
import { useNoteStore } from '../store/noteStore';
import { useVault } from './useVault';

export const useCommands = () => {
  const registerCommand = useCommandStore((state) => state.registerCommand);
  const { toggleTheme, toggleSidebar, setHelpOpen, setGlobalSearchOpen } = useUIStore();
  const { setLayoutMode, notes, selectedNoteIndex, activeNote } = useNoteStore();
  const { openVault, createNote, renameNote, deleteNote } = useVault();

  useEffect(() => {
    registerCommand({
      id: 'global-search',
      name: 'Global Search',
      shortcut: 'Ctrl+Shift+F',
      description: 'Search through the contents of all notes',
      action: () => setGlobalSearchOpen(true),
      category: 'Search'
    });

    registerCommand({
      id: 'show-help',
      name: 'Show Shortcuts & Help',
      shortcut: 'Ctrl+?',
      description: 'View the keyboard shortcut reference',
      action: () => setHelpOpen(true),
      category: 'Help'
    });

    registerCommand({
      id: 'new-note',
      name: 'New Note',
      shortcut: 'Ctrl+N',
      description: 'Create a new markdown note in the current vault',
      action: () => {
        const name = prompt('Note name:');
        if (name) createNote(name);
      },
      category: 'File'
    });

    registerCommand({
      id: 'rename-note',
      name: 'Rename Current Note',
      shortcut: 'Ctrl+Shift+R',
      description: 'Rename the currently selected or active note',
      action: () => {
        const note = activeNote || notes[selectedNoteIndex];
        if (note) {
          const name = prompt('New name:', note.name);
          if (name) renameNote(note, name);
        }
      },
      category: 'File'
    });

    registerCommand({
      id: 'delete-note',
      name: 'Delete Note',
      shortcut: 'Ctrl+Shift+D',
      description: 'Permanently delete the selected note',
      action: () => {
        const note = notes[selectedNoteIndex];
        if (note && confirm(`Delete "${note.name}"?`)) {
          deleteNote(note);
        }
      },
      category: 'File'
    });

    registerCommand({
      id: 'switch-to-tabs',
      name: 'Switch to Tabs Layout',
      shortcut: 'Ctrl+Shift+T',
      description: 'Show multiple open notes as tabs',
      action: () => setLayoutMode('tabs'),
      category: 'Layout'
    });

    registerCommand({
      id: 'switch-to-history',
      name: 'Switch to History Layout',
      shortcut: 'Ctrl+Shift+H',
      description: 'Show single note with history navigation',
      action: () => setLayoutMode('history'),
      category: 'Layout'
    });

    registerCommand({
      id: 'open-vault',
      name: 'Open Vault',
      shortcut: 'Ctrl+O',
      description: 'Select a directory containing markdown notes',
      action: openVault,
      category: 'File'
    });

    registerCommand({
      id: 'toggle-theme',
      name: 'Toggle Theme',
      shortcut: 'Ctrl+Shift+L',
      description: 'Switch between dark and parchment themes',
      action: toggleTheme,
      category: 'View'
    });

    registerCommand({
      id: 'toggle-sidebar',
      name: 'Toggle Sidebar',
      shortcut: 'Ctrl+\\',
      description: 'Show or hide the sidebar',
      action: toggleSidebar,
      category: 'View'
    });
  }, [registerCommand, toggleTheme, toggleSidebar, setHelpOpen, setGlobalSearchOpen, setLayoutMode, notes, selectedNoteIndex, activeNote, openVault, createNote, renameNote, deleteNote]);
};

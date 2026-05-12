import { open } from '@tauri-apps/plugin-dialog';
import { readDir, readTextFile, writeTextFile, remove, rename } from '@tauri-apps/plugin-fs';
import { useNoteStore, Note } from '../store/noteStore';
import { useVaultStore } from '../store/vaultStore';
import { useEffect } from 'react';

export const useVault = () => {
  const { vaultPath, setVaultPath } = useVaultStore();
  const { notes, setNotes, setActiveNote, activeNote, closeNote } = useNoteStore();

  const createNote = async (name: string) => {
    if (!vaultPath) return;
    try {
      const fileName = name.endsWith('.md') ? name : `${name}.md`;
      const path = `${vaultPath}/${fileName}`;
      await writeTextFile(path, '# ' + name); // Default content
      const newNote: Note = { path, name, content: '# ' + name };
      setNotes([...notes, newNote]);
      setActiveNote(newNote);
    } catch (err) {
      console.error('Failed to create note:', err);
    }
  };

  const renameNote = async (note: Note, newName: string) => {
    try {
      const newFileName = newName.endsWith('.md') ? newName : `${newName}.md`;
      const newPath = note.path.substring(0, note.path.lastIndexOf('/')) + '/' + newFileName;
      await rename(note.path, newPath);
      
      const updatedNotes = notes.map(n => n.path === note.path 
        ? { ...n, path: newPath, name: newName } 
        : n);
      setNotes(updatedNotes);
      
      if (activeNote?.path === note.path) {
        setActiveNote({ ...activeNote, path: newPath, name: newName });
      }
    } catch (err) {
      console.error('Failed to rename note:', err);
    }
  };

  const deleteNote = async (note: Note) => {
    try {
      await remove(note.path);
      const updatedNotes = notes.filter(n => n.path !== note.path);
      setNotes(updatedNotes);
      closeNote(note.path);
    } catch (err) {
      console.error('Failed to delete note:', err);
    }
  };

  const saveNote = async (note: Note, content: string) => {
    try {
      await writeTextFile(note.path, content);
      // Update local state as well
      const updatedNotes = notes.map(n => n.path === note.path ? { ...n, content } : n);
      setNotes(updatedNotes);
    } catch (err) {
      console.error('Failed to save note:', err);
    }
  };

  const loadNotes = async (path: string) => {
    try {
      const entries = await readDir(path);
      const fetchedNotes: Note[] = [];

      for (const entry of entries) {
        if (entry.isFile && entry.name?.endsWith('.md')) {
          const notePath = `${path}/${entry.name}`;
          fetchedNotes.push({
            path: notePath,
            name: entry.name.replace('.md', ''),
            content: '', // Placeholder
          });
        }
      }

      // Eagerly load all contents for search
      const notesWithContent = await Promise.all(
        fetchedNotes.map(async (note) => {
          try {
            const content = await readTextFile(note.path);
            return { ...note, content };
          } catch (e) {
            return note;
          }
        })
      );

      setNotes(notesWithContent);
    } catch (err) {
      console.error('Failed to load notes:', err);
    }
  };

  const openVault = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: 'Select Note Vault',
    });

    if (selected && typeof selected === 'string') {
      setVaultPath(selected);
      await loadNotes(selected);
    }
  };

  const openNote = async (note: Note) => {
    try {
      const content = await readTextFile(note.path);
      setActiveNote({ ...note, content });
    } catch (err) {
      console.error('Failed to open note:', err);
    }
  };

  useEffect(() => {
    if (vaultPath) {
      loadNotes(vaultPath);
    }
  }, [vaultPath]);

  return { vaultPath, openVault, openNote, saveNote, deleteNote, createNote, renameNote };
};

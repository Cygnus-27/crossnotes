import { open } from '@tauri-apps/plugin-dialog';
import { readDir, readTextFile, writeTextFile, remove, rename } from '@tauri-apps/plugin-fs';
import { useNoteStore, Note } from '../store/noteStore';
import { useVaultStore } from '../store/vaultStore';
import { useEffect, useCallback } from 'react';

export const useVault = () => {
  const { vaultPath, setVaultPath } = useVaultStore();
  const { setNotes, setActiveNote, activeNote, notes } = useNoteStore();

  const createNote = useCallback(async (name: string) => {
    const currentVaultPath = useVaultStore.getState().vaultPath;
    if (!currentVaultPath) return;
    try {
      const fileName = name.endsWith('.md') ? name : `${name}.md`;
      const path = `${currentVaultPath}/${fileName}`;
      await writeTextFile(path, '# ' + name); // Default content
      const newNote: Note = { path, name, content: '# ' + name };
      
      const currentNotes = useNoteStore.getState().notes;
      setNotes([...currentNotes, newNote]);
      setActiveNote(newNote);
    } catch (err) {
      console.error('Failed to create note:', err);
    }
  }, [setNotes, setActiveNote]);

  const renameNote = useCallback(async (note: Note, newName: string) => {
    try {
      const newFileName = newName.endsWith('.md') ? newName : `${newName}.md`;
      const newPath = note.path.substring(0, note.path.lastIndexOf('/')) + '/' + newFileName;
      await rename(note.path, newPath);
      
      const currentNotes = useNoteStore.getState().notes;
      const updatedNotes = currentNotes.map(n => n.path === note.path 
        ? { ...n, path: newPath, name: newName } 
        : n);
      setNotes(updatedNotes);
      
      const currentActive = useNoteStore.getState().activeNote;
      if (currentActive?.path === note.path) {
        setActiveNote({ ...currentActive, path: newPath, name: newName });
      }
    } catch (err) {
      console.error('Failed to rename note:', err);
    }
  }, [setNotes, setActiveNote]);

  const deleteNote = useCallback(async (note: Note) => {
    try {
      await remove(note.path);
      const currentNotes = useNoteStore.getState().notes;
      const updatedNotes = currentNotes.filter(n => n.path !== note.path);
      setNotes(updatedNotes);
      useNoteStore.getState().closeNote(note.path);
    } catch (err) {
      console.error('Failed to delete note:', err);
    }
  }, [setNotes]);

  const saveNote = useCallback(async (note: Note, content: string) => {
    if (!note.path) return;
    try {
      await writeTextFile(note.path, content);
      const currentNotes = useNoteStore.getState().notes;
      const updatedNotes = currentNotes.map(n => n.path === note.path ? { ...n, content } : n);
      useNoteStore.setState({ notes: updatedNotes, isDirty: false });
    } catch (err) {
      console.error('Failed to save note:', err);
    }
  }, []);

  const loadNotes = useCallback(async (path: string): Promise<Note[]> => {
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
      return notesWithContent;
    } catch (err) {
      console.error('Failed to load notes:', err);
      return [];
    }
  }, [setNotes]);

  const openVault = useCallback(async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: 'Select Note Vault',
    });

    if (selected && typeof selected === 'string') {
      setVaultPath(selected);
      const loaded = await loadNotes(selected);
      if (loaded.length > 0) {
        const firstNote = loaded[0];
        const content = await readTextFile(firstNote.path);
        setActiveNote({ ...firstNote, content });
        useNoteStore.getState().setSelectedNoteIndex(0);
      }
    }
  }, [setVaultPath, loadNotes, setActiveNote]);

  const openNote = useCallback(async (note: Note) => {
    try {
      const content = await readTextFile(note.path);
      setActiveNote({ ...note, content });
    } catch (err) {
      console.error('Failed to open note:', err);
    }
  }, [setActiveNote]);

  useEffect(() => {
    if (vaultPath && notes.length === 0) {
      loadNotes(vaultPath).then(loaded => {
        if (loaded.length > 0 && !activeNote) {
          const firstNote = loaded[0];
          readTextFile(firstNote.path).then(content => {
             setActiveNote({ ...firstNote, content });
             useNoteStore.getState().setSelectedNoteIndex(0);
          });
        }
      });
    }
  }, [vaultPath, notes.length, setActiveNote, activeNote, loadNotes]);

  return { vaultPath, openVault, openNote, saveNote, deleteNote, createNote, renameNote };
};

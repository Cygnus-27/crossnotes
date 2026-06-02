import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  readDir,
  readTextFile,
  writeTextFile,
  remove,
  rename,
} from "@tauri-apps/plugin-fs";
import { useNoteStore, Note } from "../store/noteStore";
import { useVaultStore } from "../store/vaultStore";
import { useEffect, useCallback } from "react";

export const useVault = () => {
  const { vaultPath, setVaultPath } = useVaultStore();
  const { setNotes, setActiveNote, activeNote, notes, resetWorkspace } =
    useNoteStore();

  const createNote = useCallback(
    async (name: string) => {
      const currentVaultPath = useVaultStore.getState().vaultPath;
      if (!currentVaultPath) return;

      try {
        const fileName = name.endsWith(".md") ? name : `${name}.md`;
        const basePath = currentVaultPath.replace(/\\/g, "/");
        const path = `${basePath}/${fileName}`;
        await writeTextFile(path, "# " + name);
        const newNote: Note = { path, name, content: "# " + name };

        const currentNotes = useNoteStore.getState().notes;
        setNotes([...currentNotes, newNote]);
        setActiveNote(newNote);
      } catch (err) {
        console.error("Failed to create note:", err);
      }
    },
    [setNotes, setActiveNote],
  );

  const renameNote = useCallback(
    async (note: Note, newName: string) => {
      try {
        const newFileName = newName.endsWith(".md") ? newName : `${newName}.md`;
        const normalizedNotePath = note.path.replace(/\\/g, "/");
        const newPath = `${normalizedNotePath.substring(0, normalizedNotePath.lastIndexOf("/"))}/${newFileName}`;
        await rename(note.path, newPath);

        const currentNotes = useNoteStore.getState().notes;
        const updatedNotes = currentNotes.map((n) =>
          n.path === note.path ? { ...n, path: newPath, name: newName } : n,
        );
        setNotes(updatedNotes);

        const currentActive = useNoteStore.getState().activeNote;
        if (currentActive?.path === note.path) {
          setActiveNote({ ...currentActive, path: newPath, name: newName });
        }
      } catch (err) {
        console.error("Failed to rename note:", err);
      }
    },
    [setNotes, setActiveNote],
  );

  const deleteNote = useCallback(
    async (note: Note) => {
      try {
        await remove(note.path);
        const currentNotes = useNoteStore.getState().notes;
        const updatedNotes = currentNotes.filter((n) => n.path !== note.path);
        setNotes(updatedNotes);
        useNoteStore.getState().closeNote(note.path);
      } catch (err) {
        console.error("Failed to delete note:", err);
      }
    },
    [setNotes],
  );

  const saveNote = useCallback(async (note: Note, content: string) => {
    if (!note.path) return;

    try {
      await writeTextFile(note.path, content);
      const currentNotes = useNoteStore.getState().notes;
      const updatedNotes = currentNotes.map((n) =>
        n.path === note.path ? { ...n, content } : n,
      );
      useNoteStore.setState({ notes: updatedNotes, isDirty: false });
    } catch (err) {
      console.error("Failed to save note:", err);
      alert(
        "Failed to save note. Please check permissions or folder path.\n" + err,
      );
    }
  }, []);

  const loadNotes = useCallback(
    async (path: string): Promise<Note[]> => {
      try {
        const entries = await readDir(path);
        const fetchedNotes: Note[] = [];
        const normalizedBase = path.replace(/\\/g, "/");

        for (const entry of entries) {
          if (entry.isFile && entry.name?.endsWith(".md")) {
            fetchedNotes.push({
              path: `${normalizedBase}/${entry.name}`,
              name: entry.name.replace(".md", ""),
              content: "",
            });
          }
        }

        const notesWithContent = await Promise.all(
          fetchedNotes.map(async (note) => {
            try {
              const content = await readTextFile(note.path);
              return { ...note, content };
            } catch {
              return note;
            }
          }),
        );

        notesWithContent.sort((a, b) => a.name.localeCompare(b.name));
        setNotes(notesWithContent);
        return notesWithContent;
      } catch (err) {
        console.error("Failed to load notes:", err);
        resetWorkspace();
        return [];
      }
    },
    [setNotes, resetWorkspace],
  );

  const useDefaultVault = useCallback(async () => {
    try {
      const defaultVaultPath = await invoke<string>("get_default_vault");
      const normalizedVaultPath = defaultVaultPath.replace(/\\/g, "/");
      setVaultPath(normalizedVaultPath);
      const loaded = await loadNotes(normalizedVaultPath);
      if (loaded.length > 0) {
        setActiveNote(loaded[0]);
        useNoteStore.getState().setSelectedNoteIndex(0);
      } else {
        setActiveNote(null);
      }
    } catch (err) {
      console.error("Failed to open default vault:", err);
      alert("Failed to open app vault.\n" + err);
    }
  }, [loadNotes, setActiveNote, setVaultPath]);

  const createVault = useCallback(async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Choose where to create the new vault",
      });

      let parentDir: string | null = null;
      if (Array.isArray(selected)) {
        parentDir = selected[0] ?? null;
      } else if (typeof selected === "string") {
        parentDir = selected;
      }

      if (!parentDir) return;

      const vaultName = window.prompt(
        "Name your new vault",
        "CrossNotes Vault",
      );
      const trimmedName = vaultName?.trim();
      if (!trimmedName) return;

      const newVaultPath = await invoke<string>("create_vault", {
        parentPath: parentDir.replace(/\\/g, "/"),
        vaultName: trimmedName,
      });

      const normalizedVaultPath = newVaultPath.replace(/\\/g, "/");
      setVaultPath(normalizedVaultPath);
      setNotes([]);
      setActiveNote(null);
      useNoteStore.getState().setSelectedNoteIndex(0);
    } catch (err) {
      console.error("Failed to create vault:", err);
      alert("Failed to create vault.\n" + err);
    }
  }, [setActiveNote, setNotes, setVaultPath]);

  const openVault = useCallback(async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select Note Vault",
      });

      let vaultDir: string | null = null;
      if (Array.isArray(selected)) {
        vaultDir = selected[0] ?? null;
      } else if (typeof selected === "string") {
        vaultDir = selected;
      }

      if (!vaultDir) return;

      vaultDir = vaultDir.replace(/\\/g, "/");
      setVaultPath(vaultDir);

      const loaded = await loadNotes(vaultDir);
      if (loaded.length > 0) {
        const firstNote = loaded[0];
        setActiveNote(firstNote);
        useNoteStore.getState().setSelectedNoteIndex(0);
      } else {
        setActiveNote(null);
      }
    } catch (err) {
      console.error("Failed to open vault:", err);
    }
  }, [setVaultPath, loadNotes, setActiveNote]);

  const openNote = useCallback(
    async (note: Note) => {
      try {
        const content = await readTextFile(note.path);
        setActiveNote({ ...note, content });
      } catch (err) {
        console.error("Failed to open note:", err);
      }
    },
    [setActiveNote],
  );

  useEffect(() => {
    if (!vaultPath) {
      resetWorkspace();
      return;
    }

    if (notes.length === 0 && !activeNote) {
      loadNotes(vaultPath).then((loaded) => {
        if (loaded.length > 0) {
          setActiveNote(loaded[0]);
          useNoteStore.getState().setSelectedNoteIndex(0);
        }
      });
    }
  }, [
    vaultPath,
    notes.length,
    activeNote,
    loadNotes,
    setActiveNote,
    resetWorkspace,
  ]);

  return {
    vaultPath,
    openVault,
    createVault,
    useDefaultVault,
    loadNotes,
    openNote,
    saveNote,
    deleteNote,
    createNote,
    renameNote,
  };
};

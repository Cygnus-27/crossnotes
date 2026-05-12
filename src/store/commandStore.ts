import { create } from 'zustand';

export interface Command {
  id: string;
  name: string;
  shortcut?: string;
  description?: string;
  action: () => void;
  category?: string;
}

interface CommandState {
  isOpen: boolean;
  commands: Command[];
  setIsOpen: (open: boolean) => void;
  registerCommand: (command: Command) => void;
}

export const useCommandStore = create<CommandState>((set) => ({
  isOpen: false,
  commands: [],
  setIsOpen: (isOpen) => set({ isOpen }),
  registerCommand: (command) => set((state) => ({
    commands: [...state.commands.filter(c => c.id !== command.id), command]
  })),
}));

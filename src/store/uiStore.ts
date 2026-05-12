import { create } from 'zustand';

interface UIState {
  sidebarCollapsed: boolean;
  theme: 'dark' | 'light';
  quickOpenOpen: boolean;
  helpOpen: boolean;
  globalSearchOpen: boolean;
  focusedElement: 'sidebar' | 'editor';
  toggleSidebar: () => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  setQuickOpenOpen: (open: boolean) => void;
  setHelpOpen: (open: boolean) => void;
  setGlobalSearchOpen: (open: boolean) => void;
  setFocusedElement: (element: 'sidebar' | 'editor') => void;
  toggleTheme: () => void;
}

export const useUIStore = create<UIState>((set) => ({
  sidebarCollapsed: false,
  theme: 'dark',
  quickOpenOpen: false,
  helpOpen: false,
  globalSearchOpen: false,
  focusedElement: 'editor',
  toggleSidebar: () => set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),
  setSidebarCollapsed: (collapsed) => set({ sidebarCollapsed: collapsed }),
  setQuickOpenOpen: (open) => set({ quickOpenOpen: open }),
  setHelpOpen: (open) => set({ helpOpen: open }),
  setGlobalSearchOpen: (open) => set({ globalSearchOpen: open }),
  setFocusedElement: (focusedElement) => set({ focusedElement }),
  toggleTheme: () => set((state) => {
    const newTheme = state.theme === 'dark' ? 'light' : 'dark';
    document.documentElement.setAttribute('data-theme', newTheme);
    return { theme: newTheme };
  }),
}));

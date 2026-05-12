import React from 'react';
import { Sidebar } from './Sidebar';
import { StatusBar } from './StatusBar';

interface AppShellProps {
  children: React.ReactNode;
}

export const AppShell: React.FC<AppShellProps> = ({ children }) => {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh' }}>
      <div className="app-shell">
        <Sidebar />
        <main className="main-content">
          {children}
        </main>
      </div>
      <StatusBar />
    </div>
  );
};

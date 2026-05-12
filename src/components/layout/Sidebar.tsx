import React from 'react';
import { useUIStore } from '../../store/uiStore';
import { useVault } from '../../hooks/useVault';
import { NoteTree } from '../notes/NoteTree';

export const Sidebar: React.FC = () => {
  const collapsed = useUIStore((state) => state.sidebarCollapsed);
  const { vaultPath, openVault } = useVault();

  return (
    <aside className={`sidebar ${collapsed ? 'collapsed' : ''}`}>
      <div style={{ padding: '16px', height: '100%', display: 'flex', flexDirection: 'column', gap: '16px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <h2 style={{ fontSize: '11px', color: 'var(--text-secondary)', fontWeight: 'bold', textTransform: 'uppercase', letterSpacing: '0.05em' }}>
            {vaultPath ? 'Vault' : 'Welcome'}
          </h2>
          {vaultPath && (
             <button 
               onClick={openVault}
               style={{ 
                 background: 'none', 
                 border: 'none', 
                 color: 'var(--text-faint)', 
                 cursor: 'pointer',
                 fontSize: '10px'
               }}
             >
               Change
             </button>
          )}
        </div>

        <div style={{ flex: 1, overflowY: 'auto' }}>
          {vaultPath ? (
            <NoteTree />
          ) : (
            <div style={{ fontSize: '13px', color: 'var(--text-secondary)', display: 'flex', flexDirection: 'column', gap: '12px' }}>
              <p>To get started, open a folder containing your markdown notes.</p>
              <button 
                onClick={openVault}
                style={{
                  backgroundColor: 'var(--bg-elevated)',
                  border: '1px solid var(--border)',
                  color: 'var(--text-primary)',
                  padding: '8px',
                  borderRadius: '4px',
                  cursor: 'pointer',
                  fontSize: '12px',
                  fontWeight: 500
                }}
              >
                Open Vault
              </button>
            </div>
          )}
        </div>
        
        <div style={{ fontSize: '10px', color: 'var(--text-faint)' }}>
          <span className="keyboard-hint">Ctrl + \</span> to hide
        </div>
      </div>
    </aside>
  );
};

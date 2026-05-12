import React from 'react';
import { useNoteStore } from '../../store/noteStore';
import { useUIStore } from '../../store/uiStore';

export const Header: React.FC = () => {
  const activeNote = useNoteStore((state) => state.activeNote);
  const setHelpOpen = useUIStore((state) => state.setHelpOpen);

  return (
    <header style={{
      height: '48px',
      borderBottom: '1px solid var(--border)',
      display: 'flex',
      alignItems: 'center',
      padding: '0 24px',
      backgroundColor: 'var(--bg-base)',
      justifyContent: 'space-between',
      flexShrink: 0
    }}>
      <div style={{ fontSize: '14px', fontWeight: 500, color: 'var(--text-secondary)' }}>
        {activeNote ? activeNote.name : 'CrossNotes'}
      </div>
      
      <div style={{ display: 'flex', gap: '12px', alignItems: 'center' }}>
        <button
          onClick={() => setHelpOpen(true)}
          style={{
            background: 'none',
            border: 'none',
            color: 'var(--text-faint)',
            cursor: 'pointer',
            fontSize: '12px',
            display: 'flex',
            alignItems: 'center',
            gap: '6px',
            padding: '4px 8px',
            borderRadius: '4px',
          }}
          onMouseEnter={e => e.currentTarget.style.backgroundColor = 'var(--bg-elevated)'}
          onMouseLeave={e => e.currentTarget.style.backgroundColor = 'transparent'}
        >
          <span style={{ fontSize: '16px' }}>?</span> Help
          <span className="keyboard-hint" style={{ opacity: 0.8 }}>Ctrl + ?</span>
        </button>
      </div>
    </header>
  );
};

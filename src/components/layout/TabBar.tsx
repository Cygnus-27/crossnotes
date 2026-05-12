import React from 'react';
import { useNoteStore } from '../../store/noteStore';

export const TabBar: React.FC = () => {
  const { openNotes, activeNote, setActiveNote, closeNote, layoutMode } = useNoteStore();

  if (layoutMode !== 'tabs' || openNotes.length === 0) return null;

  return (
    <div style={{
      display: 'flex',
      backgroundColor: 'var(--bg-surface)',
      borderBottom: '1px solid var(--border)',
      overflowX: 'auto',
      scrollbarWidth: 'none',
      height: '36px',
      alignItems: 'stretch'
    }}>
      {openNotes.map((note) => (
        <div
          key={note.path}
          onClick={() => setActiveNote(note)}
          style={{
            padding: '0 16px',
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
            fontSize: '13px',
            cursor: 'pointer',
            backgroundColor: activeNote?.path === note.path ? 'var(--bg-base)' : 'transparent',
            borderRight: '1px solid var(--border)',
            color: activeNote?.path === note.path ? 'var(--accent)' : 'var(--text-secondary)',
            whiteSpace: 'nowrap',
            position: 'relative',
            borderTop: activeNote?.path === note.path ? '2px solid var(--accent)' : '2px solid transparent'
          }}
        >
          {note.name}
          <span
            onClick={(e) => {
              e.stopPropagation();
              closeNote(note.path);
            }}
            style={{
              fontSize: '14px',
              opacity: 0.5,
              padding: '2px 4px',
              borderRadius: '4px',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}
            onMouseEnter={e => e.currentTarget.style.backgroundColor = 'var(--bg-elevated)'}
            onMouseLeave={e => e.currentTarget.style.backgroundColor = 'transparent'}
          >
            ×
          </span>
        </div>
      ))}
    </div>
  );
};

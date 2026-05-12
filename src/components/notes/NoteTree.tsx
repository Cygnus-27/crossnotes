import React from 'react';
import { useNoteStore } from '../../store/noteStore';
import { useVault } from '../../hooks/useVault';
import { useUIStore } from '../../store/uiStore';

export const NoteTree: React.FC = () => {
  const { notes, activeNote, selectedNoteIndex, setSelectedNoteIndex } = useNoteStore();
  const { openNote } = useVault();
  const focusedElement = useUIStore(state => state.focusedElement);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
      {notes.map((note, index) => {
        const isActive = activeNote?.path === note.path;
        const isSelected = index === selectedNoteIndex && focusedElement === 'sidebar';
        
        return (
          <div
            key={note.path}
            onClick={() => {
              openNote(note);
              setSelectedNoteIndex(index);
            }}
            style={{
              padding: '6px 12px',
              borderRadius: '6px',
              fontSize: '13px',
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
              gap: '8px',
              backgroundColor: isSelected 
                ? 'var(--bg-elevated)' 
                : isActive 
                  ? 'rgba(156, 124, 91, 0.1)' 
                  : 'transparent',
              color: isSelected || isActive ? 'var(--accent)' : 'var(--text-secondary)',
              borderLeft: isSelected ? '2px solid var(--accent)' : '2px solid transparent',
              transition: 'background-color 0.1s ease',
            }}
            onMouseEnter={() => setSelectedNoteIndex(index)}
          >
            <span style={{ opacity: 0.5 }}>#</span>
            <span style={{ flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
              {note.name}
            </span>
          </div>
        );
      })}
    </div>
  );
};

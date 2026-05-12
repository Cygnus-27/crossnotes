import React, { useState, useEffect, useRef } from 'react';
import Fuse from 'fuse.js';
import { useNoteStore } from '../../store/noteStore';
import { useVault } from '../../hooks/useVault';

interface QuickOpenProps {
  isOpen: boolean;
  setIsOpen: (open: boolean) => void;
}

export const QuickOpen: React.FC<QuickOpenProps> = ({ isOpen, setIsOpen }) => {
  const { notes } = useNoteStore();
  const { openNote } = useVault();
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const fuse = new Fuse(notes, {
    keys: ['name'],
    threshold: 0.4,
  });

  const results = query ? fuse.search(query).map(r => r.item) : notes;

  useEffect(() => {
    if (isOpen) {
      setTimeout(() => inputRef.current?.focus(), 10);
      setSelectedIndex(0);
      setQuery('');
    }
  }, [isOpen]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!isOpen) return;

      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setSelectedIndex(prev => (prev + 1) % results.length);
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setSelectedIndex(prev => (prev - 1 + results.length) % results.length);
      } else if (e.key === 'Enter') {
        e.preventDefault();
        if (results[selectedIndex]) {
          openNote(results[selectedIndex]);
          setIsOpen(false);
        }
      } else if (e.key === 'Escape') {
        setIsOpen(false);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, results, selectedIndex, openNote, setIsOpen]);

  if (!isOpen) return null;

  return (
    <div style={{
      position: 'fixed',
      top: 0, left: 0, right: 0, bottom: 0,
      backgroundColor: 'rgba(0,0,0,0.5)',
      display: 'flex',
      justifyContent: 'center',
      paddingTop: '15vh',
      zIndex: 1000,
    }} onClick={() => setIsOpen(false)}>
      <div style={{
        width: '600px',
        maxWidth: '90vw',
        backgroundColor: 'var(--bg-surface)',
        borderRadius: '8px',
        border: '1px solid var(--border)',
        boxShadow: '0 10px 25px rgba(0,0,0,0.5)',
        display: 'flex',
        flexDirection: 'column',
        overflow: 'hidden',
        height: 'min-content',
        maxHeight: '60vh'
      }} onClick={e => e.stopPropagation()}>
        <div style={{ padding: '8px 16px', fontSize: '10px', color: 'var(--text-faint)', textTransform: 'uppercase', letterSpacing: '0.05em' }}>
          Quick Open
        </div>
        <input
          ref={inputRef}
          placeholder="Search notes..."
          value={query}
          onChange={e => setQuery(e.target.value)}
          style={{
            width: '100%',
            padding: '12px 16px',
            backgroundColor: 'transparent',
            border: 'none',
            borderBottom: '1px solid var(--border)',
            color: 'var(--text-primary)',
            fontSize: '15px',
            outline: 'none',
          }}
        />
        <div style={{ overflowY: 'auto' }}>
          {results.length === 0 ? (
            <div style={{ padding: '16px', color: 'var(--text-faint)', fontSize: '14px' }}>No notes found</div>
          ) : (
            results.map((note, i) => (
              <div
                key={note.path}
                onClick={() => { openNote(note); setIsOpen(false); }}
                style={{
                  padding: '10px 16px',
                  backgroundColor: i === selectedIndex ? 'var(--bg-elevated)' : 'transparent',
                  cursor: 'pointer',
                  borderLeft: i === selectedIndex ? '2px solid var(--accent)' : '2px solid transparent',
                  display: 'flex',
                  alignItems: 'center',
                  gap: '8px'
                }}
                onMouseEnter={() => setSelectedIndex(i)}
              >
                <span style={{ fontSize: '14px', color: 'var(--text-primary)' }}>{note.name}</span>
                <span style={{ fontSize: '11px', color: 'var(--text-faint)', marginLeft: 'auto' }}>.md</span>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
};

import React, { useState, useEffect, useRef } from 'react';
import Fuse from 'fuse.js';
import { useNoteStore } from '../../store/noteStore';
import { useVault } from '../../hooks/useVault';

interface GlobalSearchProps {
  isOpen: boolean;
  setIsOpen: (open: boolean) => void;
}

export const GlobalSearch: React.FC<GlobalSearchProps> = ({ isOpen, setIsOpen }) => {
  const { notes } = useNoteStore();
  const { openNote } = useVault();
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const fuse = new Fuse(notes, {
    keys: ['name', 'content'],
    threshold: 0.4,
    includeMatches: true,
    minMatchCharLength: 2,
  });

  const results = query ? fuse.search(query) : [];

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
          openNote(results[selectedIndex].item);
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
      paddingTop: '10vh',
      zIndex: 1000,
    }} onClick={() => setIsOpen(false)}>
      <div style={{
        width: '700px',
        maxWidth: '95vw',
        backgroundColor: 'var(--bg-surface)',
        borderRadius: '8px',
        border: '1px solid var(--border)',
        boxShadow: '0 10px 25px rgba(0,0,0,0.5)',
        display: 'flex',
        flexDirection: 'column',
        overflow: 'hidden',
        height: '60vh',
      }} onClick={e => e.stopPropagation()}>
        <div style={{ padding: '8px 16px', fontSize: '10px', color: 'var(--text-faint)', textTransform: 'uppercase', letterSpacing: '0.05em' }}>
          Global Search
        </div>
        <input
          ref={inputRef}
          placeholder="Search note contents..."
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
        <div style={{ overflowY: 'auto', flex: 1 }}>
          {query && results.length === 0 ? (
            <div style={{ padding: '32px', textAlign: 'center', color: 'var(--text-faint)', fontSize: '14px' }}>
              No matches found for "{query}"
            </div>
          ) : query === '' ? (
            <div style={{ padding: '32px', textAlign: 'center', color: 'var(--text-faint)', fontSize: '14px' }}>
              Type to search through all notes...
            </div>
          ) : (
            results.map((result, i) => {
              const { item } = result;
              // Find a snippet in the content
              const contentMatch = result.matches?.find(m => m.key === 'content');
              let snippet = '';
              if (contentMatch && contentMatch.value) {
                const matchIndex = contentMatch.indices[0][0];
                const start = Math.max(0, matchIndex - 40);
                const end = Math.min(contentMatch.value.length, matchIndex + 60);
                snippet = (start > 0 ? '...' : '') + contentMatch.value.substring(start, end) + (end < contentMatch.value.length ? '...' : '');
              } else {
                snippet = item.content.substring(0, 100) + '...';
              }

              return (
                <div
                  key={item.path}
                  onClick={() => { openNote(item); setIsOpen(false); }}
                  style={{
                    padding: '16px',
                    backgroundColor: i === selectedIndex ? 'var(--bg-elevated)' : 'transparent',
                    cursor: 'pointer',
                    borderLeft: i === selectedIndex ? '3px solid var(--accent)' : '3px solid transparent',
                    borderBottom: '1px solid var(--bg-base)'
                  }}
                  onMouseEnter={() => setSelectedIndex(i)}
                >
                  <div style={{ fontSize: '14px', color: 'var(--accent)', fontWeight: 600, marginBottom: '4px' }}>{item.name}</div>
                  <div style={{ 
                    fontSize: '12px', 
                    color: 'var(--text-secondary)', 
                    fontFamily: 'var(--font-mono)',
                    lineHeight: '1.4',
                    overflowWrap: 'anywhere'
                  }}>
                    {snippet}
                  </div>
                </div>
              )
            })
          )}
        </div>
        <div style={{ padding: '8px 16px', backgroundColor: 'var(--bg-base)', borderTop: '1px solid var(--border)', fontSize: '11px', color: 'var(--text-faint)', display: 'flex', gap: '16px' }}>
          <span><span className="keyboard-hint">Enter</span> to open</span>
          <span><span className="keyboard-hint">Esc</span> to close</span>
        </div>
      </div>
    </div>
  );
};

import React, { useState, useEffect, useRef } from 'react';
import Fuse from 'fuse.js';
import { useCommandStore } from '../../store/commandStore';

export const CommandPalette: React.FC = () => {
  const { isOpen, commands, setIsOpen } = useCommandStore();
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const fuse = new Fuse(commands, {
    keys: ['name', 'category', 'description'],
    threshold: 0.4,
  });

  const results = query ? fuse.search(query).map(r => r.item) : commands;

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
          results[selectedIndex].action();
          setIsOpen(false);
        }
      } else if (e.key === 'Escape') {
        setIsOpen(false);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, results, selectedIndex]);

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
        <input
          ref={inputRef}
          placeholder="Type a command..."
          value={query}
          onChange={e => setQuery(e.target.value)}
          style={{
            width: '100%',
            padding: '16px',
            backgroundColor: 'transparent',
            border: 'none',
            borderBottom: '1px solid var(--border)',
            color: 'var(--text-primary)',
            fontSize: '16px',
            outline: 'none',
          }}
        />
        <div style={{ overflowY: 'auto' }}>
          {results.length === 0 ? (
            <div style={{ padding: '16px', color: 'var(--text-faint)', fontSize: '14px' }}>No commands found</div>
          ) : (
            results.map((cmd, i) => (
              <div
                key={cmd.id}
                onClick={() => { cmd.action(); setIsOpen(false); }}
                style={{
                  padding: '12px 16px',
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center',
                  backgroundColor: i === selectedIndex ? 'var(--bg-elevated)' : 'transparent',
                  cursor: 'pointer',
                  borderLeft: i === selectedIndex ? '2px solid var(--accent)' : '2px solid transparent'
                }}
                onMouseEnter={() => setSelectedIndex(i)}
              >
                <div>
                  <div style={{ fontSize: '14px', color: 'var(--text-primary)' }}>{cmd.name}</div>
                  {cmd.description && (
                    <div style={{ fontSize: '11px', color: 'var(--text-faint)' }}>{cmd.description}</div>
                  )}
                </div>
                {cmd.shortcut && (
                  <span className="keyboard-hint">{cmd.shortcut}</span>
                )}
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
};

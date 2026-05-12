import React from 'react';
import { useCommandStore } from '../../store/commandStore';

export const ShortcutCheatSheet: React.FC<{ isOpen: boolean; onClose: () => void }> = ({ isOpen, onClose }) => {
  const commands = useCommandStore((state) => state.commands);

  if (!isOpen) return null;

  return (
    <div style={{
      position: 'fixed',
      top: 0, left: 0, right: 0, bottom: 0,
      backgroundColor: 'rgba(0,0,0,0.7)',
      display: 'flex',
      justifyContent: 'center',
      alignItems: 'center',
      zIndex: 2000,
      backdropFilter: 'blur(4px)'
    }} onClick={onClose}>
      <div style={{
        width: '700px',
        maxWidth: '90vw',
        backgroundColor: 'var(--bg-surface)',
        borderRadius: '12px',
        border: '1px solid var(--border)',
        boxShadow: '0 20px 50px rgba(0,0,0,0.5)',
        display: 'flex',
        flexDirection: 'column',
        maxHeight: '80vh',
        overflow: 'hidden'
      }} onClick={e => e.stopPropagation()}>
        <div style={{ padding: '24px', borderBottom: '1px solid var(--border)', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <div>
            <h2 style={{ fontSize: '20px', color: 'var(--text-primary)', marginBottom: '4px' }}>Keyboard Shortcuts</h2>
            <p style={{ fontSize: '13px', color: 'var(--text-secondary)' }}>The hands-never-leave-the-keyboard guide</p>
          </div>
          <button onClick={onClose} style={{ background: 'none', border: 'none', color: 'var(--text-faint)', cursor: 'pointer', fontSize: '24px' }}>×</button>
        </div>
        
        <div style={{ overflowY: 'auto', padding: '24px', display: 'flex', flexDirection: 'column', gap: '32px' }}>
          <section>
            <h3 style={{ fontSize: '12px', color: 'var(--accent)', textTransform: 'uppercase', letterSpacing: '0.1em', marginBottom: '16px' }}>Application</h3>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '16px 48px' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', fontSize: '14px' }}>
                <span style={{ color: 'var(--text-secondary)' }}>Global Search</span>
                <span className="keyboard-hint">Ctrl + Shift + F</span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', fontSize: '14px' }}>
                <span style={{ color: 'var(--text-secondary)' }}>Quick Open</span>
                <span className="keyboard-hint">Ctrl + P</span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', fontSize: '14px' }}>
                <span style={{ color: 'var(--text-secondary)' }}>Command Palette</span>
                <span className="keyboard-hint">Ctrl + K</span>
              </div>
              {commands.filter(c => !['global-search', 'global-search-open', 'quick-open', 'show-palette'].includes(c.id)).map(cmd => (
                <div key={cmd.id} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', fontSize: '14px' }}>
                  <span style={{ color: 'var(--text-secondary)' }}>{cmd.name}</span>
                  <span className="keyboard-hint">{cmd.shortcut || '---'}</span>
                </div>
              ))}
            </div>
          </section>

          <section>
            <h3 style={{ fontSize: '12px', color: 'var(--accent)', textTransform: 'uppercase', letterSpacing: '0.1em', marginBottom: '16px' }}>Vim Mode (Editor)</h3>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '16px 48px' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', fontSize: '14px' }}>
                <span style={{ color: 'var(--text-secondary)' }}>Normal Mode</span>
                <span className="keyboard-hint">Esc</span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', fontSize: '14px' }}>
                <span style={{ color: 'var(--text-secondary)' }}>Insert Mode</span>
                <span className="keyboard-hint">i</span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', fontSize: '14px' }}>
                <span style={{ color: 'var(--text-secondary)' }}>Delete Line</span>
                <span className="keyboard-hint">dd</span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', fontSize: '14px' }}>
                <span style={{ color: 'var(--text-secondary)' }}>Copy Line</span>
                <span className="keyboard-hint">yy</span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', fontSize: '14px' }}>
                <span style={{ color: 'var(--text-secondary)' }}>Paste</span>
                <span className="keyboard-hint">p</span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', fontSize: '14px' }}>
                <span style={{ color: 'var(--text-secondary)' }}>Undo</span>
                <span className="keyboard-hint">u</span>
              </div>
            </div>
          </section>
        </div>

        <div style={{ padding: '16px 24px', backgroundColor: 'var(--bg-elevated)', borderTop: '1px solid var(--border)', fontSize: '12px', color: 'var(--text-faint)' }}>
          Tip: Press <span className="keyboard-hint">Ctrl + K</span> to quickly search for any command.
        </div>
      </div>
    </div>
  );
};

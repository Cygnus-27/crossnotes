import React from 'react';
import { useUIStore } from '../../store/uiStore';
import { useEditorStore } from '../../store/editorStore';

export const StatusBar: React.FC = () => {
  const theme = useUIStore((state) => state.theme);
  const { line, col, wordCount } = useEditorStore();

  return (
    <div className="status-bar">
      <div style={{ display: 'flex', gap: '16px', alignItems: 'center' }}>
        <span style={{ fontWeight: 600, color: 'var(--accent)' }}>VIM</span>
        <span>UTF-8</span>
      </div>
      <div style={{ marginLeft: 'auto', display: 'flex', gap: '16px', alignItems: 'center' }}>
        <span>Ln {line}, Col {col}</span>
        <span>{wordCount} words</span>
        <span style={{ 
          textTransform: 'uppercase', 
          backgroundColor: 'var(--bg-elevated)', 
          padding: '2px 6px', 
          borderRadius: '3px',
          fontSize: '9px',
          fontWeight: 'bold'
        }}>{theme}</span>
      </div>
    </div>
  );
};

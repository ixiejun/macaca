import React from 'react';
import { createRoot } from 'react-dom/client';
import './styles.css';
import { WorkbenchApp } from './WorkbenchApp';

const root = document.getElementById('root');
if (!root) throw new Error('Workbench root element is missing');

createRoot(root).render(
  <React.StrictMode>
    <WorkbenchApp />
  </React.StrictMode>,
);

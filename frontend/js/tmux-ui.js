/**
 * tmux-ui.js — native DOM overlays for tmux integration:
 * command-prompt, confirm-before, output text overlays, prefix badges, mode alerts.
 */

export function showPrompt({ mount, prompt = ':', initial = '', onSubmit, onCancel }) {
  if (!mount) return { close: () => {} };

  // Remove existing prompt
  const existing = mount.querySelector('.tmux-overlay-prompt');
  if (existing) existing.remove();

  const bar = document.createElement('div');
  bar.className = 'tmux-overlay-prompt';

  const label = document.createElement('span');
  label.className = 'tmux-prompt-label';
  label.textContent = prompt.endsWith(' ') || prompt.endsWith(':') ? prompt : prompt + ': ';

  const input = document.createElement('input');
  input.type = 'text';
  input.className = 'tmux-prompt-input';
  input.value = initial;

  const errSpan = document.createElement('span');
  errSpan.className = 'tmux-prompt-error';

  bar.appendChild(label);
  bar.appendChild(input);
  bar.appendChild(errSpan);
  mount.appendChild(bar);

  let closed = false;
  function close() {
    if (closed) return;
    closed = true;
    bar.remove();
  }

  input.addEventListener('keydown', (e) => {
    e.stopPropagation();
    if (e.key === 'Enter') {
      e.preventDefault();
      const val = input.value;
      if (onSubmit) {
        onSubmit(val, {
          close,
          setError: (msg) => {
            errSpan.textContent = msg || '';
          }
        });
      } else {
        close();
      }
    } else if (e.key === 'Escape') {
      e.preventDefault();
      if (onCancel) onCancel();
      close();
    }
  });

  input.addEventListener('keyup', (e) => e.stopPropagation());
  input.addEventListener('keypress', (e) => e.stopPropagation());

  input.focus();
  input.setSelectionRange(input.value.length, input.value.length);
  // The triggering keydown may still be dispatching into xterm; reclaim focus
  // afterwards without touching the selection (the user may already be typing).
  setTimeout(() => {
    if (!closed && document.activeElement !== input) input.focus();
  }, 10);

  return {
    close,
    setError: (msg) => {
      errSpan.textContent = msg || '';
    }
  };
}

export function showConfirm({ mount, text, onYes, onNo }) {
  if (!mount) return { close: () => {} };

  const existing = mount.querySelector('.tmux-overlay-confirm');
  if (existing) existing.remove();

  const bar = document.createElement('div');
  bar.className = 'tmux-overlay-confirm';
  bar.tabIndex = -1;

  const label = document.createElement('span');
  label.className = 'tmux-confirm-text';
  label.textContent = text;

  bar.appendChild(label);
  mount.appendChild(bar);

  let closed = false;
  function close() {
    if (closed) return;
    closed = true;
    window.removeEventListener('keydown', handleKey, true);
    bar.remove();
  }

  function handleKey(e) {
    e.stopPropagation();
    e.preventDefault();
    if (e.key === 'y' || e.key === 'Y') {
      close();
      if (onYes) onYes();
    } else {
      close();
      if (onNo) onNo();
    }
  }

  window.addEventListener('keydown', handleKey, true);
  setTimeout(() => bar.focus(), 10);

  return { close };
}

export function showOverlayText({ mount, title = '', lines = [], onClose }) {
  if (!mount) return { close: () => {} };

  const existing = mount.querySelector('.tmux-overlay-text-container');
  if (existing) existing.remove();

  const container = document.createElement('div');
  container.className = 'tmux-overlay-text-container';

  const box = document.createElement('div');
  box.className = 'tmux-overlay-text-box';
  box.tabIndex = 0;

  if (title) {
    const header = document.createElement('div');
    header.className = 'tmux-overlay-text-header';
    header.textContent = title;
    box.appendChild(header);
  }

  const pre = document.createElement('pre');
  pre.className = 'tmux-overlay-text-body';
  pre.textContent = lines.join('\n');
  box.appendChild(pre);

  const closeBtn = document.createElement('button');
  closeBtn.className = 'tmux-overlay-text-close';
  closeBtn.textContent = '✕';
  box.appendChild(closeBtn);

  container.appendChild(box);
  mount.appendChild(container);

  let closed = false;
  function close() {
    if (closed) return;
    closed = true;
    container.remove();
    if (onClose) onClose();
  }

  closeBtn.addEventListener('click', close);
  container.addEventListener('click', (e) => {
    if (e.target === container) close();
  });

  box.addEventListener('keydown', (e) => {
    e.stopPropagation();
    if (e.key === 'Escape' || e.key === 'Enter' || e.key === 'q' || e.key === 'Q') {
      e.preventDefault();
      close();
    }
  });

  setTimeout(() => box.focus(), 10);

  return { close };
}

export function setPrefixBadge(paneEl, textOrNull) {
  if (!paneEl) return;
  let badge = paneEl.querySelector('.tmux-prefix-badge');
  if (!textOrNull) {
    if (badge) badge.remove();
    return;
  }
  if (!badge) {
    badge = document.createElement('div');
    badge.className = 'tmux-prefix-badge';
    paneEl.appendChild(badge);
  }
  badge.textContent = textOrNull;
}

export function showPaneBadge(paneEl, text) {
  if (!paneEl) return { remove: () => {} };
  let badge = paneEl.querySelector('.tmux-scrollback-badge');
  if (!badge) {
    badge = document.createElement('div');
    badge.className = 'tmux-scrollback-badge';
    paneEl.appendChild(badge);
  }
  badge.textContent = text || 'SCROLLBACK';
  return {
    remove: () => {
      if (badge && badge.parentNode) badge.remove();
    }
  };
}

export function showPaneModeOverlay({ paneEl, mode }) {
  if (!paneEl) return { close: () => {} };
  let overlay = paneEl.querySelector('.tmux-mode-overlay');
  if (!overlay) {
    overlay = document.createElement('div');
    overlay.className = 'tmux-mode-overlay';
    paneEl.appendChild(overlay);
  }
  overlay.textContent = `tmux ${mode} — press Esc to exit`;
  return {
    close: () => {
      if (overlay && overlay.parentNode) overlay.remove();
    }
  };
}

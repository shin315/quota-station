// The agents' and the services' logos: the drawing for the look in use, and
// a row with no known service left as it was.

import { cleanup, render } from '@testing-library/react';
import { afterEach, beforeAll, describe, expect, it } from 'vitest';
import css from '../styles/logos.css?raw';
import { AgentIcon, ServiceIcon } from './logos';

beforeAll(() => {
  const style = document.createElement('style');
  style.textContent = css;
  document.head.appendChild(style);
});

afterEach(cleanup);

/** The drawings of `agent`'s logo that show under `theme`, by their fill. */
function shown(agent: 'claude' | 'codex' | 'agy', theme: 'light' | 'dark'): string[] {
  const { container } = render(
    <div className="qs" data-theme={theme}>
      <AgentIcon agent={agent} />
    </div>,
  );
  return [...container.querySelectorAll('svg')].filter((s) => getComputedStyle(s).display !== 'none').map((s) => s.getAttribute('fill') ?? 'own');
}

describe('AgentIcon', () => {
  it('shows the dark Codex logo on the light look and the light one on the dark look', () => {
    expect(shown('codex', 'light')).toEqual(['#111']);
    cleanup();
    expect(shown('codex', 'dark')).toEqual(['#fff']);
  });

  it('shows the one drawing Claude and Antigravity have on both looks', () => {
    for (const theme of ['light', 'dark'] as const) {
      expect(shown('claude', theme)).toHaveLength(1);
      cleanup();
      expect(shown('agy', theme)).toHaveLength(1);
      cleanup();
    }
  });

  it('is hidden from screen readers and sized like the glyph it replaces', () => {
    const { container } = render(<AgentIcon agent="claude" />);
    const box = container.firstElementChild!;
    expect(box.getAttribute('aria-hidden')).toBe('true');
    expect(box.className).toContain('size-4');
    expect(container.textContent).toBe('');
  });
});

describe('ServiceIcon', () => {
  it("draws a known service's logo", () => {
    const { container } = render(<ServiceIcon service="github" />);
    expect(container.querySelectorAll('svg')).toHaveLength(2);
    expect(container.querySelector('.logo-light')).not.toBeNull();
    expect(container.querySelector('.logo-dark')).not.toBeNull();
  });

  it('draws nothing for a row with no known service', () => {
    const { container } = render(<ServiceIcon service={null} />);
    expect(container.innerHTML).toBe('');
  });
});

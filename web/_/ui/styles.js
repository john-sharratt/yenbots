// The styles the components share (§8.5), in tokens: a component's shadow
// root sees only its own styles, so the look of a button, a control and a
// field is written once here and adopted by each component that has one.
//
// Every state is styled — hover, focus, pressed, disabled, loading, error —
// and the catalogue shows the ones a pointer makes by setting `demo` on the
// host (`demo="hover"`, `demo="focus"`, `demo="pressed"`), so each state can
// be seen without being caused.

import { css } from "../vendor/lit-all.min.js";

/** Every component: its box, and the focus ring its controls take. */
export const base = css`
  :host {
    box-sizing: border-box;
    font-family: var(--yen-font-text);
    -webkit-tap-highlight-color: transparent;
  }
  *,
  *::before,
  *::after {
    box-sizing: inherit;
  }
  :host([hidden]) {
    display: none !important;
  }
  :focus-visible {
    outline: 2px solid var(--yen-focus);
    outline-offset: 2px;
  }
  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
`;

/** A button: `.btn` with `.primary`, `.secondary`, `.ghost` or `.danger`, and `.small`. */
export const button = css`
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--yen-space-2);
    min-height: var(--yen-control-height);
    min-width: var(--yen-tap);
    padding: 0 calc(var(--yen-space-4) * var(--yen-density));
    border: 1px solid transparent;
    border-radius: var(--yen-radius-button);
    font: inherit;
    font-weight: var(--yen-weight-semibold);
    font-size: var(--yen-text-md);
    line-height: 1.2;
    text-decoration: none;
    white-space: nowrap;
    cursor: pointer;
    user-select: none;
    transition:
      background-color var(--yen-duration-1) var(--yen-ease-out),
      border-color var(--yen-duration-1) var(--yen-ease-out),
      box-shadow var(--yen-duration-1) var(--yen-ease-out),
      transform var(--yen-duration-1) var(--yen-ease-out);
  }
  .btn.small {
    min-height: calc(var(--yen-control-height) * 0.8);
    padding: 0 var(--yen-space-3);
    font-size: var(--yen-text-sm);
  }
  .btn.wide {
    width: 100%;
  }
  .btn:active:not([disabled]),
  :host([demo~="pressed"]) .btn {
    transform: scale(0.97);
  }
  :host([demo~="focus"]) .btn {
    outline: 2px solid var(--yen-focus);
    outline-offset: 2px;
  }
  .btn[disabled],
  .btn[aria-disabled="true"] {
    cursor: not-allowed;
    opacity: 0.45;
  }
  .btn[aria-busy="true"] {
    cursor: progress;
  }

  .primary {
    background: var(--yen-accent);
    color: var(--yen-on-accent);
    box-shadow: var(--yen-shadow-1);
  }
  .primary:hover:not([disabled]),
  :host([demo~="hover"]) .primary {
    background: color-mix(in srgb, var(--yen-accent) 88%, var(--yen-ink));
    box-shadow: var(--yen-shadow-2);
  }
  .secondary {
    background: var(--yen-surface);
    color: var(--yen-ink);
    border-color: var(--yen-line-strong);
  }
  .secondary:hover:not([disabled]),
  :host([demo~="hover"]) .secondary {
    background: var(--yen-surface-2);
  }
  .ghost {
    background: transparent;
    color: var(--yen-accent);
  }
  .ghost:hover:not([disabled]),
  :host([demo~="hover"]) .ghost {
    background: var(--yen-accent-soft);
  }
  .danger {
    background: var(--yen-danger);
    color: var(--yen-on-danger);
  }
  .danger:hover:not([disabled]),
  :host([demo~="hover"]) .danger {
    background: color-mix(in srgb, var(--yen-danger) 88%, var(--yen-ink));
  }
`;

/** A spinner: `<span class="spinner" aria-hidden="true">`, in the ink around it. */
export const spinner = css`
  .spinner {
    display: inline-block;
    width: 1.1em;
    height: 1.1em;
    border-radius: 50%;
    border: 2px solid currentColor;
    border-right-color: transparent;
    animation: yen-spin 0.7s linear infinite;
    flex: none;
  }
  @keyframes yen-spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spinner {
      animation-duration: 2.4s;
    }
  }
`;

/** A control people type into or choose from: `.control` on an input, textarea or select. */
export const control = css`
  .control {
    display: block;
    width: 100%;
    min-height: var(--yen-control-height);
    padding: calc(var(--yen-space-2) * var(--yen-density)) var(--yen-space-3);
    border: 1px solid var(--yen-line-strong);
    border-radius: var(--yen-radius-2);
    background: var(--yen-surface);
    color: var(--yen-ink);
    font: inherit;
    font-size: var(--yen-text-md);
    line-height: var(--yen-leading);
    transition:
      border-color var(--yen-duration-1) var(--yen-ease-out),
      box-shadow var(--yen-duration-1) var(--yen-ease-out);
    appearance: none;
  }
  textarea.control {
    min-height: calc(var(--yen-control-height) * 2.6);
    resize: vertical;
  }
  .control::placeholder {
    color: var(--yen-ink-2);
    opacity: 1;
  }
  .control:hover:not(:disabled),
  :host([demo~="hover"]) .control {
    border-color: var(--yen-ink-2);
  }
  .control:focus,
  :host([demo~="focus"]) .control {
    outline: none;
    border-color: var(--yen-focus);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--yen-focus) 25%, transparent);
  }
  .control[aria-invalid="true"] {
    border-color: var(--yen-danger);
  }
  .control[aria-invalid="true"]:focus {
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--yen-danger) 25%, transparent);
  }
  .control:disabled {
    background: var(--yen-surface-2);
    color: var(--yen-ink-3);
    cursor: not-allowed;
  }
  select.control {
    padding-right: var(--yen-space-6);
    background-image: linear-gradient(45deg, transparent 50%, var(--yen-ink-2) 50%),
      linear-gradient(135deg, var(--yen-ink-2) 50%, transparent 50%);
    background-position:
      calc(100% - 1.2rem) 55%,
      calc(100% - 0.85rem) 55%;
    background-size:
      0.35rem 0.35rem,
      0.35rem 0.35rem;
    background-repeat: no-repeat;
  }
`;

/** A labelled field: `.field` holding `.label`, the control, `.hint` and `.error`. */
export const field = css`
  .field {
    display: grid;
    gap: var(--yen-space-1);
  }
  .label {
    font-size: var(--yen-text-sm);
    font-weight: var(--yen-weight-semibold);
    color: var(--yen-ink);
  }
  .required {
    color: var(--yen-danger);
  }
  .hint {
    margin: 0;
    font-size: var(--yen-text-sm);
    color: var(--yen-ink-2);
  }
  .error {
    margin: 0;
    font-size: var(--yen-text-sm);
    color: var(--yen-danger);
    display: flex;
    gap: var(--yen-space-1);
    align-items: flex-start;
  }
`;

/** A raised panel: `.panel`, the look of a card, a sheet's body and the shell's notices. */
export const panel = css`
  .panel {
    background: var(--yen-surface);
    border: 1px solid var(--yen-line);
    border-radius: var(--yen-radius-3);
    box-shadow: var(--yen-shadow-1);
    padding: var(--yen-space-4);
  }
`;

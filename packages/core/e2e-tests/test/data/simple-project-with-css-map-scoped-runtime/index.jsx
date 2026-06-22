/* eslint-disable react/no-unknown-property */
/* eslint-disable no-undef */

import React from 'react';
import {cssMap, cssMapScoped, keyframes} from '@compiled/react';
import {createRoot} from 'react-dom/client';

// cssMapScoped (runtime mode, extract:false) — sheets are hoisted as
// `const _N = "..."` declarations and injected at runtime by the CC/CS
// components inserted by the transform.
//
// Mirrors the -extracted fixture and covers the same edge cases:
//   - Nested descendant selectors (`.editor .panel`, `.editor .panel-title`).
//   - Two cssMapScoped variables sharing variant keys (must produce unique
//     class names).
//   - `@media` query inside a variant.
//   - `@keyframes` via `keyframes()` — global, not scoped.
//   - `css={[base, override1, override2]}` array form.

const pulse = keyframes({
  from: {opacity: 1},
  to: {opacity: 0.6},
});

const panelStyles = cssMapScoped({
  default: {
    '.editor .panel': {
      padding: '8px',
      backgroundColor: 'rgb(240, 240, 240)',
      borderRadius: '4px',
      // Autoprefix coverage: `user-select` should be emitted with the
      // `-webkit-user-select` vendor prefix when run through the postcss
      // pipeline. This exercises that cssMapScoped's non-atomicify plugin
      // shares the same autoprefixer pass as atomic mode.
      userSelect: 'none',
      // Parent-pseudo selectors: verify that `&:hover` and `&:focus` are
      // correctly flattened by the postcss-stack-based non_atomicify plugin
      // (this was a known bug in the SWC pipeline that the postcss path fixes).
      '&:hover': {
        cursor: 'pointer',
      },
      '&:focus': {
        outlineColor: 'rgb(0, 0, 255)',
      },
      // Deeper descendant nesting: `.editor .panel .panel-footer` — exercises
      // multi-level descendant scoping under a single non-atomic class.
      '.panel-footer': {
        marginTop: '6px',
        fontSize: '12px',
        color: 'rgb(100, 100, 100)',
      },
      // `&` referring to the OUTER descendant chain `.editor .panel`. This
      // pattern is common in editor / toolbar code:
      //   '.editor .panel': { '& .panel-icon': { ... } }
      // The expected flattened selector is
      //   `.cc-<hash> .editor .panel .panel-icon`.
      '& .panel-icon': {
        color: 'rgb(170, 170, 170)',
      },
    },
    // RIGHT-side ampersand for RTL/LTR i18n:
    //   `[dir="rtl"] &`: { '.editor blockquote': { ... } }
    // The variant class is attached on the RIGHT of `[dir="rtl"]`, so the
    // flattened selector is `[dir="rtl"] .cc-<hash> .editor blockquote`.
    // Only applies when the document direction is RTL.
    '[dir="rtl"] &': {
      '.editor blockquote': {
        paddingLeft: 0,
        paddingRight: 16,
        borderRightWidth: 2,
        borderRightStyle: 'solid',
        borderRightColor: 'rgb(0, 100, 200)',
      },
    },
    '.editor .panel-title': {
      fontWeight: 'bold',
      fontFamily: 'sans-serif',
    },
    '@media (min-width: 1px)': {
      '.editor .panel': {letterSpacing: '1px'},
    },
    // Nested at-rules: @supports containing @media — exercises the
    // recursive at-rule scoping in non_atomicify_rules.
    '@supports (display: grid)': {
      '.editor .panel': {
        display: 'grid',
        '@media (min-width: 1px)': {
          rowGap: '12px',
        },
      },
    },
  },
});

// Comma-separated selector lists at the variant level: `.editor .panel,
// .editor .panel-title` must scope EACH selector independently to
// `.cc-<hash> .editor .panel, .cc-<hash> .editor .panel-title`.
const panelDangerStyles = cssMapScoped({
  default: {
    // Comma-separated selectors sharing a declaration: opacity:0.95 must
    // apply to BOTH `.editor .panel` and `.editor .panel-title`.
    '.editor .panel, .editor .panel-title': {
      opacity: 0.95,
    },
    '.editor .panel': {
      backgroundColor: 'rgb(255, 224, 224)',
    },
    '.editor .panel-title': {
      color: 'red',
    },
  },
});

const panelDangerStylesNew = cssMapScoped({
  default: {
    '.editor .panel': {
      backgroundColor: 'rgb(255, 0, 0)',
      animationName: pulse,
      animationDuration: '2s',
      animationIterationCount: 'infinite',
    },
    '.editor .panel-title': {
      color: 'rgb(255, 255, 255)',
    },
  },
});

// Atomic cssMap — used together with cssMapScoped on a single element to
// verify that the two APIs compose correctly in `css={[scoped, atomic]}`.
// Each declaration produces a separate `_xxxxxxxx` class (atomic), distinct
// from the `cc-xxxxxx` classes produced by cssMapScoped.
const messageStyles = cssMap({
  info: {
    borderStyle: 'solid',
    borderColor: 'rgb(0, 128, 255)',
  },
});

const root = createRoot(document.getElementById('app'));

const page = (
  <>
    {/* Default panel — base styling only. Includes tabIndex so we can verify
        the :focus pseudo, and a .panel-footer for descendant nesting. */}
    <div data-testid="default-panel" css={panelStyles.default}>
      <div className="editor">
        <div className="panel" tabIndex={0} data-testid="default-panel-inner">
          <div className="panel-title">Default panel title</div>
          Default content
          {/* `& .panel-icon` inside `.editor .panel` should style this icon. */}
          <div className="panel-icon" data-testid="default-panel-icon">★</div>
          <div className="panel-footer" data-testid="default-panel-footer">Footer text</div>
        </div>
      </div>
    </div>

    <div
      data-testid="danger-panel"
      css={[panelStyles.default, panelDangerStyles.default]}>
      <div className="editor">
        <div className="panel">
          <div className="panel-title">Danger panel title</div>
          Danger content
        </div>
      </div>
    </div>

    <div
      data-testid="danger-new-panel"
      css={[
        panelStyles.default,
        panelDangerStyles.default,
        panelDangerStylesNew.default,
      ]}>
      <div className="editor">
        <div className="panel">
          <div className="panel-title">New danger panel title</div>
          New danger content
        </div>
      </div>
    </div>

    {/*
      RTL panel — exercises right-side `&` flattening:
        `[dir="rtl"] &`: { '.editor blockquote': { ... } }
      We set `dir="rtl"` on the wrapper so the `[dir="rtl"]` ancestor selector
      actually matches. The blockquote inside `.editor` should pick up the
      RTL-flipped padding + right border.
    */}
    {/*
      `dir="rtl"` is on the OUTER wrapper so that `[dir="rtl"]` is an
      ANCESTOR of the variant-class element (the inner `data-testid` div),
      matching the descendant selector `[dir="rtl"] .cc-<hash> .editor blockquote`.
    */}
    <div dir="rtl">
      <div data-testid="rtl-panel" css={panelStyles.default}>
        <div className="editor">
          <blockquote data-testid="rtl-blockquote">RTL quote</blockquote>
        </div>
      </div>
    </div>

    {/*
      Mixed panel — combines a `cssMapScoped` variant with an atomic `cssMap`
      variant on the same element via the `css={[scoped, atomic]}` array form.
      Verifies that the two APIs compose at runtime:
        - `panelStyles.default` (cssMapScoped) — `cc-<hash>` non-atomic class
          providing the descendant-scoped editor styling (background, padding).
        - `messageStyles.info` (cssMap) — atomic `_xxxxxxxx` classes providing
          per-declaration border + outline.
      Both styles must apply to the rendered DOM with no specificity conflicts.
    */}
    <div data-testid="mixed-panel" css={[panelStyles.default, messageStyles.info]}>
      <div className="editor">
        <div className="panel" data-testid="mixed-panel-inner">
          <div className="panel-title">Mixed panel title</div>
          Mixed content
        </div>
      </div>
    </div>
  </>
);

root.render(page);

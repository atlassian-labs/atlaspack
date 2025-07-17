import {useCallback, useMemo, memo} from 'react';
import path from 'path';

// @ts-expect-error TS7016
import {CodemirrorEditor} from '../codemirror';

import {
  EditorView,
  drawSelection,
  highlightActiveLine,
  highlightSpecialChars,
  keymap,
  lineNumbers,
  rectangularSelection,
  // @ts-expect-error TS7016
} from '@codemirror/view';
// @ts-expect-error TS7016
import {EditorState} from '@codemirror/state';
import {
  bracketMatching,
  defaultHighlightStyle,
  foldGutter,
  foldKeymap,
  indentOnInput,
  syntaxHighlighting,
  // @ts-expect-error TS7016
} from '@codemirror/language';
import {
  defaultKeymap,
  indentMore,
  indentLess,
  history,
  historyKeymap,
  // @ts-expect-error TS7016
} from '@codemirror/commands';
// @ts-expect-error TS7016
import {searchKeymap, highlightSelectionMatches} from '@codemirror/search';
import {
  autocompletion,
  completionKeymap,
  closeBrackets,
  closeBracketsKeymap,
  // @ts-expect-error TS7016
} from '@codemirror/autocomplete';
// @ts-expect-error TS7016
import {lintKeymap} from '@codemirror/lint';
// import {oneDark} from '@codemirror/theme-one-dark';

// @ts-expect-error TS7016
import {html} from '@codemirror/lang-html';
// @ts-expect-error TS7016
import {javascript} from '@codemirror/lang-javascript';
// @ts-expect-error TS7016
import {css} from '@codemirror/lang-css';
// @ts-expect-error TS7016
import {json} from '@codemirror/lang-json';

const theme = EditorView.theme({
  '.cm-content': {
    fontFamily: 'SFMono-Regular, Consolas, Liberation Mono, Menlo, monospace',
    fontSize: '14px',
  },
  '.cm-gutters': {
    fontFamily: 'SFMono-Regular, Consolas, Liberation Mono, Menlo, monospace',
    fontSize: '14px',
  },
});

const CONFIG_FILE = /^\.\w*rc$/;
const Editor: any = memo(function Editor({
  // @ts-expect-error TS2339
  filename,
  // @ts-expect-error TS2339
  readOnly,
  // @ts-expect-error TS2339
  content,
  // @ts-expect-error TS2339
  onChange,
  // @ts-expect-error TS2339
  diagnostics,
}) {
  const extension = path.extname(filename).slice(1);

  const extensions = useMemo(
    () =>
      [
        !readOnly && lineNumbers(),
        highlightSpecialChars(),
        history(),
        foldGutter(),
        drawSelection(),
        EditorState.allowMultipleSelections.of(true),
        indentOnInput(),
        syntaxHighlighting(defaultHighlightStyle),
        bracketMatching(),
        closeBrackets(),
        autocompletion(),
        rectangularSelection(),
        highlightActiveLine(),
        highlightSelectionMatches(),
        theme,
        // oneDark,
        keymap.of([
          ...closeBracketsKeymap,
          ...defaultKeymap,
          ...searchKeymap,
          ...historyKeymap,
          ...foldKeymap,
          ...completionKeymap,
          ...lintKeymap,
          {
            key: 'Tab',
            preventDefault: true,
            run: indentMore,
          },
          {
            key: 'Shift-Tab',
            preventDefault: true,
            run: indentLess,
          },
        ]),
        extension === 'json' || CONFIG_FILE.test(path.basename(filename))
          ? json()
          : extension.startsWith('js') || extension.startsWith('ts')
            ? javascript({
                jsx: extension.endsWith('x'),
                typescript: extension.includes('ts'),
              })
            : extension === 'html'
              ? html()
              : extension === 'css'
                ? css()
                : null,
      ].filter(Boolean),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [extension],
  );

  return (
    // @ts-expect-error TS17004
    <CodemirrorEditor
      value={content}
      onChange={onChange}
      extensions={extensions}
      readOnly={readOnly}
      diagnostics={diagnostics}
    />
  );
});

function EditorWrapper({
  dispatch,
  name,
  value,
  readOnly,
  diagnostics,
}: any): any {
  let onChange = useCallback(
    (value) => dispatch({type: 'view.setValue', name, value}),
    [dispatch, name],
  );

  return (
    // @ts-expect-error TS17004
    <Editor
      filename={name}
      content={value}
      onChange={onChange}
      readOnly={readOnly}
      diagnostics={diagnostics}
    />
  );
}

export {EditorWrapper as Editor};

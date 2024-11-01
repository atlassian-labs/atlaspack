// This script merges the default settings for VSCode with any existing settings
const fs = require('fs');
const path = require('path');

const DEFAULT_SETTINGS = {
  // Fix for broken NAPI macros
  'rust-analyzer.procMacro.ignored': {'napi-derive': ['napi']},
  // Disable type checking for Flow
  'javascript.validate.enable': false,
  // Misc
  'rust-analyzer.cargo.features': 'all',
  '[rust]': {
    'editor.defaultFormatter': 'rust-lang.rust-analyzer',
    'editor.formatOnSave': true,
    'editor.formatOnType': true,
  },
};

const paths = {
  '.vscode': path.join(__dirname, '..', '.vscode'),
  '.vscode/settings.json': path.join(
    __dirname,
    '..',
    '.vscode',
    'settings.json',
  ),
};

let currentSettings = {};

if (
  fs.existsSync(paths['.vscode']) &&
  fs.existsSync(paths['.vscode/settings.json'])
) {
  currentSettings = JSON.parse(
    fs.readFileSync(paths['.vscode/settings.json'], 'utf8'),
  );
} else {
  fs.mkdirSync(paths['.vscode'], {recursive: true});
}

const update = {
  ...currentSettings,
  ...DEFAULT_SETTINGS,
  '[rust]': {
    ...(currentSettings['[rust]'] || {}),
    ...DEFAULT_SETTINGS['[rust]'],
  },
};

fs.writeFileSync(
  paths['.vscode/settings.json'],
  JSON.stringify(update, null, 2),
  'utf8',
);

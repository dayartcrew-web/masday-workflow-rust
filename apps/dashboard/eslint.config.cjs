// Flat config for ESLint v9+.
// Next.js v16 ships `eslint-config-next` as a flat-config array export.

/** @type {import('eslint').Linter.FlatConfig[]} */
module.exports = [
  ...require('eslint-config-next'),
  {
    ignores: ['node_modules/**'],
  },
];

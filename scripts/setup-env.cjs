/**
 * setup-env.cjs — Postinstall script that creates a .env from .env.example
 * if one does not already exist. Runs automatically via `npm install`.
 *
 * Service-scoped: the .env lives alongside the project (not machine-global).
 */
'use strict';

const fs = require('fs');
const path = require('path');

const root = path.resolve(__dirname, '..');
const envPath = path.join(root, '.env');
const examplePath = path.join(root, '.env.example');

if (fs.existsSync(envPath)) {
  // .env already exists — do not overwrite
  return;
}

if (!fs.existsSync(examplePath)) {
  // .env.example missing — nothing to scaffold
  return;
}

fs.copyFileSync(examplePath, envPath);
console.log('[agenticpdf] Created .env from .env.example — edit it with your OTEL credentials.');

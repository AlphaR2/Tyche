/**
 * restore-pkg-names.cjs
 *
 * Codama's renderVisitor overwrites the package.json in each generated client
 * directory with `"name": "js-client"` on every `just generate` run.
 *
 * This script re-applies the correct workspace package names after generation
 * so that the npm workspace resolution stays intact.
 *
 * Run via: node scripts/restore-pkg-names.cjs
 */

'use strict';

const fs = require('fs');
const path = require('path');

const PACKAGES = [
  {
    path: 'clients/js/src/generated/tyche-core/package.json',
    name: 'tyche-generated-core',
    description: 'Auto-generated Codama client for tyche-core. Do not edit manually.',
  },
  {
    path: 'clients/js/src/generated/tyche-escrow/package.json',
    name: 'tyche-generated-escrow',
    description: 'Auto-generated Codama client for tyche-escrow. Do not edit manually.',
  },
  {
    path: 'clients/js/src/generated/tyche-auction/package.json',
    name: 'tyche-generated-auction',
    description: 'Auto-generated Codama client for tyche-auction. Do not edit manually.',
  },
  {
    path: 'clients/js/src/generated/tyche-voter-weight-plugin/package.json',
    name: 'tyche-generated-voter-weight-plugin',
    description: 'Auto-generated Codama client for tyche-voter-weight-plugin. Do not edit manually.',
  },
];

for (const { path: pkgPath, name, description } of PACKAGES) {
  const fullPath = path.resolve(process.cwd(), pkgPath);
  const pkg = JSON.parse(fs.readFileSync(fullPath, 'utf8'));

  pkg.name = name;
  pkg.version = pkg.version ?? '0.1.0';
  pkg.description = description;

  fs.writeFileSync(fullPath, JSON.stringify(pkg, null, 2) + '\n');
  console.log(`[restore-pkg-names] ${name}`);
}

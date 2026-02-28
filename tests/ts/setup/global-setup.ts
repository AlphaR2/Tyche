/**
 * Vitest setup file — runs in every test worker before any test executes.
 *
 * Listed under `setupFiles` in vitest.config.ts so that env vars are available
 * in the same process/worker that runs the tests (unlike `globalSetup` which
 * runs in a separate process and cannot share env vars reliably).
 *
 * Loads .env.test so all test modules can read `process.env.*` without
 * calling `dotenv.config()` themselves.
 */

import { config } from 'dotenv';
import { resolve } from 'path';
import { fileURLToPath } from 'url';

const __dirname = fileURLToPath(new URL('.', import.meta.url));
const result = config({ path: resolve(__dirname, '../.env.test') });

if (result.error) {
  const code = (result.error as NodeJS.ErrnoException).code;
  if (code !== 'ENOENT') {
    // A missing .env.test is acceptable (CI injects vars directly).
    // Any other parse error — e.g. malformed file — is fatal.
    throw result.error;
  }
}

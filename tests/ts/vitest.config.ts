import { defineConfig } from 'vitest/config';
import { resolve } from 'path';
import { fileURLToPath } from 'url';

const __dirname = fileURLToPath(new URL('.', import.meta.url));

export default defineConfig({
  resolve: {
    alias: {
      // SDK source (avoids needing a built dist during development)
      'tyche-sdk': resolve(__dirname, '../../packages/sdk/src/index.ts'),
      // Codama-generated instruction clients
      'tyche-generated-core':    resolve(__dirname, '../../clients/js/src/generated/tyche-core/src/generated'),
      'tyche-generated-escrow':  resolve(__dirname, '../../clients/js/src/generated/tyche-escrow/src/generated'),
      'tyche-generated-auction': resolve(__dirname, '../../clients/js/src/generated/tyche-auction/src/generated'),
    },
  },
  test: {
    // All tests run against devnet — allow generous timeouts
    testTimeout:  60_000,
    hookTimeout:  30_000,
    // Run files sequentially to avoid devnet rate-limit issues
    pool:        'forks',
    poolOptions: { forks: { singleFork: true } },
    // setupFiles runs in each worker (same process as tests), so dotenv.config()
    // correctly populates process.env for env.ts and all test files.
    setupFiles:  ['./setup/global-setup.ts'],
    // Test file pattern
    include:     ['**/*.test.ts'],
    exclude:     ['node_modules/**'],
    reporter:    'verbose',
  },
});

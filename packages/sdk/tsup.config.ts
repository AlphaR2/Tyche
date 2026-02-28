import { defineConfig } from 'tsup';
import path from 'path';

// Resolve generated packages by physical path so the build works without
// npm workspace symlinks (required for Windows + WSL environments).
const ROOT = path.resolve(__dirname, '../..');
const GENERATED = path.join(ROOT, 'clients/js/src/generated');

// Entry and outDir use absolute paths so this config works when invoked
// from the repo root (npx tsup --config packages/sdk/tsup.config.ts).
const SDK_DIR = __dirname;

export default defineConfig({
  entry: { index: path.join(SDK_DIR, 'src/index.ts') },
  outDir: path.join(SDK_DIR, 'dist'),
  format: ['esm', 'cjs'],
  dts: true,
  splitting: false,
  sourcemap: true,
  clean: true,
  treeshake: true,
  // Peer deps — consumers install these; NOT bundled.
  external: ['@solana/kit', '@solana/program-client-core'],
  // Map generated workspace package names → physical source directories.
  // The generated packages are bundled inline into tyche-sdk.
  esbuildOptions(options) {
    options.alias = {
      'tyche-generated-core':    path.join(GENERATED, 'tyche-core/src/generated'),
      'tyche-generated-escrow':  path.join(GENERATED, 'tyche-escrow/src/generated'),
      'tyche-generated-auction': path.join(GENERATED, 'tyche-auction/src/generated'),
    };
  },
  outExtension({ format }) {
    return { js: format === 'esm' ? '.js' : '.cjs' };
  },
});

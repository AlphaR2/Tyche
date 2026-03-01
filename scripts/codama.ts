/**
 * Generates TypeScript clients from fixed IDL files using Codama.
 *
 * Discriminators are NOT applied via the Codama visitor API
 * (updateInstructionsVisitor fails in strict-mode Node ≥22 due to a
 * caller/callee access error inside the visitor itself).
 *
 * Instead, `scripts/patch-discriminators.cjs` post-processes the generated
 * files and replaces the 1-byte ordinal discriminators with the correct 8-byte
 * SHA256 values.  That script is invoked separately (see `npm run generate`
 * and `just generate`).
 */

import { rootNodeFromAnchor } from "@codama/nodes-from-anchor";
import { renderVisitor as renderJsVisitor } from "@codama/renderers-js";
import { visit } from "codama";
import { readFileSync } from "fs";

const programs = [
  {
    idl: "clients/idls/tyche_core.json",
    name: "tyche-core",
    out: "clients/js/src/generated/tyche-core",
  },
  {
    idl: "clients/idls/tyche_escrow.json",
    name: "tyche-escrow",
    out: "clients/js/src/generated/tyche-escrow",
  },
  {
    idl: "clients/idls/tyche_auction.json",
    name: "tyche-auction",
    out: "clients/js/src/generated/tyche-auction",
  },
  {
    idl: "clients/idls/tyche_voter_weight_plugin.json",
    name: "tyche-voter-weight-plugin",
    out: "clients/js/src/generated/tyche-voter-weight-plugin",
  },
];

async function generateClients() {
  for (const { idl, name, out } of programs) {
    const raw = JSON.parse(readFileSync(idl, "utf-8"));
    const node = rootNodeFromAnchor(raw);
    await visit(node, renderJsVisitor(out) as any);
    console.log(`generated ${name} → ${out}`);
  }
}

generateClients().catch((e) => {
  console.error("generation failed:", e);
  process.exit(1);
});

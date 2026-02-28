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
];

async function generateClients() {
  for (const { idl, name, out } of programs) {
    const raw = JSON.parse(readFileSync(idl, "utf-8"));
    const node = rootNodeFromAnchor(raw);
    await visit(node, renderJsVisitor(out));
    console.log(`generated ${name} → ${out}`);
  }
}

generateClients().catch((e) => {
  console.error("generation failed:", e);
  process.exit(1);
});

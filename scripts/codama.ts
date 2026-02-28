import { rootNodeFromAnchor } from "@codama/nodes-from-anchor";
import { renderVisitor as renderJsVisitor } from "@codama/renderers-js";
import {
  visit,
  updateInstructionsVisitor,
  constantDiscriminatorNode,
  constantValueNode,
  bytesTypeNode,
  bytesValueNode,
} from "codama";
import { readFileSync } from "fs";

const CORE_DISCRIMINANTS: Record<string, number[]> = {
    CreateCompetition:        [110, 212, 234, 212, 118, 128, 158, 244],
    ActivateCompetition:      [153, 105, 130,  88, 198, 208,  30, 118],
    ExtendCompetition:        [  9,   0,  18, 247, 115,  18, 176, 115],
    CloseCompetition:         [ 49, 166, 127,  67,  43, 108, 132,  96],
    SettleCompetition:        [ 83, 121,   9, 141, 170, 133, 230, 151],
    CancelCompetition:        [ 62,   4, 198,  98, 200,  41, 255,  72],
    RegisterBid:              [ 26, 173,  93,  67, 171, 107, 118, 212],
    InitializeProtocolConfig: [ 28,  50,  43, 233, 244,  98, 123, 118],
    UpdateProtocolConfig:     [197,  97, 123,  54, 221, 168,  11, 135],
    UpdateCrankAuthority:     [108,  25, 148, 152,  87,  11, 210,  84],
};

const ESCROW_DISCRIMINANTS: Record<string, number[]> = {
    Deposit: [242,  35, 198, 137,  82, 225, 242, 182],
    Release: [253, 249,  15, 206,  28, 127, 193, 241],
    Refund:  [  2,  96, 183, 251,  63, 208,  46,  46],
};

const AUCTION_DISCRIMINANTS: Record<string, number[]> = {
    CreateAuction:   [234,   6, 201, 246,  47, 219, 176, 107],
    ActivateAuction: [212,  24, 210,   7, 183, 147,  66, 109],
    PlaceBid:        [238,  77, 148,  91, 200, 151,  92, 146],
    FinalizeAuction: [220, 209, 175, 193,  57, 132, 241, 168],
    CancelAuction:   [156,  43, 197, 110, 218, 105, 143, 182],
    CloseBidRecord:  [191, 178, 243, 199,  31, 166, 172, 200],
};

const VOTER_WEIGHT_DISCRIMINANTS: Record<string, number[]> = {
    CreateRegistrar:             [132, 235, 36, 49, 139, 66, 202, 69],
    CreateVoterWeightRecord:     [184, 249, 133, 178, 88, 152, 250, 186],
    UpdateVoterWeightRecord:     [ 45, 185,  3, 36, 109, 190, 115, 169],
    UpdateMaxVoterWeightRecord:  [103, 175, 201, 251,  2,  9, 251, 179],
};

const ALL_DISCRIMINANTS = {
    ...CORE_DISCRIMINANTS,
    ...ESCROW_DISCRIMINANTS,
    ...AUCTION_DISCRIMINANTS,
    ...VOTER_WEIGHT_DISCRIMINANTS,
};

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
    let node = rootNodeFromAnchor(raw);

    // Build the instruction mapping for updateInstructionsVisitor
    const instructionUpdates: Record<string, (node: any) => any> = {};
    for (const [ixName, disc] of Object.entries(ALL_DISCRIMINANTS)) {
        instructionUpdates[ixName] = (node) => ({
            ...node,
            discriminator: constantDiscriminatorNode(
                constantValueNode(
                    bytesTypeNode(),
                    bytesValueNode(new Uint8Array(disc))
                )
            )
        });
    }

    // Apply explicit discriminators to instruction nodes
    node = visit(node, updateInstructionsVisitor(instructionUpdates as any) as any) as any;

    await visit(node, renderJsVisitor(out) as any);
    console.log(`generated ${name} → ${out}`);
  }
}

generateClients().catch((e) => {
  console.error("generation failed:", e);
  process.exit(1);
});

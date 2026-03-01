/**
 * patch-discriminators.cjs
 *
 * Post-processes Codama-generated TypeScript instruction files to inject the
 * correct 8-byte Anchor-style discriminators.
 *
 * Codama generates 1-byte ordinal discriminators (e.g. `new Uint8Array([0])`).
 * This script replaces them with the real SHA256-derived values used by the
 * on-chain programs.  It also fixes the encoder/decoder byte-size from 1 → 8.
 *
 * Run: node scripts/patch-discriminators.cjs
 * (Called automatically by `npm run generate` and `just generate`.)
 */

"use strict";
const { readFileSync, writeFileSync, existsSync } = require("fs");
const { join } = require("path");

// ── Discriminant tables ────────────────────────────────────────────────────────

const CORE_DISCRIMINANTS = {
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

const ESCROW_DISCRIMINANTS = {
    Deposit: [242,  35, 198, 137,  82, 225, 242, 182],
    Release: [253, 249,  15, 206,  28, 127, 193, 241],
    Refund:  [  2,  96, 183, 251,  63, 208,  46,  46],
};

const AUCTION_DISCRIMINANTS = {
    CreateAuction:   [234,   6, 201, 246,  47, 219, 176, 107],
    ActivateAuction: [212,  24, 210,   7, 183, 147,  66, 109],
    PlaceBid:        [238,  77, 148,  91, 200, 151,  92, 146],
    FinalizeAuction: [220, 209, 175, 193,  57, 132, 241, 168],
    CancelAuction:   [156,  43, 197, 110, 218, 105, 143, 182],
    CloseBidRecord:  [191, 178, 243, 199,  31, 166, 172, 200],
};

const VOTER_WEIGHT_DISCRIMINANTS = {
    CreateRegistrar:            [132, 235,  36,  49, 139,  66, 202,  69],
    CreateVoterWeightRecord:    [184, 249, 133, 178,  88, 152, 250, 186],
    UpdateVoterWeightRecord:    [ 45, 185,   3,  36, 109, 190, 115, 169],
    UpdateMaxVoterWeightRecord: [103, 175, 201, 251,   2,   9, 251, 179],
};

// ── Utilities ──────────────────────────────────────────────────────────────────

/** "CreateCompetition" → "CREATE_COMPETITION" */
function toScreamingSnake(name) {
    return name.replace(/([A-Z])/g, "_$1").replace(/^_/, "").toUpperCase();
}

/** "CreateCompetition" → "createCompetition" */
function toCamel(name) {
    return name.charAt(0).toLowerCase() + name.slice(1);
}

/**
 * Patch one instruction file.
 *
 * Patterns replaced (all scoped to the discriminator — other byte-sized
 * fields like padding use different identifiers and are not affected):
 *
 *   export const FOO_DISCRIMINATOR = new Uint8Array([<any>]);
 *   →  export const FOO_DISCRIMINATOR = new Uint8Array([b0, …, b7]);
 *
 *   fixEncoderSize(getBytesEncoder(), 1).encode(   (discriminatorBytes fn)
 *   →  fixEncoderSize(getBytesEncoder(), 8).encode(
 *
 *   ["discriminator", fixEncoderSize(getBytesEncoder(), 1)]   (struct codec)
 *   →  ["discriminator", fixEncoderSize(getBytesEncoder(), 8)]
 *
 *   ["discriminator", fixDecoderSize(getBytesDecoder(), 1)]
 *   →  ["discriminator", fixDecoderSize(getBytesDecoder(), 8)]
 */
function patchFile(filePath, constName, disc) {
    if (!existsSync(filePath)) {
        console.warn(`  WARN: not found — ${filePath}`);
        return;
    }

    let src = readFileSync(filePath, "utf-8");
    const before = src;

    const newArray = `new Uint8Array([${disc.join(", ")}])`;

    // 1. Discriminator constant value.
    src = src.replace(
        new RegExp(`(export const ${constName} = )new Uint8Array\\([^)]+\\);`),
        `$1${newArray};`,
    );

    // 2. Encoder size in getXxxDiscriminatorBytes().
    src = src.replace(
        /fixEncoderSize\(getBytesEncoder\(\), 1\)\.encode\(/g,
        "fixEncoderSize(getBytesEncoder(), 8).encode(",
    );

    // 3. Encoder size in struct codec (keyed by field name "discriminator").
    src = src.replace(
        /\["discriminator", fixEncoderSize\(getBytesEncoder\(\), 1\)\]/g,
        '["discriminator", fixEncoderSize(getBytesEncoder(), 8)]',
    );

    // 4. Decoder size in struct codec.
    src = src.replace(
        /\["discriminator", fixDecoderSize\(getBytesDecoder\(\), 1\)\]/g,
        '["discriminator", fixDecoderSize(getBytesDecoder(), 8)]',
    );

    if (src === before) {
        console.warn(`  WARN: no patterns matched in ${filePath} — already patched or unexpected format`);
        return;
    }

    writeFileSync(filePath, src);
    console.log(`  patched ${constName} → [${disc.join(", ")}]`);
}

/**
 * Patch all instruction files for one program.
 *
 * @param {string} outDir  — the `out` dir passed to renderJsVisitor,
 *                           e.g. "clients/js/src/generated/tyche-core"
 * @param {Record<string, number[]>} discriminants
 */
function patchProgram(outDir, discriminants) {
    // Codama renders into {outDir}/src/generated/instructions/{camelName}.ts
    const ixDir = join(outDir, "src", "generated", "instructions");

    for (const [name, disc] of Object.entries(discriminants)) {
        const fileName = toCamel(name) + ".ts";
        const constName = toScreamingSnake(name) + "_DISCRIMINATOR";
        patchFile(join(ixDir, fileName), constName, disc);
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

patchProgram("clients/js/src/generated/tyche-core",               CORE_DISCRIMINANTS);
patchProgram("clients/js/src/generated/tyche-escrow",             ESCROW_DISCRIMINANTS);
patchProgram("clients/js/src/generated/tyche-auction",            AUCTION_DISCRIMINANTS);
patchProgram("clients/js/src/generated/tyche-voter-weight-plugin", VOTER_WEIGHT_DISCRIMINANTS);

console.log("discriminators patched");

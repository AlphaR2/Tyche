import { readFileSync, writeFileSync } from "fs";

// ─── actual discriminator values from compute_discriminators ─────────────────
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

// ─── fix Address type ─────────────────────────────────────────────────────────
// Shank emits { "defined": "Address" } which Codama cannot resolve.
// Replace with the canonical byte-array representation { "array": ["u8", 32] }.
function fixAddressTypes(obj: any): any {
    if (obj === null || typeof obj !== "object") return obj;
    if (obj.defined === "Address") {
        return { array: ["u8", 32] };
    }
    for (const key of Object.keys(obj)) {
        obj[key] = fixAddressTypes(obj[key]);
    }
    return obj;
}

// ─── fix discriminants ────────────────────────────────────────────────────────
// Shank emits { "discriminant": { "type": "u8", "value": N } }.
// Codama expects { "discriminator": [b0, b1, ..., b7] }.
function fixDiscriminants(idl: any, map: Record<string, number[]>): any {
    for (const ix of idl.instructions) {
        const disc = map[ix.name];
        if (!disc) {
            console.warn(`WARNING: no discriminant mapping for instruction "${ix.name}"`);
            continue;
        }
        ix.discriminator = disc;
        delete ix.discriminant;
    }
    return idl;
}

// ─── process one IDL file ─────────────────────────────────────────────────────
function fixIdl(path: string, discriminants: Record<string, number[]>): void {
    let idl = JSON.parse(readFileSync(path, "utf-8"));
    idl = fixAddressTypes(idl);
    idl = fixDiscriminants(idl, discriminants);
    writeFileSync(path, JSON.stringify(idl, null, 2));
    console.log(`fixed: ${path}`);
}

// ─── run ──────────────────────────────────────────────────────────────────────
fixIdl("clients/idls/tyche_core.json",                 CORE_DISCRIMINANTS);
fixIdl("clients/idls/tyche_escrow.json",               ESCROW_DISCRIMINANTS);
fixIdl("clients/idls/tyche_auction.json",              AUCTION_DISCRIMINANTS);
fixIdl("clients/idls/tyche_voter_weight_plugin.json",  VOTER_WEIGHT_DISCRIMINANTS);


console.log("all IDLs fixed");

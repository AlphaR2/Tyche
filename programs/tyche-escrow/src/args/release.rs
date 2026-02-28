// `Release` has no caller-supplied args.
// All values needed for fund distribution are read directly from on-chain
// account state: `vault.amount` is the canonical purchase price, fee rate
// and treasury come from `ProtocolConfig`, and winner status from
// `ParticipantRecord`.  Accepting a caller-supplied amount would allow a
// malicious crank to under-pay the seller by passing an arbitrarily low value.

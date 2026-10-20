use soroban_sdk::{contracterror, contracttype, Address, BytesN};

/// Lifecycle of a single care voucher.
///
/// ```text
///                  claim            attest          settle
///   Funded ──────────────> Claimed ────────> Attested ──────> Settled
///     │                       │                  │
///     │ refund (past expiry)  │ dispute          │ dispute
///     ▼                       ▼                  ▼
///  Refunded  <──────────  Disputed  ────────────────> (resolve)
///     ▲                       │
///     └───────────────────────┘
///       refund (claimed, never attested, past expiry + claim grace)
/// ```
///
/// `Settled` and `Refunded` are terminal.
#[contracttype]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoucherStatus {
    Funded = 0,
    Claimed = 1,
    Attested = 2,
    Settled = 3,
    Disputed = 4,
    Refunded = 5,
}

/// An escrowed commitment to pay one provider for one service.
///
/// The funder is whoever paid — the patient themselves, or a family member
/// sending from abroad. `beneficiary_ref` is an opaque salted hash of an
/// off-chain patient identifier; it carries no clinical meaning and cannot
/// be reversed into an identity from on-chain data alone.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Voucher {
    pub id: u64,
    pub funder: Address,
    pub beneficiary_ref: BytesN<32>,
    pub provider: Address,
    pub service_code: u32,
    pub amount: i128,
    pub status: VoucherStatus,
    pub created_at: u64,
    pub expires_at: u64,
    pub claimed_at: u64,
    pub attested_at: u64,
    /// Earliest ledger time at which `settle` may run. Zero until attested.
    pub dispute_deadline: u64,
}

/// Optional terms a funder can attach to a voucher with
/// `create_voucher_with_options`. Both default to off, and a voucher funded
/// with `create_voucher` behaves exactly as before.
#[contracttype]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VoucherOptions {
    /// SHA-256 of a secret the funder gives the patient. When set, the clinic
    /// can only claim with `claim_with_proof`, revealing the secret, which
    /// shows it had the patient in front of it.
    pub presence_commitment: Option<BytesN<32>>,
    /// Seconds the funder has to dispute after attestation, in place of the
    /// protocol-wide `Config.dispute_window`. Must lie within the dispute
    /// window bounds.
    pub dispute_window: Option<u64>,
}

/// Protocol-level settings, fixed at initialization.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub admin: Address,
    pub registry: Address,
    pub receipt_book: Address,
    /// Settlement token. USDC on Stellar in production.
    pub token: Address,
    /// Seconds a funder has to dispute after attestation.
    pub dispute_window: u64,
    /// Protocol fee in basis points. 100 = 1%.
    pub fee_bps: u32,
    pub fee_account: Address,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Config,
    NextId,
    Voucher(u64),
    /// Address nominated by `propose_admin`, awaiting `accept_admin`.
    PendingAdmin,
    /// Most attestations one attester may make per UTC day. Unset means
    /// `DEFAULT_MAX_ATTESTATIONS_PER_DAY`.
    AttestationCap,
    /// Attestations made by an attester on a UTC day (`timestamp / 86400`).
    /// Temporary storage: it only has to outlive the day it counts.
    Attestations(Address, u64),
    /// A voucher's `VoucherOptions`. Written only for vouchers that set one,
    /// so a plain voucher pays for no extra storage.
    Options(u64),
    /// Shortest and longest per-voucher dispute window a funder may choose.
    /// Unset means `DEFAULT_DISPUTE_WINDOW_BOUNDS`.
    DisputeWindowBounds,
}

#[contracterror]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u32)]
pub enum VoucherError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    NotAuthorized = 3,
    VoucherNotFound = 4,
    ProviderNotActive = 5,
    ServiceNotOffered = 6,
    AmountBelowPrice = 7,
    InvalidAmount = 8,
    InvalidExpiry = 9,
    InvalidStatus = 10,
    VoucherExpired = 11,
    NotYetExpired = 12,
    DisputeWindowOpen = 13,
    DisputeWindowClosed = 14,
    InvalidFee = 15,
    MathOverflow = 16,
    NoPendingAdmin = 17,
    /// The clinic has paused this service; it cannot be funded until it
    /// is made available again.
    ServiceUnavailable = 18,
    /// This attester has used up today's attestations; try again tomorrow
    /// (UTC) or ask the admin to raise the cap.
    AttestationCapReached = 19,
    /// The attestation cap must be at least 1.
    InvalidAttestationCap = 20,
    /// This voucher was funded with a presence commitment, so the clinic must
    /// claim it with `claim_with_proof`.
    PresenceProofRequired = 21,
    /// The secret does not hash to the voucher's presence commitment.
    InvalidPresenceProof = 22,
    /// `claim_with_proof` on a voucher funded without a presence commitment.
    NoPresenceCommitment = 23,
    /// The chosen dispute window is outside the allowed bounds, or the bounds
    /// themselves are invalid.
    InvalidDisputeWindow = 24,
}

/// Maps the shared helpers' failures onto this contract's error numbers, so
/// callers keep seeing the same codes they did before the helpers were shared.
impl From<sci_common::CommonError> for VoucherError {
    fn from(error: sci_common::CommonError) -> Self {
        match error {
            sci_common::CommonError::NotAuthorized => VoucherError::NotAuthorized,
            sci_common::CommonError::NoPendingAdmin => VoucherError::NoPendingAdmin,
        }
    }
}

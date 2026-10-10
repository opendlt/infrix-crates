//! Infrix Types Library
//!
//! Core types for building Infrix smart contracts. This crate provides
//! fundamental types that are used across the Infrix ecosystem.
//!
//! # Features
//!
//! - `std` (default): Enable standard library support
//! - `alloc`: Enable alloc support without std
//!
//! # Example
//!
//! ```rust
//! use infrix_types::{U256, Address, Hash};
//!
//! let balance = U256::from(1000u64);
//! let zero = U256::zero();
//! assert!(balance > zero);
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use alloc::string::String;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::vec::Vec;

/// 256-bit unsigned integer
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
#[repr(C)]
pub struct U256 {
    /// Low 128 bits
    pub low: u128,
    /// High 128 bits
    pub high: u128,
}

impl U256 {
    /// Zero constant
    pub const ZERO: Self = Self { low: 0, high: 0 };

    /// One constant
    pub const ONE: Self = Self { low: 1, high: 0 };

    /// Create a zero value
    #[inline]
    pub const fn zero() -> Self {
        Self::ZERO
    }

    /// Create a one value
    #[inline]
    pub const fn one() -> Self {
        Self::ONE
    }

    /// Create from u64
    #[inline]
    pub const fn from_u64(value: u64) -> Self {
        Self {
            low: value as u128,
            high: 0,
        }
    }

    /// Create from u128
    #[inline]
    pub const fn from_u128(value: u128) -> Self {
        Self {
            low: value,
            high: 0,
        }
    }

    /// Create from two u128 values (high, low)
    #[inline]
    pub const fn from_parts(high: u128, low: u128) -> Self {
        Self { low, high }
    }

    /// Create from big-endian bytes
    pub fn from_be_bytes(bytes: &[u8; 32]) -> Self {
        let mut high_bytes = [0u8; 16];
        let mut low_bytes = [0u8; 16];
        high_bytes.copy_from_slice(&bytes[0..16]);
        low_bytes.copy_from_slice(&bytes[16..32]);
        Self {
            high: u128::from_be_bytes(high_bytes),
            low: u128::from_be_bytes(low_bytes),
        }
    }

    /// Create from little-endian bytes
    pub fn from_le_bytes(bytes: &[u8; 32]) -> Self {
        let mut low_bytes = [0u8; 16];
        let mut high_bytes = [0u8; 16];
        low_bytes.copy_from_slice(&bytes[0..16]);
        high_bytes.copy_from_slice(&bytes[16..32]);
        Self {
            low: u128::from_le_bytes(low_bytes),
            high: u128::from_le_bytes(high_bytes),
        }
    }

    /// Convert to big-endian bytes
    pub fn to_be_bytes(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[0..16].copy_from_slice(&self.high.to_be_bytes());
        bytes[16..32].copy_from_slice(&self.low.to_be_bytes());
        bytes
    }

    /// Convert to little-endian bytes
    pub fn to_le_bytes(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[0..16].copy_from_slice(&self.low.to_le_bytes());
        bytes[16..32].copy_from_slice(&self.high.to_le_bytes());
        bytes
    }

    /// Check if value is zero
    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.low == 0 && self.high == 0
    }

    /// Checked addition
    pub fn checked_add(&self, other: &Self) -> Option<Self> {
        let (low, carry) = self.low.overflowing_add(other.low);
        let high = self.high.checked_add(other.high)?;
        let high = if carry { high.checked_add(1)? } else { high };
        Some(Self { low, high })
    }

    /// Checked subtraction
    pub fn checked_sub(&self, other: &Self) -> Option<Self> {
        let (low, borrow) = self.low.overflowing_sub(other.low);
        let high = self.high.checked_sub(other.high)?;
        let high = if borrow { high.checked_sub(1)? } else { high };
        Some(Self { low, high })
    }

    /// Checked multiplication
    pub fn checked_mul(&self, other: &Self) -> Option<Self> {
        // For simplicity, only handle cases where result fits in 256 bits
        if self.high != 0 && other.high != 0 {
            return None; // Would overflow
        }

        if self.high == 0 && other.high == 0 {
            let (low, high) = wide_mul_u128(self.low, other.low);
            return Some(Self { low, high });
        }

        // One has high bits, one doesn't
        let (a, b) = if self.high != 0 {
            (self, other)
        } else {
            (other, self)
        };

        // a.high != 0, b.high == 0
        let low_product = wide_mul_u128(a.low, b.low);
        let cross_product = a.high.checked_mul(b.low)?;

        let high = low_product.1.checked_add(cross_product)?;
        Some(Self {
            low: low_product.0,
            high,
        })
    }

    /// Checked division
    pub fn checked_div(&self, other: &Self) -> Option<Self> {
        if other.is_zero() {
            return None;
        }

        // Simple division for when divisor fits in u128
        if other.high == 0 && self.high == 0 {
            return Some(Self {
                low: self.low / other.low,
                high: 0,
            });
        }

        // For larger divisions, use long division algorithm
        if *self < *other {
            return Some(Self::ZERO);
        }

        // Fast path: the divisor fits in u128 (high limb is zero); the
        // full 256-bit binary long division below handles the rest.
        if other.high == 0 {
            let divisor = other.low;
            let mut quotient = Self::ZERO;

            // Process high bits
            let mut remainder: u128 = self.high % divisor;
            quotient.high = self.high / divisor;

            // Process low bits with remainder
            let combined = ((remainder as u128) << 64) | (self.low >> 64);
            let q_high = combined / divisor;
            remainder = combined % divisor;

            let combined = (remainder << 64) | (self.low & ((1u128 << 64) - 1));
            let q_low = combined / divisor;

            quotient.low = (q_high << 64) | q_low;
            return Some(quotient);
        }

        // Full 256-bit division - use binary long division
        let mut quotient = Self::ZERO;
        let mut remainder = Self::ZERO;

        for i in (0..256).rev() {
            // Left shift remainder by 1
            remainder.high = (remainder.high << 1) | (remainder.low >> 127);
            remainder.low <<= 1;

            // Set lowest bit from dividend
            let bit = if i >= 128 {
                (self.high >> (i - 128)) & 1
            } else {
                (self.low >> i) & 1
            };
            remainder.low |= bit as u128;

            // If remainder >= divisor, subtract and set quotient bit
            if remainder >= *other {
                remainder = remainder.checked_sub(other)?;
                if i >= 128 {
                    quotient.high |= 1u128 << (i - 128);
                } else {
                    quotient.low |= 1u128 << i;
                }
            }
        }

        Some(quotient)
    }

    /// Saturating addition
    pub fn saturating_add(&self, other: &Self) -> Self {
        self.checked_add(other).unwrap_or(Self::max_value())
    }

    /// Saturating subtraction
    pub fn saturating_sub(&self, other: &Self) -> Self {
        self.checked_sub(other).unwrap_or(Self::zero())
    }

    /// Maximum value
    #[inline]
    pub const fn max_value() -> Self {
        Self {
            low: u128::MAX,
            high: u128::MAX,
        }
    }

    /// Count leading zeros
    pub fn leading_zeros(&self) -> u32 {
        if self.high == 0 {
            128 + self.low.leading_zeros()
        } else {
            self.high.leading_zeros()
        }
    }

    /// Count trailing zeros
    pub fn trailing_zeros(&self) -> u32 {
        if self.low == 0 {
            128 + self.high.trailing_zeros()
        } else {
            self.low.trailing_zeros()
        }
    }

    /// Bit count
    pub fn count_ones(&self) -> u32 {
        self.low.count_ones() + self.high.count_ones()
    }
}

// Helper function for wide multiplication
fn wide_mul_u128(a: u128, b: u128) -> (u128, u128) {
    let a_lo = a as u64 as u128;
    let a_hi = (a >> 64) as u64 as u128;
    let b_lo = b as u64 as u128;
    let b_hi = (b >> 64) as u64 as u128;

    let lo_lo = a_lo * b_lo;
    let hi_lo = a_hi * b_lo;
    let lo_hi = a_lo * b_hi;
    let hi_hi = a_hi * b_hi;

    let (mid, carry1) = lo_lo.overflowing_add(hi_lo << 64);
    let (mid, carry2) = mid.overflowing_add(lo_hi << 64);

    let high = hi_hi + (hi_lo >> 64) + (lo_hi >> 64) + carry1 as u128 + carry2 as u128;

    (mid, high)
}

impl From<u64> for U256 {
    #[inline]
    fn from(value: u64) -> Self {
        Self::from_u64(value)
    }
}

impl From<u128> for U256 {
    #[inline]
    fn from(value: u128) -> Self {
        Self::from_u128(value)
    }
}

impl core::fmt::Debug for U256 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "U256({:#x}, {:#x})", self.high, self.low)
    }
}

impl core::fmt::Display for U256 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.high == 0 {
            write!(f, "{}", self.low)
        } else {
            write!(f, "0x{:032x}{:032x}", self.high, self.low)
        }
    }
}

impl core::ops::Add for U256 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        self.checked_add(&rhs).expect("U256 overflow")
    }
}

impl core::ops::Sub for U256 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(&rhs).expect("U256 underflow")
    }
}

impl core::ops::Mul for U256 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        self.checked_mul(&rhs).expect("U256 overflow")
    }
}

/// 32-byte hash type
#[derive(Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct Hash(pub [u8; 32]);

impl Hash {
    /// Create zero hash
    #[inline]
    pub const fn zero() -> Self {
        Self([0u8; 32])
    }

    /// Create from bytes
    #[inline]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Get as bytes
    #[inline]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Get as mutable bytes
    #[inline]
    pub fn as_bytes_mut(&mut self) -> &mut [u8; 32] {
        &mut self.0
    }

    /// Check if zero
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.0 == [0u8; 32]
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsMut<[u8]> for Hash {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl From<[u8; 32]> for Hash {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl core::fmt::Debug for Hash {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Hash(0x")?;
        for byte in &self.0[..4] {
            write!(f, "{:02x}", byte)?;
        }
        write!(f, "...")?;
        for byte in &self.0[28..] {
            write!(f, "{:02x}", byte)?;
        }
        write!(f, ")")
    }
}

impl core::fmt::Display for Hash {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "0x")?;
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

/// Accumulate URL address type
#[derive(Clone, PartialEq, Eq)]
pub struct Address {
    /// Raw bytes of the address (up to 256 bytes)
    data: [u8; 256],
    /// Length of the address
    len: u8,
}

impl Address {
    /// Maximum address length.
    ///
    /// 255, not 256, because `len` is a `u8` and the wire form's length prefix
    /// is one byte. It read 256 until 2026-09-05, which meant `from_bytes` on a
    /// 256-byte input built an address whose length field was `256 as u8` — ZERO
    /// — and every caller then saw an EMPTY address. The value was off by one
    /// against its own representation, and the failure was silent in the worst
    /// possible place: a recipient. `from_bytes` now returns `None` there.
    pub const MAX_LEN: usize = 255;

    /// Create empty address
    pub const fn empty() -> Self {
        Self {
            data: [0u8; 256],
            len: 0,
        }
    }

    /// Create from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > Self::MAX_LEN {
            return None;
        }
        let mut data = [0u8; 256];
        data[..bytes.len()].copy_from_slice(bytes);
        Some(Self {
            data,
            len: bytes.len() as u8,
        })
    }

    /// Create from string
    pub fn from_str(s: &str) -> Option<Self> {
        Self::from_bytes(s.as_bytes())
    }

    /// Get as bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.data[..self.len as usize]
    }

    /// Get length
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Convert to string (if valid UTF-8)
    #[cfg(feature = "alloc")]
    pub fn to_string(&self) -> Option<String> {
        core::str::from_utf8(self.as_bytes())
            .ok()
            .map(|s| s.to_string())
    }

    /// Get as str (if valid UTF-8)
    pub fn as_str(&self) -> Option<&str> {
        core::str::from_utf8(self.as_bytes()).ok()
    }

    /// Check if this is an ADI URL
    pub fn is_adi(&self) -> bool {
        self.as_str()
            .map(|s| s.starts_with("acc://") && !s.contains('/'))
            .unwrap_or(false)
    }

    /// Get the ADI portion of the URL
    pub fn adi(&self) -> Option<&str> {
        self.as_str().and_then(|s| {
            if s.starts_with("acc://") {
                let rest = &s[6..];
                rest.split('/').next()
            } else {
                None
            }
        })
    }

    /// Get the path portion of the URL
    pub fn path(&self) -> Option<&str> {
        self.as_str().and_then(|s| {
            if s.starts_with("acc://") {
                let rest = &s[6..];
                rest.find('/').map(|i| &rest[i..])
            } else {
                None
            }
        })
    }
}

impl Default for Address {
    fn default() -> Self {
        Self::empty()
    }
}

impl core::fmt::Debug for Address {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Some(s) = self.as_str() {
            write!(f, "Address({:?})", s)
        } else {
            write!(f, "Address({:?})", self.as_bytes())
        }
    }
}

impl core::fmt::Display for Address {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Some(s) = self.as_str() {
            write!(f, "{}", s)
        } else {
            write!(f, "<binary address>")
        }
    }
}

/// Contract execution error.
///
/// # This enum is the ABI's result-code registry
///
/// The dispatcher generated by `#[infrix::contract]` exports
/// `__infrix_call(input_ptr, input_len, output_ptr) -> i32`, which returns the
/// number of bytes written on success and, on failure, **the negation of this
/// enum's discriminant** (see [`Error::result_code`]). A contract refusing with
/// [`Error::UnknownFunction`] therefore answers `-25` on the wire, and one
/// refusing with [`Error::InsufficientBalance`] answers `-7`.
///
/// That makes these discriminants a **published wire contract, not an internal
/// detail**. A host reads them to tell an unrecognised selector apart from a
/// domain refusal, and every already-deployed contract answers the numbers
/// below. Consequences, in order of how much they will hurt:
///
/// * **Never renumber a variant.** `test_result_codes_are_a_pinned_wire_contract`
///   pins every discriminant for exactly this reason; changing one silently
///   re-labels the refusals of contracts already on chain.
/// * **Never reuse a retired number** for a different meaning.
/// * Appending a new variant is safe. A host that does not know it reports the
///   refusal verbatim as unrecognised rather than guessing.
///
/// The in-tree consumers of this table are `pkg/host/contract_result_code.go`
/// in infrix-core and the result-code table in the JavaScript SDK; both hold
/// themselves against this file in CI.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum Error {
    /// No error
    Success = 0,
    /// Invalid argument
    InvalidArgument = 1,
    /// Out of memory
    OutOfMemory = 2,
    /// Out of gas
    OutOfGas = 3,
    /// Storage error
    StorageError = 4,
    /// Authorization failed
    Unauthorized = 5,
    /// Account not found
    AccountNotFound = 6,
    /// Insufficient balance
    InsufficientBalance = 7,
    /// Insufficient credits
    InsufficientCredits = 8,
    /// Contract not found
    ContractNotFound = 9,
    /// Function not found
    FunctionNotFound = 10,
    /// Invalid state
    InvalidState = 11,
    /// Overflow
    Overflow = 12,
    /// Underflow
    Underflow = 13,
    /// Division by zero
    DivisionByZero = 14,
    /// Invalid signature
    InvalidSignature = 15,
    /// Revert with reason
    Revert = 16,
    /// Panic
    Panic = 17,
    /// Call depth exceeded
    CallDepthExceeded = 18,
    /// Read only violation
    ReadOnlyViolation = 19,
    /// Reentrancy detected
    ReentrancyDetected = 20,
    /// Function not payable
    NotPayable = 21,
    /// Zero address not allowed
    ZeroAddress = 22,
    /// Contract not initialized
    ContractNotInitialized = 23,
    /// Invalid input data
    InvalidInput = 24,
    /// Unknown function selector
    UnknownFunction = 25,
    /// Encoding error
    EncodingError = 26,
    /// Decoding error
    DecodingError = 27,
    /// Buffer too small
    BufferTooSmall = 28,
    /// Governance operation failed
    GovernanceError = 29,
    /// Approval required before execution can proceed
    ApprovalRequired = 30,
    /// Caller lacks required capability
    CapabilityDenied = 31,
    /// Caller lacks required role
    RoleRequired = 32,
    /// Policy denied the operation
    PolicyDenied = 33,
    /// Unknown error
    Unknown = 255,
}

impl Error {
    /// The value the dispatcher returns on the wire for this error: the
    /// negation of the discriminant.
    ///
    /// This is the single definition of the ABI's failure convention. The
    /// `__infrix_call` entry point generated by `#[infrix::contract]` returns
    /// exactly this for its `Err` arm, so the convention is expressed once and
    /// cannot drift between the macro and this type.
    ///
    /// `Error::Success.result_code()` is `0`, which on the wire is a zero-byte
    /// success rather than a refusal; a dispatcher never returns `Success` down
    /// the error arm.
    pub fn result_code(self) -> i32 {
        -(self as u32 as i32)
    }

    /// Recover the error a wire result code names, or `None` when the value is
    /// not a refusal this registry defines.
    ///
    /// Non-negative input is a byte count, not a code, and yields `None`. So
    /// does a negative value with no variant: a contract may answer its own
    /// numbers, and a caller must report those verbatim rather than borrow a
    /// meaning for them.
    pub fn from_result_code(code: i32) -> Option<Self> {
        if code >= 0 {
            return None;
        }
        let magnitude = code.checked_neg()? as u32;
        let recovered = Self::from_code(magnitude);
        // from_code funnels anything unrecognised into Unknown (255), so an
        // input that is not literally 255 but came back as Unknown was never in
        // the registry.
        if recovered == Self::Unknown && magnitude != Self::Unknown as u32 {
            return None;
        }
        Some(recovered)
    }

    /// Create from error code
    pub fn from_code(code: u32) -> Self {
        match code {
            0 => Self::Success,
            1 => Self::InvalidArgument,
            2 => Self::OutOfMemory,
            3 => Self::OutOfGas,
            4 => Self::StorageError,
            5 => Self::Unauthorized,
            6 => Self::AccountNotFound,
            7 => Self::InsufficientBalance,
            8 => Self::InsufficientCredits,
            9 => Self::ContractNotFound,
            10 => Self::FunctionNotFound,
            11 => Self::InvalidState,
            12 => Self::Overflow,
            13 => Self::Underflow,
            14 => Self::DivisionByZero,
            15 => Self::InvalidSignature,
            16 => Self::Revert,
            17 => Self::Panic,
            18 => Self::CallDepthExceeded,
            19 => Self::ReadOnlyViolation,
            20 => Self::ReentrancyDetected,
            21 => Self::NotPayable,
            22 => Self::ZeroAddress,
            23 => Self::ContractNotInitialized,
            24 => Self::InvalidInput,
            25 => Self::UnknownFunction,
            26 => Self::EncodingError,
            27 => Self::DecodingError,
            28 => Self::BufferTooSmall,
            29 => Self::GovernanceError,
            30 => Self::ApprovalRequired,
            31 => Self::CapabilityDenied,
            32 => Self::RoleRequired,
            33 => Self::PolicyDenied,
            _ => Self::Unknown,
        }
    }

    /// Get error code
    pub fn code(&self) -> u32 {
        *self as u32
    }

    /// Check if success
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    /// Check if error
    pub fn is_error(&self) -> bool {
        !self.is_success()
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Success => write!(f, "success"),
            Self::InvalidArgument => write!(f, "invalid argument"),
            Self::OutOfMemory => write!(f, "out of memory"),
            Self::OutOfGas => write!(f, "out of gas"),
            Self::StorageError => write!(f, "storage error"),
            Self::Unauthorized => write!(f, "unauthorized"),
            Self::AccountNotFound => write!(f, "account not found"),
            Self::InsufficientBalance => write!(f, "insufficient balance"),
            Self::InsufficientCredits => write!(f, "insufficient credits"),
            Self::ContractNotFound => write!(f, "contract not found"),
            Self::FunctionNotFound => write!(f, "function not found"),
            Self::InvalidState => write!(f, "invalid state"),
            Self::Overflow => write!(f, "overflow"),
            Self::Underflow => write!(f, "underflow"),
            Self::DivisionByZero => write!(f, "division by zero"),
            Self::InvalidSignature => write!(f, "invalid signature"),
            Self::Revert => write!(f, "revert"),
            Self::Panic => write!(f, "panic"),
            Self::CallDepthExceeded => write!(f, "call depth exceeded"),
            Self::ReadOnlyViolation => write!(f, "read only violation"),
            Self::ReentrancyDetected => write!(f, "reentrancy detected"),
            Self::NotPayable => write!(f, "function not payable"),
            Self::ZeroAddress => write!(f, "zero address not allowed"),
            Self::ContractNotInitialized => write!(f, "contract not initialized"),
            Self::InvalidInput => write!(f, "invalid input data"),
            Self::UnknownFunction => write!(f, "unknown function selector"),
            Self::EncodingError => write!(f, "encoding error"),
            Self::DecodingError => write!(f, "decoding error"),
            Self::BufferTooSmall => write!(f, "buffer too small"),
            Self::GovernanceError => write!(f, "governance operation failed"),
            Self::ApprovalRequired => write!(f, "approval required"),
            Self::CapabilityDenied => write!(f, "capability denied"),
            Self::RoleRequired => write!(f, "role required"),
            Self::PolicyDenied => write!(f, "policy denied"),
            Self::Unknown => write!(f, "unknown error"),
        }
    }
}

/// Result type for contract operations
pub type Result<T> = core::result::Result<T, Error>;

/// Account information from L0
#[derive(Clone, Debug, Default)]
pub struct Account {
    /// Account address
    pub address: Address,
    /// Account type
    pub account_type: AccountType,
    /// Token balance (for token accounts)
    pub balance: U256,
    /// Credit balance
    pub credits: u64,
    /// Transaction count/nonce
    pub nonce: u64,
    /// Code hash (for contracts)
    pub code_hash: Hash,
    /// State root (for contracts)
    pub state_root: Hash,
}

/// Account type
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[repr(u8)]
pub enum AccountType {
    /// Unknown account type
    #[default]
    Unknown = 0,
    /// ADI (Accumulate Digital Identifier)
    ADI = 1,
    /// Token account
    TokenAccount = 2,
    /// Data account
    DataAccount = 3,
    /// Key book
    KeyBook = 4,
    /// Key page
    KeyPage = 5,
    /// Contract account
    Contract = 6,
}

impl AccountType {
    /// Create from byte value
    pub fn from_byte(b: u8) -> Self {
        match b {
            1 => Self::ADI,
            2 => Self::TokenAccount,
            3 => Self::DataAccount,
            4 => Self::KeyBook,
            5 => Self::KeyPage,
            6 => Self::Contract,
            _ => Self::Unknown,
        }
    }
}

/// Token information
#[derive(Clone, Debug)]
pub struct TokenInfo {
    /// Token URL
    pub url: Address,
    /// Token symbol
    pub symbol: [u8; 8],
    /// Token precision (decimals)
    pub precision: u8,
    /// Total supply
    pub total_supply: U256,
}

impl Default for TokenInfo {
    fn default() -> Self {
        Self {
            url: Address::empty(),
            symbol: [0u8; 8],
            precision: 0,
            total_supply: U256::zero(),
        }
    }
}

/// Signature data
#[derive(Clone, Debug)]
pub struct Signature {
    /// Signature type
    pub sig_type: SignatureType,
    /// Public key
    pub public_key: [u8; 64],
    /// Public key length
    pub public_key_len: u8,
    /// Signature bytes
    pub signature: [u8; 128],
    /// Signature length
    pub signature_len: u8,
    /// Signer URL
    pub signer: Address,
}

impl Default for Signature {
    fn default() -> Self {
        Self {
            sig_type: SignatureType::Unknown,
            public_key: [0u8; 64],
            public_key_len: 0,
            signature: [0u8; 128],
            signature_len: 0,
            signer: Address::empty(),
        }
    }
}

/// Signature type
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[repr(u8)]
pub enum SignatureType {
    /// Unknown signature type
    #[default]
    Unknown = 0,
    /// ED25519
    ED25519 = 1,
    /// SECP256K1
    SECP256K1 = 2,
    /// SECP256K1 with recovery
    SECP256K1Recoverable = 3,
    /// RCD1 (Factom compatibility)
    RCD1 = 4,
    /// BLS signature
    BLS = 5,
}

impl SignatureType {
    /// Create from byte value
    pub fn from_byte(b: u8) -> Self {
        match b {
            1 => Self::ED25519,
            2 => Self::SECP256K1,
            3 => Self::SECP256K1Recoverable,
            4 => Self::RCD1,
            5 => Self::BLS,
            _ => Self::Unknown,
        }
    }
}

/// Event topic (32 bytes)
#[derive(Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct Topic(pub [u8; 32]);

impl Topic {
    /// Empty topic
    pub const EMPTY: Self = Self([0u8; 32]);

    /// Create from bytes
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Create from u32 (used for event signatures)
    pub fn from_u32(value: u32) -> Self {
        let mut bytes = [0u8; 32];
        bytes[..4].copy_from_slice(&value.to_be_bytes());
        Self(bytes)
    }

    /// Create from hash
    pub fn from_hash(hash: Hash) -> Self {
        Self(hash.0)
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.0 == [0u8; 32]
    }
}

impl From<[u8; 32]> for Topic {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl core::fmt::Debug for Topic {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Topic(0x")?;
        for byte in &self.0[..8] {
            write!(f, "{:02x}", byte)?;
        }
        write!(f, "...)")
    }
}

/// Contract event
#[derive(Clone, Debug)]
pub struct Event {
    /// Event topics (indexed parameters)
    pub topics: [Topic; 4],
    /// Number of topics
    pub topic_count: u8,
    /// Event data (non-indexed parameters)
    pub data: [u8; 1024],
    /// Data length
    pub data_len: u16,
}

impl Default for Event {
    fn default() -> Self {
        Self {
            topics: [Topic([0u8; 32]); 4],
            topic_count: 0,
            data: [0u8; 1024],
            data_len: 0,
        }
    }
}

impl Event {
    /// Create new event with one topic
    pub fn new(topic0: Topic) -> Self {
        let mut event = Self::default();
        event.topics[0] = topic0;
        event.topic_count = 1;
        event
    }

    /// Add a topic
    pub fn with_topic(mut self, topic: Topic) -> Self {
        if self.topic_count < 4 {
            self.topics[self.topic_count as usize] = topic;
            self.topic_count += 1;
        }
        self
    }

    /// Set event data
    pub fn with_data(mut self, data: &[u8]) -> Self {
        let len = core::cmp::min(data.len(), 1024);
        self.data[..len].copy_from_slice(&data[..len]);
        self.data_len = len as u16;
        self
    }

    /// Get topics slice
    pub fn topics(&self) -> &[Topic] {
        &self.topics[..self.topic_count as usize]
    }

    /// Get data slice
    pub fn data(&self) -> &[u8] {
        &self.data[..self.data_len as usize]
    }
}

/// Execution context information
#[derive(Clone, Debug, Default)]
pub struct Context {
    /// Caller address
    pub caller: Address,
    /// Current block height
    pub block_height: u64,
    /// Current block timestamp (Unix seconds)
    pub block_time: u64,
    /// Transaction hash
    pub tx_hash: Hash,
    /// Value sent with call
    pub value: U256,
    /// Gas limit
    pub gas_limit: u64,
}

// =============================================================================
// Encoding/Decoding Traits
// =============================================================================

/// Trait for encoding values to bytes
pub trait Encode {
    /// Encode self into the buffer, returning the number of bytes written
    fn encode(&self, buffer: &mut [u8]) -> Result<usize>;
}

/// Trait for decoding values from bytes
pub trait Decode: Sized {
    /// Decode from bytes
    fn decode(data: &[u8]) -> Result<Self>;

    /// Decode from bytes, returning the value and number of bytes consumed
    fn decode_with_len(data: &[u8]) -> Result<(Self, usize)> {
        let value = Self::decode(data)?;
        Ok((value, data.len()))
    }
}

// Implement Encode/Decode for primitive types

impl Encode for u8 {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize> {
        if buffer.is_empty() {
            return Err(Error::BufferTooSmall);
        }
        buffer[0] = *self;
        Ok(1)
    }
}

impl Decode for u8 {
    fn decode(data: &[u8]) -> Result<Self> {
        if data.is_empty() {
            return Err(Error::DecodingError);
        }
        Ok(data[0])
    }

    fn decode_with_len(data: &[u8]) -> Result<(Self, usize)> {
        Ok((Self::decode(data)?, 1))
    }
}

impl Encode for u16 {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize> {
        if buffer.len() < 2 {
            return Err(Error::BufferTooSmall);
        }
        buffer[..2].copy_from_slice(&self.to_be_bytes());
        Ok(2)
    }
}

impl Decode for u16 {
    fn decode(data: &[u8]) -> Result<Self> {
        if data.len() < 2 {
            return Err(Error::DecodingError);
        }
        let mut bytes = [0u8; 2];
        bytes.copy_from_slice(&data[..2]);
        Ok(u16::from_be_bytes(bytes))
    }

    fn decode_with_len(data: &[u8]) -> Result<(Self, usize)> {
        Ok((Self::decode(data)?, 2))
    }
}

impl Encode for u32 {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize> {
        if buffer.len() < 4 {
            return Err(Error::BufferTooSmall);
        }
        buffer[..4].copy_from_slice(&self.to_be_bytes());
        Ok(4)
    }
}

impl Decode for u32 {
    fn decode(data: &[u8]) -> Result<Self> {
        if data.len() < 4 {
            return Err(Error::DecodingError);
        }
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(&data[..4]);
        Ok(u32::from_be_bytes(bytes))
    }

    fn decode_with_len(data: &[u8]) -> Result<(Self, usize)> {
        Ok((Self::decode(data)?, 4))
    }
}

impl Encode for u64 {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize> {
        if buffer.len() < 8 {
            return Err(Error::BufferTooSmall);
        }
        buffer[..8].copy_from_slice(&self.to_be_bytes());
        Ok(8)
    }
}

impl Decode for u64 {
    fn decode(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            return Err(Error::DecodingError);
        }
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&data[..8]);
        Ok(u64::from_be_bytes(bytes))
    }

    fn decode_with_len(data: &[u8]) -> Result<(Self, usize)> {
        Ok((Self::decode(data)?, 8))
    }
}

impl Encode for u128 {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize> {
        if buffer.len() < 16 {
            return Err(Error::BufferTooSmall);
        }
        buffer[..16].copy_from_slice(&self.to_be_bytes());
        Ok(16)
    }
}

impl Decode for u128 {
    fn decode(data: &[u8]) -> Result<Self> {
        if data.len() < 16 {
            return Err(Error::DecodingError);
        }
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&data[..16]);
        Ok(u128::from_be_bytes(bytes))
    }

    fn decode_with_len(data: &[u8]) -> Result<(Self, usize)> {
        Ok((Self::decode(data)?, 16))
    }
}

impl Encode for bool {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize> {
        if buffer.is_empty() {
            return Err(Error::BufferTooSmall);
        }
        buffer[0] = if *self { 1 } else { 0 };
        Ok(1)
    }
}

impl Decode for bool {
    fn decode(data: &[u8]) -> Result<Self> {
        if data.is_empty() {
            return Err(Error::DecodingError);
        }
        Ok(data[0] != 0)
    }

    fn decode_with_len(data: &[u8]) -> Result<(Self, usize)> {
        Ok((Self::decode(data)?, 1))
    }
}

impl Encode for U256 {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize> {
        if buffer.len() < 32 {
            return Err(Error::BufferTooSmall);
        }
        buffer[..32].copy_from_slice(&self.to_be_bytes());
        Ok(32)
    }
}

impl Decode for U256 {
    fn decode(data: &[u8]) -> Result<Self> {
        if data.len() < 32 {
            return Err(Error::DecodingError);
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&data[..32]);
        Ok(U256::from_be_bytes(&bytes))
    }

    fn decode_with_len(data: &[u8]) -> Result<(Self, usize)> {
        Ok((Self::decode(data)?, 32))
    }
}

impl Encode for Hash {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize> {
        if buffer.len() < 32 {
            return Err(Error::BufferTooSmall);
        }
        buffer[..32].copy_from_slice(&self.0);
        Ok(32)
    }
}

impl Decode for Hash {
    fn decode(data: &[u8]) -> Result<Self> {
        if data.len() < 32 {
            return Err(Error::DecodingError);
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&data[..32]);
        Ok(Hash(bytes))
    }

    fn decode_with_len(data: &[u8]) -> Result<(Self, usize)> {
        Ok((Self::decode(data)?, 32))
    }
}

// ---------------------------------------------------------------------------
// Signed integers.
//
// Two's complement, big-endian, at the same widths as their unsigned twins.
// `rust_type_to_schema` has emitted "i32" and "i64" since it was written, so a
// contract could DECLARE a signed parameter — and then fail to build, because
// no Decode existed for it. The declaration vocabulary and the codec now agree.
//
// Matrix row A.ABI.CODEC.DECLAREDTYPES.

macro_rules! impl_signed_codec {
    ($t:ty, $width:expr) => {
        impl Encode for $t {
            fn encode(&self, buffer: &mut [u8]) -> Result<usize> {
                if buffer.len() < $width {
                    return Err(Error::BufferTooSmall);
                }
                buffer[..$width].copy_from_slice(&self.to_be_bytes());
                Ok($width)
            }
        }

        impl Decode for $t {
            fn decode(data: &[u8]) -> Result<Self> {
                if data.len() < $width {
                    return Err(Error::DecodingError);
                }
                let mut bytes = [0u8; $width];
                bytes.copy_from_slice(&data[..$width]);
                Ok(<$t>::from_be_bytes(bytes))
            }

            fn decode_with_len(data: &[u8]) -> Result<(Self, usize)> {
                Ok((Self::decode(data)?, $width))
            }
        }
    };
}

impl_signed_codec!(i8, 1);
impl_signed_codec!(i16, 2);
impl_signed_codec!(i32, 4);
impl_signed_codec!(i64, 8);
impl_signed_codec!(i128, 16);

/// A 20-byte identifier — the width an EVM-shaped address occupies.
///
/// `rust_type_to_schema` maps `[u8; 20]` to the schema type "bytes20"; this is
/// the codec that makes such a declaration callable. Fixed width, so no length
/// prefix: the declaration says how many bytes there are.
impl Encode for [u8; 20] {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize> {
        if buffer.len() < 20 {
            return Err(Error::BufferTooSmall);
        }
        buffer[..20].copy_from_slice(self);
        Ok(20)
    }
}

impl Decode for [u8; 20] {
    fn decode(data: &[u8]) -> Result<Self> {
        if data.len() < 20 {
            return Err(Error::DecodingError);
        }
        let mut out = [0u8; 20];
        out.copy_from_slice(&data[..20]);
        Ok(out)
    }

    fn decode_with_len(data: &[u8]) -> Result<(Self, usize)> {
        Ok((Self::decode(data)?, 20))
    }
}

// ---------------------------------------------------------------------------
// Variable-length values: String and Vec<u8>.
//
// # THE LENGTH PREFIX IS FOUR BYTES, AND NOT ONE
//
// `Address` carries a ONE-byte length, and that is now permanent: contracts are
// deployed speaking it. It is also the wrong choice for a general value, and
// demonstrably so — it used to admit 256 bytes while the prefix is a `u8`, so an
// address of exactly 256 bytes wrote its length as 0 and decoded as empty.
// (Encode now refuses that rather than corrupting it; see below.)
//
// A string or a byte string has no comparable ceiling: a memo, a serialised
// proof, a JSON payload. A one-byte prefix would put the same silent-truncation
// edge in front of every one of them. So these carry a BIG-ENDIAN u32 length,
// which matches every other multi-byte integer in this codec, needs no varint
// machinery in `no_std`, and cannot truncate.
//
// The cost is three bytes per value against Address's encoding. That is the
// right trade for a value whose size the type does not bound.
//
// # WHAT DECODE REFUSES
//
// A length the buffer cannot satisfy, and — for String — bytes that are not
// valid UTF-8. A contract handed a `String` whose bytes are not UTF-8 would be
// holding something the type says is impossible, so it is a decoding error and
// not a lossy conversion.

/// Maximum length this codec will encode or decode for a variable-length value.
///
/// It is not a storage limit; it is a sanity bound that keeps a corrupt or
/// hostile length prefix from being taken at face value. The dispatch output
/// buffer is 4 KiB, so a RETURN is bounded well below this in practice, and an
/// argument larger than this is far past anything the ABI was built to carry.
pub const MAX_VARLEN: usize = 16 * 1024 * 1024;

#[cfg(any(feature = "std", feature = "alloc"))]
fn encode_varlen(bytes: &[u8], buffer: &mut [u8]) -> Result<usize> {
    if bytes.len() > MAX_VARLEN {
        return Err(Error::InvalidArgument);
    }
    let total = 4 + bytes.len();
    if buffer.len() < total {
        return Err(Error::BufferTooSmall);
    }
    buffer[..4].copy_from_slice(&(bytes.len() as u32).to_be_bytes());
    buffer[4..total].copy_from_slice(bytes);
    Ok(total)
}

#[cfg(any(feature = "std", feature = "alloc"))]
fn decode_varlen(data: &[u8]) -> Result<(&[u8], usize)> {
    if data.len() < 4 {
        return Err(Error::DecodingError);
    }
    let mut len_bytes = [0u8; 4];
    len_bytes.copy_from_slice(&data[..4]);
    let len = u32::from_be_bytes(len_bytes) as usize;
    if len > MAX_VARLEN {
        return Err(Error::DecodingError);
    }
    if data.len() < 4 + len {
        return Err(Error::DecodingError);
    }
    Ok((&data[4..4 + len], 4 + len))
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl Encode for String {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize> {
        encode_varlen(self.as_bytes(), buffer)
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl Decode for String {
    fn decode(data: &[u8]) -> Result<Self> {
        let (bytes, _) = decode_varlen(data)?;
        // A String whose bytes are not UTF-8 is not a String. Refuse rather
        // than convert lossily: the contract would otherwise act on characters
        // nobody sent.
        match core::str::from_utf8(bytes) {
            Ok(s) => Ok(s.into()),
            Err(_) => Err(Error::DecodingError),
        }
    }

    fn decode_with_len(data: &[u8]) -> Result<(Self, usize)> {
        let (bytes, consumed) = decode_varlen(data)?;
        match core::str::from_utf8(bytes) {
            Ok(s) => Ok((s.into(), consumed)),
            Err(_) => Err(Error::DecodingError),
        }
    }
}

/// `&str` encodes exactly as `String`, so a contract can RETURN a borrowed
/// string without an allocation. It has no `Decode`: a decoded value must own
/// its bytes, and the macro refuses a `&str` parameter by name rather than
/// letting it fail as an unsatisfied trait bound.
#[cfg(any(feature = "std", feature = "alloc"))]
impl Encode for &str {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize> {
        encode_varlen(self.as_bytes(), buffer)
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl Encode for Vec<u8> {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize> {
        encode_varlen(self.as_slice(), buffer)
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl Decode for Vec<u8> {
    fn decode(data: &[u8]) -> Result<Self> {
        let (bytes, _) = decode_varlen(data)?;
        Ok(bytes.into())
    }

    fn decode_with_len(data: &[u8]) -> Result<(Self, usize)> {
        let (bytes, consumed) = decode_varlen(data)?;
        Ok((bytes.into(), consumed))
    }
}

/// `&[u8]` encodes exactly as `Vec<u8>`, for the same reason `&str` does.
#[cfg(any(feature = "std", feature = "alloc"))]
impl Encode for &[u8] {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize> {
        encode_varlen(self, buffer)
    }
}

impl Encode for Address {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize> {
        let len = self.len();
        // The length prefix is ONE byte. Before MAX_LEN was corrected to 255,
        // an address of exactly 256 bytes wrote its length as 0 and decoded as empty —
        // a different address, silently. Refuse instead. The prefix width is
        // permanent (deployed contracts speak it), so the bound is enforced
        // here rather than widened. String and Vec<u8> carry a u32 length for
        // exactly this reason.
        if len > u8::MAX as usize {
            return Err(Error::InvalidArgument);
        }
        if buffer.len() < 1 + len {
            return Err(Error::BufferTooSmall);
        }
        buffer[0] = len as u8;
        buffer[1..1 + len].copy_from_slice(self.as_bytes());
        Ok(1 + len)
    }
}

impl Decode for Address {
    fn decode(data: &[u8]) -> Result<Self> {
        if data.is_empty() {
            return Err(Error::DecodingError);
        }
        let len = data[0] as usize;
        if data.len() < 1 + len {
            return Err(Error::DecodingError);
        }
        Address::from_bytes(&data[1..1 + len]).ok_or(Error::DecodingError)
    }

    fn decode_with_len(data: &[u8]) -> Result<(Self, usize)> {
        if data.is_empty() {
            return Err(Error::DecodingError);
        }
        let len = data[0] as usize;
        Ok((Self::decode(data)?, 1 + len))
    }
}

/// An account that will HOLD an asset after a call — a recipient.
///
/// # Why this is a type and not an attribute
///
/// A router that moves value along a path has to put an account somewhere in the
/// callee's argument list, and `transfer(to: Address, amount: u64)` does not say
/// which of its parameters that is. `transfer_from(from: Address, to: Address,
/// amount: u64)` has two addresses and only one of them is the destination, so
/// the TYPE `Address` cannot carry the answer either.
///
/// Declaring it by wrapping the type means the contract's own signature is the
/// declaration, the compiler stops it being attached to something that is not an
/// account, and there is no attribute to keep in sync with the parameter list.
/// The `#[infrix::contract]` macro emits `"role":"recipient"` into the
/// `infrix:schema` section for every `Recipient` parameter, and the platform's
/// router fills exactly those.
///
/// The wire form is an Address and nothing else — a one-byte length followed by
/// the account URL's bytes — so this is a compile-time and schema-time
/// distinction that costs nothing on the wire and cannot desynchronise from it.
///
/// Matrix row A.DEFI.TRANSFERHOP.RECIPIENT.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Recipient(Address);

impl Recipient {
    /// Wrap an address as the recipient of a transfer.
    pub const fn new(address: Address) -> Self {
        Self(address)
    }

    /// The account itself.
    pub fn address(&self) -> &Address {
        &self.0
    }

    /// Consume the wrapper, returning the account.
    pub fn into_address(self) -> Address {
        self.0
    }
}

impl core::ops::Deref for Recipient {
    type Target = Address;
    fn deref(&self) -> &Address {
        &self.0
    }
}

impl From<Address> for Recipient {
    fn from(a: Address) -> Self {
        Self(a)
    }
}

impl From<Recipient> for Address {
    fn from(r: Recipient) -> Self {
        r.0
    }
}

// The wire form IS Address's. A recipient that encoded differently would be a
// second address format, and the router and the callee would disagree about the
// bytes as soon as one of them was updated.
impl Encode for Recipient {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize> {
        self.0.encode(buffer)
    }
}

impl Decode for Recipient {
    fn decode(data: &[u8]) -> Result<Self> {
        Address::decode(data).map(Self)
    }

    fn decode_with_len(data: &[u8]) -> Result<(Self, usize)> {
        let (a, n) = Address::decode_with_len(data)?;
        Ok((Self(a), n))
    }
}

impl Encode for () {
    fn encode(&self, _buffer: &mut [u8]) -> Result<usize> {
        Ok(0)
    }
}

impl Decode for () {
    fn decode(_data: &[u8]) -> Result<Self> {
        Ok(())
    }

    fn decode_with_len(_data: &[u8]) -> Result<(Self, usize)> {
        Ok(((), 0))
    }
}

// =============================================================================
// Contract Traits and Types
// =============================================================================

/// Result of a contract call
#[derive(Clone, Debug)]
pub struct CallResult {
    /// Output data
    pub data: [u8; 4096],
    /// Length of output data
    pub data_len: usize,
}

impl CallResult {
    /// Create empty result
    pub fn empty() -> Self {
        Self {
            data: [0u8; 4096],
            data_len: 0,
        }
    }

    /// Create from data
    pub fn from_data(data: &[u8]) -> Self {
        let mut result = Self::empty();
        let len = core::cmp::min(data.len(), 4096);
        result.data[..len].copy_from_slice(&data[..len]);
        result.data_len = len;
        result
    }

    /// Get data as slice
    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.data_len]
    }
}

impl Default for CallResult {
    fn default() -> Self {
        Self::empty()
    }
}

/// Trait for contract instances
pub trait ContractInstance: Sized {
    /// Load contract state from storage
    fn load() -> Option<Self>;

    /// Save contract state to storage
    fn save(&self) -> Result<()>;

    /// Delete contract state from storage
    fn delete();
}

/// Trait for converting return values to Result
pub trait IntoResult<T> {
    /// Convert to Result
    fn into_result(self) -> Result<T>;
}

impl<T> IntoResult<T> for T {
    fn into_result(self) -> Result<T> {
        Ok(self)
    }
}

impl<T> IntoResult<T> for Result<T> {
    fn into_result(self) -> Result<T> {
        self
    }
}

/// Function mutability
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Mutability {
    /// View function (read-only)
    View = 0,
    /// Mutable function (can modify state)
    Mutable = 1,
    /// Payable function (can receive tokens)
    Payable = 2,
}

/// Function ABI information
#[derive(Clone, Debug)]
pub struct FunctionAbi {
    /// Function name
    pub name: &'static str,
    /// Function selector (4 bytes)
    pub selector: u32,
    /// Function mutability
    pub mutability: Mutability,
}

/// Trait for events
pub trait EventTrait {
    /// Get event signature
    fn signature() -> u32;

    /// Emit the event
    fn emit(&self) -> Result<()>;
}

impl Hash {
    /// Hash a value for event indexing
    pub fn hash_value<T: Encode>(value: &T) -> Self {
        let mut buffer = [0u8; 256];
        let len = value.encode(&mut buffer).unwrap_or(0);
        // Simple hash - in production would use actual crypto hash
        let mut hash = [0u8; 32];
        for (i, &byte) in buffer[..len].iter().enumerate() {
            hash[i % 32] ^= byte;
        }
        Hash(hash)
    }
}

/// Minimal no_std keccak256 implementation (FIPS 202 / SHA-3).
///
/// This avoids pulling in an external crate while providing correct function
/// selectors and event topics.  The implementation follows the Keccak-f[1600]
/// permutation with the SHA-3/keccak256 parameters (rate=1088, capacity=512,
/// suffix=0x01).
pub fn keccak256(input: &[u8]) -> [u8; 32] {
    const RATE: usize = 136; // 1088 bits / 8
    let mut state = [0u64; 25];

    // Absorb
    let mut offset = 0usize;
    while offset + RATE <= input.len() {
        for i in 0..(RATE / 8) {
            let word = u64::from_le_bytes([
                input[offset + i * 8],
                input[offset + i * 8 + 1],
                input[offset + i * 8 + 2],
                input[offset + i * 8 + 3],
                input[offset + i * 8 + 4],
                input[offset + i * 8 + 5],
                input[offset + i * 8 + 6],
                input[offset + i * 8 + 7],
            ]);
            state[i] ^= word;
        }
        keccak_f1600(&mut state);
        offset += RATE;
    }

    // Pad last block (keccak uses 0x01 suffix, not SHA-3's 0x06)
    let mut last_block = [0u8; RATE];
    let remaining = input.len() - offset;
    last_block[..remaining].copy_from_slice(&input[offset..]);
    last_block[remaining] = 0x01;
    last_block[RATE - 1] |= 0x80;

    for i in 0..(RATE / 8) {
        let word = u64::from_le_bytes([
            last_block[i * 8],
            last_block[i * 8 + 1],
            last_block[i * 8 + 2],
            last_block[i * 8 + 3],
            last_block[i * 8 + 4],
            last_block[i * 8 + 5],
            last_block[i * 8 + 6],
            last_block[i * 8 + 7],
        ]);
        state[i] ^= word;
    }
    keccak_f1600(&mut state);

    // Squeeze 32 bytes
    let mut out = [0u8; 32];
    for i in 0..4 {
        let bytes = state[i].to_le_bytes();
        out[i * 8..(i + 1) * 8].copy_from_slice(&bytes);
    }
    out
}

/// Keccak-f[1600] permutation (24 rounds).
fn keccak_f1600(state: &mut [u64; 25]) {
    const RC: [u64; 24] = [
        0x0000000000000001,
        0x0000000000008082,
        0x800000000000808A,
        0x8000000080008000,
        0x000000000000808B,
        0x0000000080000001,
        0x8000000080008081,
        0x8000000000008009,
        0x000000000000008A,
        0x0000000000000088,
        0x0000000080008009,
        0x000000008000000A,
        0x000000008000808B,
        0x800000000000008B,
        0x8000000000008089,
        0x8000000000008003,
        0x8000000000008002,
        0x8000000000000080,
        0x000000000000800A,
        0x800000008000000A,
        0x8000000080008081,
        0x8000000000008080,
        0x0000000080000001,
        0x8000000080008008,
    ];
    const ROT: [u32; 24] = [
        1, 3, 6, 10, 15, 21, 28, 36, 45, 55, 2, 14, 27, 41, 56, 8, 25, 43, 62, 18, 39, 61, 20, 44,
    ];
    const PI: [usize; 24] = [
        10, 7, 11, 17, 18, 3, 5, 16, 8, 21, 24, 4, 15, 23, 19, 13, 12, 2, 20, 14, 22, 9, 6, 1,
    ];

    for round in 0..24 {
        // theta
        let mut c = [0u64; 5];
        for x in 0..5 {
            c[x] = state[x] ^ state[x + 5] ^ state[x + 10] ^ state[x + 15] ^ state[x + 20];
        }
        let mut d = [0u64; 5];
        for x in 0..5 {
            d[x] = c[(x + 4) % 5] ^ c[(x + 1) % 5].rotate_left(1);
        }
        for x in 0..5 {
            for y in 0..5 {
                state[x + 5 * y] ^= d[x];
            }
        }

        // rho and pi
        let mut current = state[1];
        for i in 0..24 {
            let j = PI[i];
            let temp = state[j];
            state[j] = current.rotate_left(ROT[i]);
            current = temp;
        }

        // chi
        for y in 0..5 {
            let mut row = [0u64; 5];
            for x in 0..5 {
                row[x] = state[x + 5 * y];
            }
            for x in 0..5 {
                state[x + 5 * y] = row[x] ^ (!row[(x + 1) % 5] & row[(x + 2) % 5]);
            }
        }

        // iota
        state[0] ^= RC[round];
    }
}

/// Governance type definitions — intents, plans, outcomes, approvals,
/// trust profiles, capabilities, roles, settlements, escrows, evidence,
/// anchors, and disclosures.
pub mod governance;
pub use governance::*;

/// ABI encoding helpers
pub mod abi {
    use super::*;

    /// Encode a u64 value
    pub fn encode_u64(value: u64) -> [u8; 32] {
        let mut result = [0u8; 32];
        result[24..32].copy_from_slice(&value.to_be_bytes());
        result
    }

    /// Decode a u64 value
    pub fn decode_u64(data: &[u8]) -> Option<u64> {
        if data.len() < 32 {
            return None;
        }
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&data[24..32]);
        Some(u64::from_be_bytes(bytes))
    }

    /// Encode a U256 value
    pub fn encode_u256(value: &U256) -> [u8; 32] {
        value.to_be_bytes()
    }

    /// Decode a U256 value
    pub fn decode_u256(data: &[u8]) -> Option<U256> {
        if data.len() < 32 {
            return None;
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&data[..32]);
        Some(U256::from_be_bytes(&bytes))
    }

    /// Encode a bool value
    pub fn encode_bool(value: bool) -> [u8; 32] {
        let mut result = [0u8; 32];
        result[31] = if value { 1 } else { 0 };
        result
    }

    /// Decode a bool value
    pub fn decode_bool(data: &[u8]) -> Option<bool> {
        if data.len() < 32 {
            return None;
        }
        Some(data[31] != 0)
    }

    /// Encode bytes32
    pub fn encode_bytes32(value: &[u8; 32]) -> [u8; 32] {
        *value
    }

    /// Compute function selector (first 4 bytes of keccak256).
    ///
    /// Uses a minimal no_std keccak256 implementation so that selectors are
    /// computed correctly without requiring an external crate.
    pub fn function_selector(signature: &str) -> [u8; 4] {
        let hash = keccak256(signature.as_bytes());
        [hash[0], hash[1], hash[2], hash[3]]
    }

    /// Compute event topic (keccak256 of event signature).
    pub fn event_topic(signature: &str) -> Topic {
        Topic(keccak256(signature.as_bytes()))
    }
}

/// Byte utilities
pub mod bytes {
    /// Copy bytes with length prefix
    pub fn copy_with_len(src: &[u8], dst: &mut [u8]) -> usize {
        let len = core::cmp::min(src.len(), dst.len().saturating_sub(4));
        dst[..4].copy_from_slice(&(len as u32).to_le_bytes());
        dst[4..4 + len].copy_from_slice(&src[..len]);
        4 + len
    }

    /// Read bytes with length prefix
    pub fn read_with_len(src: &[u8]) -> Option<&[u8]> {
        if src.len() < 4 {
            return None;
        }
        let mut len_bytes = [0u8; 4];
        len_bytes.copy_from_slice(&src[..4]);
        let len = u32::from_le_bytes(len_bytes) as usize;
        if src.len() < 4 + len {
            return None;
        }
        Some(&src[4..4 + len])
    }

    /// Pad bytes to 32 bytes (right-padded)
    pub fn pad_right_32(src: &[u8]) -> [u8; 32] {
        let mut result = [0u8; 32];
        let len = core::cmp::min(src.len(), 32);
        result[..len].copy_from_slice(&src[..len]);
        result
    }

    /// Pad bytes to 32 bytes (left-padded)
    pub fn pad_left_32(src: &[u8]) -> [u8; 32] {
        let mut result = [0u8; 32];
        let len = core::cmp::min(src.len(), 32);
        result[32 - len..].copy_from_slice(&src[..len]);
        result
    }
}

#[cfg(test)]
mod tests {

    /// Matrix row A.ABI.CODEC.DECLAREDTYPES.
    ///
    /// A signed integer is two's complement, big-endian, at its own width — the
    /// same shape as its unsigned twin. Before 2026-09-05 a contract declaring
    /// one did not FAIL to encode, it failed to BUILD: no Decode existed, so
    /// `#[call] fn f(&mut self, delta: i64)` stopped at
    /// "the trait bound i64: Decode is not satisfied".
    #[test]
    fn test_signed_integers_round_trip_at_their_own_width() {
        fn round<T>(v: T, width: usize)
        where
            T: Encode + Decode + PartialEq + core::fmt::Debug + Copy,
        {
            let mut buf = [0u8; 32];
            let n = v.encode(&mut buf).expect("encode");
            assert_eq!(n, width, "encoded to the wrong width");
            let (back, consumed) = T::decode_with_len(&buf[..n]).expect("decode");
            assert_eq!(back, v);
            assert_eq!(
                consumed, width,
                "decode_with_len must report the declared width"
            );
        }
        for v in [0i8, 1, -1, i8::MIN, i8::MAX] {
            round(v, 1);
        }
        for v in [0i16, 1, -1, i16::MIN, i16::MAX] {
            round(v, 2);
        }
        for v in [0i32, 1, -1, i32::MIN, i32::MAX] {
            round(v, 4);
        }
        for v in [0i64, 1, -1, i64::MIN, i64::MAX] {
            round(v, 8);
        }
        for v in [0i128, 1, -1, i128::MIN, i128::MAX] {
            round(v, 16);
        }

        // Big-endian, and negative values are two's complement — not a sign
        // byte and not little-endian.
        let mut buf = [0u8; 8];
        (-2i32).encode(&mut buf).expect("encode");
        assert_eq!(&buf[..4], &[0xff, 0xff, 0xff, 0xfe]);
        (258i32).encode(&mut buf).expect("encode");
        assert_eq!(&buf[..4], &[0x00, 0x00, 0x01, 0x02]);
    }

    /// A signed value must NOT be readable as its unsigned twin's bytes by
    /// accident: the widths match, so the discipline that keeps them apart is
    /// the declaration, and this pins the bytes each produces.
    #[test]
    fn test_a_signed_value_and_its_unsigned_twin_agree_on_width_only() {
        let mut a = [0u8; 8];
        let mut b = [0u8; 8];
        assert_eq!(1i64.encode(&mut a).unwrap(), 1u64.encode(&mut b).unwrap());
        assert_eq!(
            a, b,
            "a positive value encodes identically in both signednesses"
        );
        (-1i64).encode(&mut a).unwrap();
        u64::MAX.encode(&mut b).unwrap();
        assert_eq!(
            a, b,
            "-1 and u64::MAX share a bit pattern; only the declaration separates them"
        );
    }

    #[test]
    fn test_bytes20_is_fixed_width_and_carries_no_prefix() {
        let v: [u8; 20] = [7u8; 20];
        let mut buf = [0u8; 32];
        let n = v.encode(&mut buf).expect("encode");
        assert_eq!(n, 20, "a fixed-width type carries no length prefix");
        assert_eq!(&buf[..20], &v);
        let (back, consumed) = <[u8; 20]>::decode_with_len(&buf[..n]).expect("decode");
        assert_eq!(back, v);
        assert_eq!(consumed, 20);
        assert!(
            <[u8; 20]>::decode(&buf[..19]).is_err(),
            "a short buffer must not decode"
        );
    }

    /// String and Vec<u8> carry a BIG-ENDIAN u32 length. Address's one-byte
    /// prefix is permanent because deployed contracts speak it; it is also the
    /// wrong choice for a value whose size the type does not bound, and this
    /// pins the wider prefix so it cannot be quietly narrowed to match.
    #[test]
    fn test_variable_length_values_carry_a_four_byte_length() {
        let s = String::from("hello");
        let mut buf = [0u8; 64];
        let n = s.encode(&mut buf).expect("encode");
        assert_eq!(n, 4 + 5);
        assert_eq!(&buf[..4], &[0, 0, 0, 5], "the length is a big-endian u32");
        assert_eq!(&buf[4..9], b"hello");
        let (back, consumed) = String::decode_with_len(&buf[..n]).expect("decode");
        assert_eq!(back, s);
        assert_eq!(consumed, n);

        let v: Vec<u8> = Vec::from([1u8, 2, 3]);
        let n = v.encode(&mut buf).expect("encode");
        assert_eq!(n, 4 + 3);
        assert_eq!(&buf[..4], &[0, 0, 0, 3]);
        let (back, consumed) = Vec::<u8>::decode_with_len(&buf[..n]).expect("decode");
        assert_eq!(back, v);
        assert_eq!(consumed, n);

        // Empty is a legal value, and distinct from absent.
        let empty = String::new();
        let n = empty.encode(&mut buf).expect("encode");
        assert_eq!(n, 4);
        assert_eq!(String::decode(&buf[..n]).expect("decode"), "");
    }

    /// A variable-length value leaves the decoder exactly where the next
    /// parameter begins — the property the generated dispatcher's
    /// `offset += consumed` depends on.
    #[test]
    fn test_a_string_leaves_the_decoder_positioned_for_the_next_parameter() {
        let mut buf = [0u8; 64];
        let label = String::from("abcd");
        let n = label.encode(&mut buf).expect("encode");
        let amount: u64 = 7;
        let m = amount.encode(&mut buf[n..]).expect("encode");

        let (got_label, consumed) = String::decode_with_len(&buf[..n + m]).expect("decode");
        assert_eq!(got_label, "abcd");
        let (got_amount, consumed_amount) =
            u64::decode_with_len(&buf[consumed..n + m]).expect("decode");
        assert_eq!(got_amount, 7, "the amount was read from the wrong offset");
        assert_eq!(consumed + consumed_amount, n + m);
    }

    #[test]
    fn test_variable_length_decoding_refuses_rather_than_guesses() {
        // A length the buffer cannot satisfy.
        assert!(String::decode(&[0, 0, 0, 8, b'a']).is_err());
        assert!(Vec::<u8>::decode(&[0, 0, 0, 8, 1]).is_err());
        // A prefix that is not even complete.
        assert!(String::decode(&[0, 0]).is_err());
        // A length past the sanity bound is not taken at face value.
        assert!(String::decode(&[0xff, 0xff, 0xff, 0xff]).is_err());
        // Bytes that are not UTF-8 are not a String. Refused, not converted
        // lossily: the contract would otherwise act on characters nobody sent.
        assert!(String::decode(&[0, 0, 0, 1, 0xff]).is_err());
        // The same bytes ARE a legal Vec<u8>.
        assert_eq!(
            Vec::<u8>::decode(&[0, 0, 0, 1, 0xff]).expect("decode"),
            Vec::from([0xffu8])
        );
        // A buffer too small to encode into fails closed rather than truncating.
        let mut tiny = [0u8; 4];
        assert!(String::from("hello").encode(&mut tiny).is_err());
    }

    /// `&str` and `&[u8]` encode exactly as their owned forms, so a contract
    /// can return a borrowed value without an allocation and a caller cannot
    /// tell the two apart on the wire.
    #[test]
    fn test_borrowed_values_encode_as_their_owned_forms() {
        let mut a = [0u8; 64];
        let mut b = [0u8; 64];
        let n = "hello".encode(&mut a).expect("encode &str");
        let m = String::from("hello").encode(&mut b).expect("encode String");
        assert_eq!((n, &a[..n]), (m, &b[..m]));

        let slice: &[u8] = &[1, 2, 3];
        let n = slice.encode(&mut a).expect("encode &[u8]");
        let m = Vec::from([1u8, 2, 3])
            .encode(&mut b)
            .expect("encode Vec<u8>");
        assert_eq!((n, &a[..n]), (m, &b[..m]));
    }

    /// The Address prefix is one byte. Before MAX_LEN was corrected to 255, an
    /// address of exactly 256 bytes encoded its length as 0 and decoded as EMPTY —
    /// a different address, silently. It is refused now.
    #[test]
    fn test_an_address_too_long_for_its_prefix_is_refused_not_truncated() {
        // It is refused at CONSTRUCTION, which is where the truncation was: the
        // length field is a u8, so `256 as u8` was 0 and every caller saw an
        // empty address.
        assert!(
            Address::from_bytes(&[b'a'; 256]).is_none(),
            "a 256-byte address was constructed; its length field would hold 0"
        );
        let mut buf = [0u8; 512];
        // 255 is the largest that round-trips.
        let ok = Address::from_bytes(&[b'a'; 255]).expect("255-byte address");
        let n = ok.encode(&mut buf).expect("encode");
        assert_eq!(n, 1 + 255);
        assert_eq!(
            Address::decode(&buf[..n]).expect("decode").as_bytes(),
            &[b'a'; 255]
        );
    }

    /// A Recipient must be byte-for-byte an Address on the wire. The
    /// distinction it draws is for the SCHEMA — which parameter a router may
    /// fill with a destination — and a wire form of its own would mean a caller
    /// that does not know about roles sends bytes the callee cannot decode.
    ///
    /// Matrix row A.DEFI.TRANSFERHOP.RECIPIENT.
    #[test]
    fn test_recipient_encodes_exactly_as_an_address() {
        let url = b"acc://alice.acme/tokens";
        let addr = Address::from_bytes(url).expect("address");
        let recipient = Recipient::new(addr.clone());

        let mut a = [0u8; 512];
        let mut r = [0u8; 512];
        let na = addr.encode(&mut a).expect("encode address");
        let nr = recipient.encode(&mut r).expect("encode recipient");
        assert_eq!(
            na, nr,
            "a recipient encodes to a different length than its address"
        );
        assert_eq!(
            &a[..na],
            &r[..nr],
            "a recipient is not byte-identical to its address"
        );

        // And the wire form is the one the platform emits: a one-byte length
        // followed by the URL.
        assert_eq!(r[0] as usize, url.len());
        assert_eq!(&r[1..1 + url.len()], url);

        let (back, consumed) = Recipient::decode_with_len(&r[..nr]).expect("decode");
        assert_eq!(
            consumed, nr,
            "decode_with_len must report the whole address"
        );
        assert_eq!(back.address().as_bytes(), url);
        assert_eq!(Address::from(back).as_bytes(), url);
    }

    /// decode_with_len is what the generated dispatcher advances `offset` by, so
    /// a recipient followed by another parameter must leave the reader in
    /// exactly the right place.
    #[test]
    fn test_a_recipient_leaves_the_decoder_positioned_for_the_next_parameter() {
        let url = b"acc://venue.acme/pool";
        let recipient = Recipient::new(Address::from_bytes(url).expect("address"));
        let mut buf = [0u8; 512];
        let n = recipient.encode(&mut buf).expect("encode");
        let amount: u64 = 1_000;
        let m = amount.encode(&mut buf[n..]).expect("encode amount");

        let (got_to, consumed) = Recipient::decode_with_len(&buf[..n + m]).expect("decode to");
        assert_eq!(got_to.address().as_bytes(), url);
        let (got_amount, consumed_amount) =
            u64::decode_with_len(&buf[consumed..n + m]).expect("decode amount");
        assert_eq!(
            got_amount, amount,
            "the amount was read from the wrong offset"
        );
        assert_eq!(consumed + consumed_amount, n + m);
    }

    /// The ABI's result codes are a published wire contract: every contract
    /// already deployed answers these numbers, and hosts read them to tell an
    /// unrecognised selector apart from a domain refusal. Renumbering a variant
    /// silently re-labels those contracts' refusals, so the whole table is
    /// pinned here by literal.
    ///
    /// Appending a variant is safe and this test says so explicitly: add the
    /// new pair below. Changing an existing pair is not, and requires a
    /// coordinated release across every SDK and host.
    #[test]
    fn test_result_codes_are_a_pinned_wire_contract() {
        let registry: &[(Error, u32)] = &[
            (Error::Success, 0),
            (Error::InvalidArgument, 1),
            (Error::OutOfMemory, 2),
            (Error::OutOfGas, 3),
            (Error::StorageError, 4),
            (Error::Unauthorized, 5),
            (Error::AccountNotFound, 6),
            (Error::InsufficientBalance, 7),
            (Error::InsufficientCredits, 8),
            (Error::ContractNotFound, 9),
            (Error::FunctionNotFound, 10),
            (Error::InvalidState, 11),
            (Error::Overflow, 12),
            (Error::Underflow, 13),
            (Error::DivisionByZero, 14),
            (Error::InvalidSignature, 15),
            (Error::Revert, 16),
            (Error::Panic, 17),
            (Error::CallDepthExceeded, 18),
            (Error::ReadOnlyViolation, 19),
            (Error::ReentrancyDetected, 20),
            (Error::NotPayable, 21),
            (Error::ZeroAddress, 22),
            (Error::ContractNotInitialized, 23),
            (Error::InvalidInput, 24),
            (Error::UnknownFunction, 25),
            (Error::EncodingError, 26),
            (Error::DecodingError, 27),
            (Error::BufferTooSmall, 28),
            (Error::GovernanceError, 29),
            (Error::ApprovalRequired, 30),
            (Error::CapabilityDenied, 31),
            (Error::RoleRequired, 32),
            (Error::PolicyDenied, 33),
            (Error::Unknown, 255),
        ];
        assert_eq!(
            registry.len(),
            35,
            "a variant was added or removed without updating the pinned table"
        );
        for (variant, code) in registry {
            assert_eq!(
                *variant as u32, *code,
                "{:?} is discriminant {} but the wire contract pins it at {}; every contract                  already deployed answers the pinned number",
                variant, *variant as u32, code
            );
        }
    }

    /// The dispatcher's failure arm returns `result_code()`, so this is the
    /// exact value a host reads off the wire.
    #[test]
    fn test_result_code_is_the_negated_discriminant() {
        assert_eq!(Error::UnknownFunction.result_code(), -25);
        assert_eq!(Error::InsufficientBalance.result_code(), -7);
        assert_eq!(Error::PolicyDenied.result_code(), -33);
        assert_eq!(Error::Unknown.result_code(), -255);
        assert_eq!(
            Error::Success.result_code(),
            0,
            "success is a zero-byte result, not a refusal"
        );
    }

    /// A code round-trips, a byte count is not a refusal, and a number the
    /// registry does not define is reported as unrecognised rather than
    /// collapsed onto `Unknown`.
    #[test]
    fn test_from_result_code_reads_only_what_the_registry_defines() {
        for code in [0u32, 1, 7, 11, 24, 25, 33, 255] {
            let variant = Error::from_code(code);
            if code == 0 {
                assert_eq!(Error::from_result_code(variant.result_code()), None);
                continue;
            }
            assert_eq!(
                Error::from_result_code(variant.result_code()),
                Some(variant),
                "code {} did not round-trip through the wire representation",
                code
            );
        }
        assert_eq!(
            Error::from_result_code(8),
            None,
            "a positive return is a byte count"
        );
        assert_eq!(
            Error::from_result_code(0),
            None,
            "zero bytes written is not a refusal"
        );
        assert_eq!(
            Error::from_result_code(-4242),
            None,
            "a number the registry does not define must be reported as unrecognised, never              borrowed onto Unknown"
        );
        assert_eq!(
            Error::from_result_code(-34),
            None,
            "34 is unallocated; if a variant is ever given it, this line must be updated              deliberately"
        );
    }
    use super::*;

    #[test]
    fn test_u256_basic() {
        let zero = U256::zero();
        let one = U256::one();
        assert!(zero.is_zero());
        assert!(!one.is_zero());
        assert!(zero < one);
    }

    #[test]
    fn test_u256_arithmetic() {
        let a = U256::from(100u64);
        let b = U256::from(50u64);
        let sum = a.checked_add(&b).unwrap();
        let diff = a.checked_sub(&b).unwrap();
        assert_eq!(sum.low, 150);
        assert_eq!(diff.low, 50);
    }

    #[test]
    fn test_u256_bytes() {
        let val = U256::from(0x1234567890abcdefu64);
        let be = val.to_be_bytes();
        let le = val.to_le_bytes();
        let from_be = U256::from_be_bytes(&be);
        let from_le = U256::from_le_bytes(&le);
        assert_eq!(val, from_be);
        assert_eq!(val, from_le);
    }

    #[test]
    fn test_hash() {
        let zero = Hash::zero();
        assert!(zero.is_zero());

        let h = Hash::from([1u8; 32]);
        assert!(!h.is_zero());
        assert_eq!(h.as_bytes()[0], 1);
    }

    #[test]
    fn test_address() {
        let addr = Address::from_str("acc://example.acme/tokens").unwrap();
        assert_eq!(addr.adi(), Some("example.acme"));
        assert_eq!(addr.path(), Some("/tokens"));
    }

    #[test]
    fn test_error() {
        let err = Error::InsufficientBalance;
        assert!(err.is_error());
        assert_eq!(err.code(), 7);
        assert_eq!(Error::from_code(7), Error::InsufficientBalance);
    }
}

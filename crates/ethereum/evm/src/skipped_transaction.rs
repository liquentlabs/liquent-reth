//! Protocol encoding for transactions skipped during Liquent block execution.

use alloy_consensus::TxType;
use alloy_primitives::{Log, B256};
use alloy_sol_types::{sol, SolEvent};
use levm::InvalidTransaction;
use reth_chainspec::LIQUENT_TX_SKIPPED_LOG_ADDRESS;
use reth_ethereum_primitives::Receipt;

sol! {
    /// Emitted in the synthetic receipt of an included transaction that was invalid at final
    /// execution state and therefore applied as a no-op.
    event LiquentTxSkipped(uint16 indexed version, uint16 indexed reason);
}

/// Current version of the [`LiquentTxSkipped`] event encoding.
pub const LIQUENT_TX_SKIPPED_LOG_VERSION: u16 = 1;

/// `keccak256("LiquentTxSkipped(uint16,uint16)")`, derived from the Solidity declaration.
pub const LIQUENT_TX_SKIPPED_LOG_TOPIC0: B256 = LiquentTxSkipped::SIGNATURE_HASH;

macro_rules! define_skip_reasons {
    (
        active {
            $($variant:ident = $code:literal => $pattern:pat),+ $(,)?
        }
        reserved {
            $($reserved_variant:ident = $reserved_code:literal),* $(,)?
        }
    ) => {
        /// The stable wire tag for a skipped [`InvalidTransaction`].
        ///
        /// The initial assignments follow revm's declaration order. Once assigned, a code must
        /// never be changed or reused: new variants are appended and removed variants are moved
        /// to the `reserved` section of the reason table.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(u16)]
        pub enum LiquentTxSkipReason {
            $(
                #[doc = concat!("Corresponds to [`InvalidTransaction::", stringify!($variant), "`].")]
                $variant = $code,
            )+
            $(
                #[doc = concat!("Reserved historical reason code for `", stringify!($reserved_variant), "`.")]
                $reserved_variant = $reserved_code,
            )*
        }

        impl LiquentTxSkipReason {
            /// Returns the wire tag for `error`.
            pub const fn from_invalid_transaction(error: &InvalidTransaction) -> Self {
                match error {
                    $($pattern => Self::$variant,)+
                }
            }

            /// Parses a wire tag.
            pub const fn from_code(code: u16) -> Option<Self> {
                match code {
                    $($code => Some(Self::$variant),)+
                    $($reserved_code => Some(Self::$reserved_variant),)*
                    _ => None,
                }
            }
        }
    };
}

define_skip_reasons! {
    active {
        PriorityFeeGreaterThanMaxFee = 0 => InvalidTransaction::PriorityFeeGreaterThanMaxFee,
        GasPriceLessThanBasefee = 1 => InvalidTransaction::GasPriceLessThanBasefee,
        CallerGasLimitMoreThanBlock = 2 => InvalidTransaction::CallerGasLimitMoreThanBlock,
        CallGasCostMoreThanGasLimit = 3 => InvalidTransaction::CallGasCostMoreThanGasLimit { .. },
        GasFloorMoreThanGasLimit = 4 => InvalidTransaction::GasFloorMoreThanGasLimit { .. },
        RejectCallerWithCode = 5 => InvalidTransaction::RejectCallerWithCode,
        LackOfFundForMaxFee = 6 => InvalidTransaction::LackOfFundForMaxFee { .. },
        OverflowPaymentInTransaction = 7 => InvalidTransaction::OverflowPaymentInTransaction,
        NonceOverflowInTransaction = 8 => InvalidTransaction::NonceOverflowInTransaction,
        NonceTooHigh = 9 => InvalidTransaction::NonceTooHigh { .. },
        NonceTooLow = 10 => InvalidTransaction::NonceTooLow { .. },
        CreateInitCodeSizeLimit = 11 => InvalidTransaction::CreateInitCodeSizeLimit,
        InvalidChainId = 12 => InvalidTransaction::InvalidChainId,
        MissingChainId = 13 => InvalidTransaction::MissingChainId,
        TxGasLimitGreaterThanCap = 14 => InvalidTransaction::TxGasLimitGreaterThanCap { .. },
        AccessListNotSupported = 15 => InvalidTransaction::AccessListNotSupported,
        MaxFeePerBlobGasNotSupported = 16 => InvalidTransaction::MaxFeePerBlobGasNotSupported,
        BlobVersionedHashesNotSupported = 17 => InvalidTransaction::BlobVersionedHashesNotSupported,
        BlobGasPriceGreaterThanMax = 18 => InvalidTransaction::BlobGasPriceGreaterThanMax { .. },
        EmptyBlobs = 19 => InvalidTransaction::EmptyBlobs,
        BlobCreateTransaction = 20 => InvalidTransaction::BlobCreateTransaction,
        TooManyBlobs = 21 => InvalidTransaction::TooManyBlobs { .. },
        BlobVersionNotSupported = 22 => InvalidTransaction::BlobVersionNotSupported,
        AuthorizationListNotSupported = 23 => InvalidTransaction::AuthorizationListNotSupported,
        AuthorizationListInvalidFields = 24 => InvalidTransaction::AuthorizationListInvalidFields,
        EmptyAuthorizationList = 25 => InvalidTransaction::EmptyAuthorizationList,
        Eip2930NotSupported = 26 => InvalidTransaction::Eip2930NotSupported,
        Eip1559NotSupported = 27 => InvalidTransaction::Eip1559NotSupported,
        Eip4844NotSupported = 28 => InvalidTransaction::Eip4844NotSupported,
        Eip7702NotSupported = 29 => InvalidTransaction::Eip7702NotSupported,
        Eip7873NotSupported = 30 => InvalidTransaction::Eip7873NotSupported,
        Eip7873MissingTarget = 31 => InvalidTransaction::Eip7873MissingTarget,
        Str = 32 => InvalidTransaction::Str(_),
    }
    reserved {}
}

/// Encoder and decoder for Liquent's synthetic skipped-transaction receipt.
#[derive(Debug, Clone, Copy, Default)]
pub struct LiquentTxSkippedEvent;

impl LiquentTxSkippedEvent {
    /// Creates the protocol log for `reason`.
    pub fn encode(reason: LiquentTxSkipReason) -> Log {
        Log {
            address: LIQUENT_TX_SKIPPED_LOG_ADDRESS,
            data: LiquentTxSkipped {
                version: LIQUENT_TX_SKIPPED_LOG_VERSION,
                reason: reason as u16,
            }
            .encode_log_data(),
        }
    }

    /// Parses a protocol log, rejecting a wrong emitter, ABI shape, version, or unknown reason.
    pub fn decode(log: &Log) -> Option<LiquentTxSkipReason> {
        if log.address != LIQUENT_TX_SKIPPED_LOG_ADDRESS {
            return None
        }

        let event = LiquentTxSkipped::decode_log(log).ok()?;
        if event.version != LIQUENT_TX_SKIPPED_LOG_VERSION {
            return None
        }

        LiquentTxSkipReason::from_code(event.reason)
    }

    /// Builds the zero-gas failed receipt for a skipped transaction.
    pub fn receipt(
        tx_type: TxType,
        cumulative_gas_used: u64,
        reason: LiquentTxSkipReason,
    ) -> Receipt {
        Receipt {
            tx_type,
            success: false,
            cumulative_gas_used,
            logs: alloc::vec![Self::encode(reason)],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{borrow::Cow, boxed::Box, vec, vec::Vec};
    use alloy_primitives::{keccak256, Address, LogData, U256};

    #[test]
    fn topic_is_derived_from_the_documented_solidity_signature() {
        assert_eq!(LIQUENT_TX_SKIPPED_LOG_TOPIC0, keccak256("LiquentTxSkipped(uint16,uint16)"));
    }

    #[test]
    fn all_active_reason_codes_are_stable() {
        for (error, expected_reason, expected_code) in stable_reason_cases() {
            let reason = LiquentTxSkipReason::from_invalid_transaction(&error);
            assert_eq!(reason, expected_reason);
            assert_eq!(reason as u16, expected_code);

            let log = LiquentTxSkippedEvent::encode(reason);
            assert_eq!(LiquentTxSkippedEvent::decode(&log), Some(reason));
            assert!(log.data.data.is_empty());
        }
    }

    #[test]
    fn str_message_does_not_affect_consensus_encoding() {
        let first = InvalidTransaction::Str(Cow::Borrowed("first diagnostic"));
        let second = InvalidTransaction::Str(Cow::Borrowed("changed diagnostic"));
        let first_log =
            LiquentTxSkippedEvent::encode(LiquentTxSkipReason::from_invalid_transaction(&first));
        let second_log =
            LiquentTxSkippedEvent::encode(LiquentTxSkipReason::from_invalid_transaction(&second));
        assert_eq!(first_log, second_log);
    }

    #[test]
    fn event_decode_rejects_non_protocol_logs_and_unknown_tags() {
        let log = LiquentTxSkippedEvent::encode(LiquentTxSkipReason::NonceTooLow);
        let mut wrong_emitter = log;
        wrong_emitter.address = Address::ZERO;
        assert_eq!(LiquentTxSkippedEvent::decode(&wrong_emitter), None);

        let unknown_reason = Log {
            address: LIQUENT_TX_SKIPPED_LOG_ADDRESS,
            data: LiquentTxSkipped { version: LIQUENT_TX_SKIPPED_LOG_VERSION, reason: u16::MAX }
                .encode_log_data(),
        };
        assert_eq!(LiquentTxSkippedEvent::decode(&unknown_reason), None);

        let malformed = Log {
            address: LIQUENT_TX_SKIPPED_LOG_ADDRESS,
            data: LogData::new_unchecked(
                alloc::vec![LIQUENT_TX_SKIPPED_LOG_TOPIC0],
                Default::default(),
            ),
        };
        assert_eq!(LiquentTxSkippedEvent::decode(&malformed), None);
    }

    fn stable_reason_cases() -> Vec<(InvalidTransaction, LiquentTxSkipReason, u16)> {
        vec![
            (
                InvalidTransaction::PriorityFeeGreaterThanMaxFee,
                LiquentTxSkipReason::PriorityFeeGreaterThanMaxFee,
                0,
            ),
            (
                InvalidTransaction::GasPriceLessThanBasefee,
                LiquentTxSkipReason::GasPriceLessThanBasefee,
                1,
            ),
            (
                InvalidTransaction::CallerGasLimitMoreThanBlock,
                LiquentTxSkipReason::CallerGasLimitMoreThanBlock,
                2,
            ),
            (
                InvalidTransaction::CallGasCostMoreThanGasLimit {
                    initial_gas: 21_001,
                    gas_limit: 21_000,
                },
                LiquentTxSkipReason::CallGasCostMoreThanGasLimit,
                3,
            ),
            (
                InvalidTransaction::GasFloorMoreThanGasLimit {
                    gas_floor: 30_000,
                    gas_limit: 21_000,
                },
                LiquentTxSkipReason::GasFloorMoreThanGasLimit,
                4,
            ),
            (
                InvalidTransaction::RejectCallerWithCode,
                LiquentTxSkipReason::RejectCallerWithCode,
                5,
            ),
            (
                InvalidTransaction::LackOfFundForMaxFee {
                    fee: Box::new(U256::from(100)),
                    balance: Box::new(U256::from(99)),
                },
                LiquentTxSkipReason::LackOfFundForMaxFee,
                6,
            ),
            (
                InvalidTransaction::OverflowPaymentInTransaction,
                LiquentTxSkipReason::OverflowPaymentInTransaction,
                7,
            ),
            (
                InvalidTransaction::NonceOverflowInTransaction,
                LiquentTxSkipReason::NonceOverflowInTransaction,
                8,
            ),
            (
                InvalidTransaction::NonceTooHigh { tx: 2, state: 1 },
                LiquentTxSkipReason::NonceTooHigh,
                9,
            ),
            (
                InvalidTransaction::NonceTooLow { tx: 0, state: 1 },
                LiquentTxSkipReason::NonceTooLow,
                10,
            ),
            (
                InvalidTransaction::CreateInitCodeSizeLimit,
                LiquentTxSkipReason::CreateInitCodeSizeLimit,
                11,
            ),
            (InvalidTransaction::InvalidChainId, LiquentTxSkipReason::InvalidChainId, 12),
            (InvalidTransaction::MissingChainId, LiquentTxSkipReason::MissingChainId, 13),
            (
                InvalidTransaction::TxGasLimitGreaterThanCap { gas_limit: 31_000, cap: 30_000 },
                LiquentTxSkipReason::TxGasLimitGreaterThanCap,
                14,
            ),
            (
                InvalidTransaction::AccessListNotSupported,
                LiquentTxSkipReason::AccessListNotSupported,
                15,
            ),
            (
                InvalidTransaction::MaxFeePerBlobGasNotSupported,
                LiquentTxSkipReason::MaxFeePerBlobGasNotSupported,
                16,
            ),
            (
                InvalidTransaction::BlobVersionedHashesNotSupported,
                LiquentTxSkipReason::BlobVersionedHashesNotSupported,
                17,
            ),
            (
                InvalidTransaction::BlobGasPriceGreaterThanMax {
                    block_blob_gas_price: 2,
                    tx_max_fee_per_blob_gas: 1,
                },
                LiquentTxSkipReason::BlobGasPriceGreaterThanMax,
                18,
            ),
            (InvalidTransaction::EmptyBlobs, LiquentTxSkipReason::EmptyBlobs, 19),
            (
                InvalidTransaction::BlobCreateTransaction,
                LiquentTxSkipReason::BlobCreateTransaction,
                20,
            ),
            (
                InvalidTransaction::TooManyBlobs { max: 6, have: 7 },
                LiquentTxSkipReason::TooManyBlobs,
                21,
            ),
            (
                InvalidTransaction::BlobVersionNotSupported,
                LiquentTxSkipReason::BlobVersionNotSupported,
                22,
            ),
            (
                InvalidTransaction::AuthorizationListNotSupported,
                LiquentTxSkipReason::AuthorizationListNotSupported,
                23,
            ),
            (
                InvalidTransaction::AuthorizationListInvalidFields,
                LiquentTxSkipReason::AuthorizationListInvalidFields,
                24,
            ),
            (
                InvalidTransaction::EmptyAuthorizationList,
                LiquentTxSkipReason::EmptyAuthorizationList,
                25,
            ),
            (InvalidTransaction::Eip2930NotSupported, LiquentTxSkipReason::Eip2930NotSupported, 26),
            (InvalidTransaction::Eip1559NotSupported, LiquentTxSkipReason::Eip1559NotSupported, 27),
            (InvalidTransaction::Eip4844NotSupported, LiquentTxSkipReason::Eip4844NotSupported, 28),
            (InvalidTransaction::Eip7702NotSupported, LiquentTxSkipReason::Eip7702NotSupported, 29),
            (InvalidTransaction::Eip7873NotSupported, LiquentTxSkipReason::Eip7873NotSupported, 30),
            (
                InvalidTransaction::Eip7873MissingTarget,
                LiquentTxSkipReason::Eip7873MissingTarget,
                31,
            ),
            (
                InvalidTransaction::Str(Cow::Borrowed("custom invalid transaction")),
                LiquentTxSkipReason::Str,
                32,
            ),
        ]
    }
}

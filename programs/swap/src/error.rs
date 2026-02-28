use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Offer amounts must be greater than zero")]
    InvalidAmount,
    #[msg("Give and want mints must be different")]
    SameMintNotAllowed,
    #[msg("Offer has an invalid status value")]
    InvalidStatus,
    #[msg("Provided offer id does not match offer account state")]
    OfferIdMismatch,
    #[msg("Offer is not open")]
    OfferNotOpen,
    #[msg("Provided mint account does not match offer state")]
    MintMismatch,
}

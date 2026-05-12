use anchor_lang::prelude::*;

#[error_code]
pub enum HoldemError {
    #[msg("Table already has two players")]
    TableFull,
    #[msg("Player is already seated at this table")]
    AlreadySeated,
    #[msg("Not your turn to act")]
    NotYourTurn,
    #[msg("Invalid game state for this instruction")]
    InvalidState,
    #[msg("Insufficient stack for this action")]
    InsufficientStack,
    #[msg("Raise amount must exceed the current bet by at least one big blind")]
    InvalidRaise,
    #[msg("Game must be in Showdown state to reveal cards")]
    NotShowdown,
    #[msg("Arcium cluster is not configured")]
    ClusterNotSet,
    #[msg("Buy-in must be at least twice the big blind")]
    InvalidBuyIn,
    #[msg("Big blind must be greater than zero")]
    InvalidBlind,
    #[msg("Cannot check: there is an outstanding bet — call or fold")]
    CannotCheck,
    #[msg("Call amount exceeds stack — go all-in with submit_allin")]
    CallExceedsStack,
    #[msg("Mask does not match the committed hash")]
    InvalidMaskReveal,
    #[msg("Both players must reveal before the pot can be settled")]
    AwaitingOtherReveal,
}

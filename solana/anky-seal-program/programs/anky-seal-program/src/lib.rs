#![allow(unexpected_cfgs)]

use anchor_lang::{
    prelude::*,
    solana_program::{hash::hashv, pubkey},
};

declare_id!("4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX");

pub const LOOM_STATE_SEED: &[u8] = b"loom_state";
pub const ROLLING_ROOT_DOMAIN: &[u8] = b"ANKY_LOOM_ROOT_V1";
pub const CORE_KEY_ASSET_V1: u8 = 1;
pub const CORE_KEY_COLLECTION_V1: u8 = 5;
pub const CORE_UPDATE_AUTHORITY_COLLECTION: u8 = 2;

pub const METAPLEX_CORE_PROGRAM_ID: Pubkey =
    pubkey!("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d");

pub const OFFICIAL_COLLECTION: Pubkey = pubkey!("F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u");

#[program]
pub mod anky_seal_program {
    use super::*;

    pub fn seal_anky(ctx: Context<SealAnky>, session_hash: [u8; 32]) -> Result<()> {
        let clock = Clock::get()?;
        let timestamp = clock.unix_timestamp;
        let writer = ctx.accounts.writer.key();
        let loom_asset = ctx.accounts.loom_asset.key();

        verify_core_loom(
            &ctx.accounts.loom_asset,
            &ctx.accounts.loom_collection,
            &writer,
        )?;

        let loom_state = &mut ctx.accounts.loom_state;
        if loom_state.loom_asset == Pubkey::default() {
            loom_state.loom_asset = loom_asset;
            loom_state.created_at = timestamp;
        }

        require_keys_eq!(
            loom_state.loom_asset,
            loom_asset,
            AnkySealError::InvalidLoomState
        );

        let total_seals = loom_state
            .total_seals
            .checked_add(1)
            .ok_or(AnkySealError::SealCountOverflow)?;
        let total_seals_bytes = total_seals.to_le_bytes();
        let timestamp_bytes = timestamp.to_le_bytes();

        let rolling_root = hashv(&[
            ROLLING_ROOT_DOMAIN,
            &loom_state.rolling_root,
            writer.as_ref(),
            loom_asset.as_ref(),
            &session_hash,
            &total_seals_bytes,
            &timestamp_bytes,
        ])
        .to_bytes();

        loom_state.total_seals = total_seals;
        loom_state.latest_session_hash = session_hash;
        loom_state.rolling_root = rolling_root;
        loom_state.updated_at = timestamp;

        emit!(AnkySealed {
            writer,
            loom_asset,
            session_hash,
            total_seals,
            rolling_root,
            timestamp,
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct SealAnky<'info> {
    #[account(mut)]
    pub writer: Signer<'info>,
    /// CHECK: Verified in verify_core_loom by owner, Core Asset deserialization, asset owner, and collection.
    pub loom_asset: UncheckedAccount<'info>,
    /// CHECK: Verified in verify_core_loom by owner, Core Collection deserialization, and official key.
    pub loom_collection: UncheckedAccount<'info>,
    #[account(
        init_if_needed,
        payer = writer,
        space = 8 + LoomState::INIT_SPACE,
        seeds = [LOOM_STATE_SEED, loom_asset.key().as_ref()],
        bump,
    )]
    pub loom_state: Account<'info, LoomState>,
    pub system_program: Program<'info, System>,
}

#[account]
#[derive(InitSpace)]
pub struct LoomState {
    pub loom_asset: Pubkey,
    pub total_seals: u64,
    pub latest_session_hash: [u8; 32],
    pub rolling_root: [u8; 32],
    pub created_at: i64,
    pub updated_at: i64,
}

#[event]
pub struct AnkySealed {
    pub writer: Pubkey,
    pub loom_asset: Pubkey,
    pub session_hash: [u8; 32],
    pub total_seals: u64,
    pub rolling_root: [u8; 32],
    pub timestamp: i64,
}

pub fn verify_core_loom(
    loom_asset: &UncheckedAccount,
    loom_collection: &UncheckedAccount,
    writer: &Pubkey,
) -> Result<()> {
    require_keys_eq!(
        *loom_asset.to_account_info().owner,
        METAPLEX_CORE_PROGRAM_ID,
        AnkySealError::InvalidLoomOwner
    );
    require_keys_eq!(
        *loom_collection.to_account_info().owner,
        METAPLEX_CORE_PROGRAM_ID,
        AnkySealError::InvalidLoomCollection
    );
    require_keys_eq!(
        loom_collection.key(),
        OFFICIAL_COLLECTION,
        AnkySealError::InvalidLoomCollection
    );

    let asset_data = loom_asset.try_borrow_data()?;
    let asset = parse_core_asset_base(asset_data.as_ref())?;
    require_keys_eq!(asset.owner, *writer, AnkySealError::InvalidLoomOwner);
    require_keys_eq!(
        asset.collection,
        OFFICIAL_COLLECTION,
        AnkySealError::InvalidLoomCollection
    );

    let collection_data = loom_collection.try_borrow_data()?;
    parse_core_collection_base(collection_data.as_ref())?;

    Ok(())
}

struct CoreAssetBase {
    owner: Pubkey,
    collection: Pubkey,
}

fn parse_core_asset_base(data: &[u8]) -> Result<CoreAssetBase> {
    let mut cursor = CoreDataCursor::new(data);
    let key = cursor.read_u8().ok_or(AnkySealError::InvalidLoomOwner)?;
    require!(key == CORE_KEY_ASSET_V1, AnkySealError::InvalidLoomOwner);

    let owner = cursor
        .read_pubkey()
        .ok_or(AnkySealError::InvalidLoomOwner)?;
    let update_authority = cursor
        .read_u8()
        .ok_or(AnkySealError::InvalidLoomCollection)?;
    require!(
        update_authority == CORE_UPDATE_AUTHORITY_COLLECTION,
        AnkySealError::InvalidLoomCollection
    );
    let collection = cursor
        .read_pubkey()
        .ok_or(AnkySealError::InvalidLoomCollection)?;

    Ok(CoreAssetBase { owner, collection })
}

fn parse_core_collection_base(data: &[u8]) -> Result<()> {
    let mut cursor = CoreDataCursor::new(data);
    let key = cursor
        .read_u8()
        .ok_or(AnkySealError::InvalidLoomCollection)?;
    require!(
        key == CORE_KEY_COLLECTION_V1,
        AnkySealError::InvalidLoomCollection
    );
    Ok(())
}

struct CoreDataCursor<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> CoreDataCursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    fn read_u8(&mut self) -> Option<u8> {
        let value = *self.data.get(self.offset)?;
        self.offset = self.offset.checked_add(1)?;
        Some(value)
    }

    fn read_pubkey(&mut self) -> Option<Pubkey> {
        let end = self.offset.checked_add(32)?;
        let bytes = self.data.get(self.offset..end)?;
        self.offset = end;
        Pubkey::try_from(bytes).ok()
    }
}

#[error_code]
pub enum AnkySealError {
    #[msg("The provided Loom asset is not owned by the expected Metaplex Core program, or the Core asset owner is not the writer.")]
    InvalidLoomOwner,
    #[msg(
        "The provided Loom collection does not match the official Anky Sojourn 9 Looms collection."
    )]
    InvalidLoomCollection,
    #[msg("The LoomState PDA does not match the provided Loom asset.")]
    InvalidLoomState,
    #[msg("The Loom seal counter overflowed.")]
    SealCountOverflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_core_asset_owner_and_collection_from_base_fields() {
        let owner = Pubkey::new_unique();
        let collection = OFFICIAL_COLLECTION;
        let mut data = Vec::new();
        data.push(CORE_KEY_ASSET_V1);
        data.extend_from_slice(owner.as_ref());
        data.push(CORE_UPDATE_AUTHORITY_COLLECTION);
        data.extend_from_slice(collection.as_ref());

        let parsed = parse_core_asset_base(&data).expect("parse asset");

        assert_eq!(parsed.owner, owner);
        assert_eq!(parsed.collection, collection);
    }

    #[test]
    fn rejects_non_collection_update_authority() {
        let owner = Pubkey::new_unique();
        let update_authority = Pubkey::new_unique();
        let mut data = Vec::new();
        data.push(CORE_KEY_ASSET_V1);
        data.extend_from_slice(owner.as_ref());
        data.push(1);
        data.extend_from_slice(update_authority.as_ref());

        assert!(parse_core_asset_base(&data).is_err());
    }

    #[test]
    fn parses_core_collection_discriminator() {
        assert!(parse_core_collection_base(&[CORE_KEY_COLLECTION_V1]).is_ok());
        assert!(parse_core_collection_base(&[CORE_KEY_ASSET_V1]).is_err());
    }
}

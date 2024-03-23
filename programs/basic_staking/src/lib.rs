use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_pack::IsInitialized;
use anchor_lang::solana_program::program_pack::Sealed;
use anchor_spl::token::{self, Mint, Token, TokenAccount};

declare_id!("HUjbRStzkMdzaYjJ22kfQfXJG6V4RqPLUfD9e4qe4j54");

#[program]
pub mod basic_staking {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, reward_rate: i64, unstaking_period: i64) -> Result<()> {
      let staking_data = &mut ctx.accounts.staking_data;
      staking_data.authority = *ctx.accounts.authority.key;
      staking_data.reward_mint_address = ctx.accounts.reward_mint_address.key();
      staking_data.reward_rate = reward_rate;
      staking_data.unstaking_period = unstaking_period;
      Ok(())
    }

    pub fn stake_nft(ctx: Context<StakeNFT>, _nft_mint_address: Pubkey) -> Result<()> {
      let staked_nft = &mut ctx.accounts.staked_nft;
        staked_nft.nft_mint_address = ctx.accounts.nft_mint_address.key();
        staked_nft.staker = *ctx.accounts.staker.key;
        staked_nft.staked_at = Clock::get()?.unix_timestamp;

        let cpi_accounts = token::Transfer {
            from: ctx.accounts.nft_token_account.to_account_info(),
            to: ctx.accounts.staked_nft_token_account.to_account_info(),
            authority: ctx.accounts.staker.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, 1)?;

        Ok(())
  }

  pub fn unstake_nft(ctx: Context<UnstakeNFT>, _nft_mint_address: Pubkey) -> Result<()> {
    let staked_nft = &ctx.accounts.staked_nft;
    let current_time = Clock::get()?.unix_timestamp;

    if current_time < staked_nft.staked_at + ctx.accounts.staking_data.unstaking_period {
        return Err(ErrorCode::UnstakingPeriodNotReached.into());
    }

    let seeds = &[
        b"staked_nft".as_ref(),
        staked_nft.nft_mint_address.as_ref(),
        staked_nft.staker.as_ref(),
        &[ctx.accounts.staking_data.bump],
    ];
    let signer = &[&seeds[..]];

    let cpi_accounts = token::Transfer {
        from: ctx.accounts.staked_nft_token_account.to_account_info(),
        to: ctx.accounts.nft_token_account.to_account_info(),
        authority: ctx.accounts.staked_nft.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    token::transfer(cpi_ctx, 1)?;

    Ok(())
  }

  pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
    let staked_nft = &ctx.accounts.staked_nft;
    let staking_data = &ctx.accounts.staking_data;
    let current_time = Clock::get()?.unix_timestamp;

    let reward_amount = (current_time - staked_nft.staked_at) * staking_data.reward_rate;

    let seeds = &[
        b"staking_data".as_ref(),
        staking_data.authority.as_ref(),
        &[staking_data.bump],
    ];
    let signer = &[&seeds[..]];

    let cpi_accounts = token::MintTo {
        mint: ctx.accounts.reward_mint_address.to_account_info(),
        to: ctx.accounts.reward_token_account.to_account_info(),
        authority: ctx.accounts.staking_data.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    token::mint_to(cpi_ctx, u64::try_from(reward_amount).unwrap())?;

    Ok(())
  }
}
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub struct StakingData {
    pub authority: Pubkey,
    pub reward_mint_address: Pubkey,
    pub reward_rate: i64,
    pub unstaking_period: i64,
    pub bump: u8,
}

impl IsInitialized for StakingData {
    fn is_initialized(&self) -> bool {
        true
    }
}

impl Sealed for StakingData {}

impl anchor_lang::Owner for StakingData {
    fn owner() -> Pubkey {
        crate::ID
    }
}

impl anchor_lang::AccountSerialize for StakingData {}
impl anchor_lang::AccountDeserialize for StakingData {
  fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::prelude::Result<Self> {
      Self::deserialize(buf).map_err(|_| ProgramError::InvalidAccountData.into())
  }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub struct StakedNFT {
    pub nft_mint_address: Pubkey,
    pub staker: Pubkey,
    pub staked_at: i64,
}

impl IsInitialized for StakedNFT {
    fn is_initialized(&self) -> bool {
        true
    }
}

impl Sealed for StakedNFT {}

impl anchor_lang::Owner for StakedNFT {
    fn owner() -> Pubkey {
        crate::ID
    }
}

impl anchor_lang::AccountSerialize for StakedNFT {}
impl anchor_lang::AccountDeserialize for StakedNFT {
  fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::prelude::Result<Self> {
      Self::deserialize(buf).map_err(|_| ProgramError::InvalidAccountData.into())
  }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = authority, space = 8 + std::mem::size_of::<StakingData>())]
    pub staking_data: Account<'info, StakingData>,
    pub reward_mint_address: Account<'info, Mint>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct StakeNFT<'info> {
    #[account(mut)]
    pub staker: Signer<'info>,
    pub nft_mint_address: Account<'info, Mint>,
    #[account(mut)]
    pub nft_token_account: Account<'info, TokenAccount>,
    #[account(
      init,
      payer = staker,
      space = 8 + std::mem::size_of::<StakedNFT>(),
      seeds = [b"staked_nft", nft_mint_address.key().as_ref(), staker.key().as_ref()],
      bump
  )]
    pub staked_nft: Account<'info, StakedNFT>,
    #[account(mut)]
    pub staked_nft_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UnstakeNFT<'info> {
    #[account(mut)]
    pub staker: Signer<'info>,
    pub nft_mint_address: Account<'info, Mint>,
    #[account(mut)]
    pub nft_token_account: Account<'info, TokenAccount>,
    #[account(
      mut,
      has_one = nft_mint_address,
      has_one = staker,
      seeds = [b"staked_nft", nft_mint_address.key().as_ref(), staker.key().as_ref()],
      bump
  )]
  pub staked_nft: Account<'info, StakedNFT>,
    pub staking_data: Account<'info, StakingData>,
    #[account(mut)]
    pub staked_nft_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(mut)]
    pub staker: Signer<'info>,
    pub nft_mint_address: Account<'info, Mint>,
    #[account(has_one = nft_mint_address, has_one = staker)]
    pub staked_nft: Account<'info, StakedNFT>,
    #[account(mut, seeds = [b"staking_data", staking_data.authority.as_ref()], bump = staking_data.bump)]
    pub staking_data: Account<'info, StakingData>,
    #[account(mut)]
    pub reward_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub reward_mint_address: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Unstaking period has not been reached")]
    UnstakingPeriodNotReached,
}
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{clock};
use anchor_spl::token::{self, CloseAccount, Mint, SetAuthority, TokenAccount, Transfer};
use spl_token::instruction::AuthorityType;

declare_id!("G2roHNqPvkVz4hko9Ha8443QrFUGg5YFkLDqW7Cyt1LK");

#[program]
pub mod spin_win {
    use super::*;

    pub const ESCROW_PDA_SEED: &str = "sw_game_vault_auth";
    pub const SPIN_ITEM_COUNT: usize = 15;
    pub const REWARD_TOKEN_COUNT_PER_ITEM: usize = 10;
    pub const MAX_REWARD_TOKEN_COUNT: usize = 150; // REWARD_TOKEN_COUNT_PER_ITEM * SPIN_ITEM_COUNT;

    pub fn initialize(
        ctx: Context<Initialize>,
        _bump : u8,
    ) -> ProgramResult {
        msg!("initialize");

        let pool = &mut ctx.accounts.pool;
        pool.owner = *ctx.accounts.initializer.key;
        pool.bump = _bump;

        let mut state = ctx.accounts.state.load_init()?;

        Ok(())
    }

    pub fn add_item(
        ctx: Context<SpinWheel>,
        item_mint_list: [Pubkey; 10],
        count: u8,
        token_type: u8,
        ratio: u8,
        amount: u64,
    ) -> ProgramResult {
        msg!("add_item");

        let mut state = ctx.accounts.state.load_mut()?;
        state.add_spinitem(ItemRewardMints{item_mint_list, count}, token_type, ratio, amount)?;

        Ok(())
    }

    pub fn set_item(
        ctx: Context<SpinWheel>,
        index: u8,
        item_mint_list: [Pubkey; 10],
        count: u8,
        token_type: u8,
        ratio: u8,
        amount: u64,
    ) -> ProgramResult {
        msg!("set_item");

        let mut state = ctx.accounts.state.load_mut()?;
        state.set_spinitem(index, ItemRewardMints{item_mint_list, count}, token_type, ratio, amount)?;

        Ok(())
    }

    pub fn spin_wheel(ctx: Context<SpinWheel>) -> ProgramResult {
        let mut state = ctx.accounts.state.load_mut()?;
        state.get_spinresult();

        return Ok(());
    }

    pub fn claim(
        ctx : Context<Claim>,
        amount: u64,
        ) -> ProgramResult {

        let (_vault_authority, vault_authority_bump) =
        Pubkey::find_program_address(&[ESCROW_PDA_SEED.as_ref()], ctx.program_id);
        let authority_seeds = &[&ESCROW_PDA_SEED.as_bytes()[..], &[vault_authority_bump]];

        // let pool = &ctx.accounts.pool;
        // let authority_seeds = &[
        //     pool.rand.as_ref(),
        //     &[pool.bump],
        // ];
        token::transfer(
            ctx.accounts.into_transfer_to_pda_context()
                .with_signer(&[&authority_seeds[..]]),
        amount,
        );

        Ok(())
    }
}

#[account]
#[derive(Default)]
pub struct Pool {
    pub owner : Pubkey,
    pub bump : u8,
}

#[derive(Accounts)]
#[instruction(_bump : u8)]
pub struct Initialize<'info> {
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut, signer)]
    pub initializer: AccountInfo<'info>,

    #[account(init, seeds=[ESCROW_PDA_SEED.as_ref()], bump=_bump, payer=initializer)]
    pool : Account<'info, Pool>,

    #[account(zero)]
    state : AccountLoader<'info, SpinItemList>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: AccountInfo<'info>,
}

// space : 32 * 10 + 1
#[zero_copy]
#[derive(Default, AnchorSerialize, AnchorDeserialize)]
pub struct ItemRewardMints {
    item_mint_list: [Pubkey; REWARD_TOKEN_COUNT_PER_ITEM],
    count: u8,
}

// space : 4975
#[account(zero_copy)]
#[repr(packed)]
pub struct SpinItemList {
    reward_mint_list: [ItemRewardMints; SPIN_ITEM_COUNT],   // 321 * 15
    token_type_list: [u8; SPIN_ITEM_COUNT],   // 15
    ratio_list: [u8; SPIN_ITEM_COUNT],  // 15
    amount_list: [u64; SPIN_ITEM_COUNT],    // 8 * 15
    last_spinindex: u8, // 1
    count: u8, // 1
}

impl ItemRewardMints {
    pub fn add_reward_item(&mut self, reward_mint: Pubkey) {
        self.item_mint_list[self.count as usize] = reward_mint;
        self.count += 1;
    }
}

impl Default for SpinItemList {
    #[inline]
    fn default() -> SpinItemList {
        SpinItemList {
            reward_mint_list: [
                ItemRewardMints {
                    ..Default::default()
                }; SPIN_ITEM_COUNT
            ],
            token_type_list: [0; SPIN_ITEM_COUNT],
            ratio_list: [0; SPIN_ITEM_COUNT],
            amount_list: [0; SPIN_ITEM_COUNT],
            last_spinindex: 0,
            count: 0,
        }
    }
}

impl SpinItemList {
    pub fn add_spinitem(&mut self, item_mint_list: ItemRewardMints, token_type: u8, ratio: u8, amount: u64,) -> Result<()> {
        require!(self.count <= SPIN_ITEM_COUNT as u8, SpinError::CountOverflowAddItem);

        self.reward_mint_list[self.count as usize] = item_mint_list;
        self.token_type_list[self.count as usize] = token_type;
        self.ratio_list[self.count as usize] = ratio;
        self.amount_list[self.count as usize] = amount;
        self.count += 1;

        Ok(())
    }

    pub fn set_spinitem(&mut self, index: u8, item_mint_list: ItemRewardMints, token_type: u8, ratio: u8, amount: u64,) -> Result<()> {
        require!(index < SPIN_ITEM_COUNT as u8, SpinError::IndexOverflowSetItem);

        self.reward_mint_list[index as usize] = item_mint_list;
        self.token_type_list[index as usize] = token_type;
        self.ratio_list[index as usize] = ratio;
        self.amount_list[index as usize] = amount;
        if self.count <= index {
            self.count = index + 1;
        }

        Ok(())
    }

    pub fn clear_spinitem(&mut self) {
        self.count = 0;
    }

    pub fn get_spinresult(&mut self) {
        let c = clock::Clock::get().unwrap();
        let r = (c.unix_timestamp % 100) as u8;
        let mut start = 0;
        for (pos, item) in self.ratio_list.iter().enumerate() {
            let end = start + item;
            if r >= start && r < end {
                self.last_spinindex = pos as u8;
                return;
            }
            start = end;
        }
    }
}

#[derive(Accounts)]
pub struct SpinWheel<'info> {
    #[account(mut)]
    state : AccountLoader<'info, SpinItemList>,
}

#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut, signer)]
    owner : AccountInfo<'info>,

    state : AccountLoader<'info, SpinItemList>,

    #[account(mut)]
    pool : Account<'info, Pool>,


    #[account(mut,owner=spl_token::id())]
    source_reward_account : AccountInfo<'info>,

    #[account(mut,owner=spl_token::id())]
    dest_reward_account : AccountInfo<'info>,

    #[account(address=spl_token::id())]
    token_program : AccountInfo<'info>,
}

impl<'info> Claim<'info> {
    fn into_transfer_to_pda_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self
                .source_reward_account
                .to_account_info()
                .clone(),
            to: self.dest_reward_account.to_account_info().clone(),
            authority: self.pool.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

#[error]
pub enum SpinError {
    #[msg("Count Overflow To Add Item")]
    CountOverflowAddItem,

    #[msg("Index Overflow To Set Item")]
    IndexOverflowSetItem,
}
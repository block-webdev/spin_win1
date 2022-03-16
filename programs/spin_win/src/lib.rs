use anchor_lang::prelude::*;
use anchor_lang::solana_program::{clock};
use anchor_spl::token::{self, CloseAccount, Mint, SetAuthority, TokenAccount, Transfer};
use spl_token::instruction::AuthorityType;

declare_id!("Cuw7DiTqrwe1vzuXUABq3sToxCXauMKT9UsniivXyruM");

#[program]
pub mod spin_win {
    use super::*;

    pub const ESCROW_PDA_SEED: &str = "sw_game_seeds";
    pub const SPIN_ITEM_COUNT: usize = 15;

    pub fn initialize(
        ctx: Context<Initialize>, _pool_bump: u8,
    ) -> ProgramResult {
        msg!("initialize");

        let state = &mut ctx.accounts.state;
        state.amount_list = [0; SPIN_ITEM_COUNT];
        state.ratio_list = [0; SPIN_ITEM_COUNT];

        Ok(())
    }

    pub fn set_item(
        ctx: Context<SetItem>,
        token_vault_bump: u8,
        ratio: [u8; 15],
        amount: [u64; 15],
    ) -> ProgramResult {
        msg!("set_item");

        let state = &mut ctx.accounts.state;
        state.ratio_list = ratio;
        state.amount_list = amount;

        // let cpi_ctx = CpiContext::new(
        //     ctx.accounts.token_program.to_account_info().clone(),
        //     token::Transfer {
        //         from: ctx.accounts.reward_account.to_account_info(),
        //         to: ctx.accounts.token_vault.to_account_info(),
        //         authority: ctx.accounts.owner.to_account_info(),
        //     },
        // );
        // token::transfer(cpi_ctx, amount)?;

        Ok(())
    }

    pub fn spin_wheel(ctx: Context<SpinWheel>) -> ProgramResult {
        let state = &mut ctx.accounts.state;
        let spin_index: u8 = get_spinresult(state) as u8;
        state.last_spinindex = spin_index;

        return Ok(());
    }

    pub fn claim(
        ctx : Context<Claim>,
        amount: u64,
        ) -> ProgramResult {

        let (_vault_authority, vault_authority_bump) =
        Pubkey::find_program_address(&[ESCROW_PDA_SEED.as_ref()], ctx.program_id);
        let authority_seeds = &[&ESCROW_PDA_SEED.as_bytes()[..], &[vault_authority_bump]];

        token::transfer(
            ctx.accounts.into_transfer_to_pda_context()
                .with_signer(&[&authority_seeds[..]]),
        amount,
        );

        Ok(())
    }

}

fn get_spinresult(state: &mut SpinItemList) -> u8 {
    let c = clock::Clock::get().unwrap();
    let r = (c.unix_timestamp % 100) as u8;
    let mut start = 0;
    for (pos, item) in state.ratio_list.iter().enumerate() {
        let end = start + item;
        if r >= start && r < end {
            return pos as u8;
        }
        start = end;
    }

    return 0;
}

#[derive(Accounts)]
#[instruction(_bump: u8)]
pub struct Initialize<'info> {
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut, signer)]
    initializer : AccountInfo<'info>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(init, payer=initializer, seeds=[ESCROW_PDA_SEED.as_ref()], bump = _bump)]
    state : Account<'info, SpinItemList>,

    system_program: Program<'info, System>,
}


#[account]
#[derive(Default)]
pub struct SpinItemList {
    ratio_list: [u8; SPIN_ITEM_COUNT],
    amount_list: [u64; SPIN_ITEM_COUNT],
    last_spinindex: u8,
}


#[derive(Accounts)]
#[instruction(_token_bump: u8)]
pub struct SetItem<'info> {
    /// CHECK: this is not dangerous.
    #[account(mut, signer)]
    owner : AccountInfo<'info>, 

    /// CHECK: this is not dangerous.
    #[account(mut)]
    state : Account<'info, SpinItemList>,

    token_mint: Account<'info, Mint>,
    #[account(
        init,
        seeds = [(*rand.key).as_ref()],
        bump = _token_bump,
        payer = owner,
        token::mint = token_mint,
        token::authority = owner,
    )]
    token_vault: Account<'info, TokenAccount>,

    rand : AccountInfo<'info>,

    /// CHECK: this is not dangerous.
    // #[account(mut)]
    // reward_account : Account<'info, TokenAccount>,

    /// CHECK: this is not dangerous.
    #[account(address=spl_token::id())]
    token_program : AccountInfo<'info>,

    system_program : Program<'info,System>,
    rent: Sysvar<'info, Rent>
}


#[derive(Accounts)]
pub struct SpinWheel<'info> {
    #[account(mut)]
    state : Account<'info, SpinItemList>,
}

#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut, signer)]
    owner : AccountInfo<'info>,

    state : Account<'info, SpinItemList>,

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
            authority: self.state.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}
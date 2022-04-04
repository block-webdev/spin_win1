import * as anchor from '@project-serum/anchor';
import { Program } from '@project-serum/anchor';
import { SpinWin } from '../target/types/spin_win';

import { PublicKey, SystemProgram, Transaction, Connection, Commitment, SYSVAR_RENT_PUBKEY } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID, Token, ASSOCIATED_TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { assert } from "chai";


describe('spin_win', () => {

  // Configure the client to use the local cluster.
  const provider = anchor.Provider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SpinWin as Program<SpinWin>;

  let mintA = null as Token;
  let mintB = null as Token;
  let initializerTokenAccountA = null;
  let initializerTokenAccountB = null;
  let takerTokenAccountA = null;
  let takerTokenAccountB = null;
  let vault_account_pda = null;
  let vault_account_bump = null;
  let vault_authority_pda = null;

  let pool_account_pda = null;
  let pool_account_bump = null;

  let token_vault_list = [];

  let testPoolAcc = null;

  const takerAmount = 1000;
  const initializerAmount = 500;

  const escrowAccount = anchor.web3.Keypair.generate();
  const payer = anchor.web3.Keypair.generate();
  const mintAuthority = anchor.web3.Keypair.generate();
  const initializerMainAccount = anchor.web3.Keypair.generate();
  const takerMainAccount = anchor.web3.Keypair.generate();

  it("Initialize program state", async () => {
    // Airdropping tokens to a payer.
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(payer.publicKey, 1000000000),
      "processed"
    );
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(provider.wallet.publicKey, 1000000000),
      "processed"
    );

    // Fund Main Accounts
    await provider.send(
      (() => {
        const tx = new Transaction();
        tx.add(
          SystemProgram.transfer({
            fromPubkey: payer.publicKey,
            toPubkey: initializerMainAccount.publicKey,
            lamports: 100000000,
          }),
          SystemProgram.transfer({
            fromPubkey: payer.publicKey,
            toPubkey: takerMainAccount.publicKey,
            lamports: 100000000,
          })
        );
        return tx;
      })(),
      [payer]
    );

    mintA = await Token.createMint(
      provider.connection,
      payer,
      mintAuthority.publicKey,
      null,
      0,
      TOKEN_PROGRAM_ID
    );

    mintB = await Token.createMint(
      provider.connection,
      payer,
      mintAuthority.publicKey,
      null,
      0,
      TOKEN_PROGRAM_ID
    );

    initializerTokenAccountA = await mintA.createAccount(initializerMainAccount.publicKey);
    takerTokenAccountA = await mintA.createAccount(takerMainAccount.publicKey);

    initializerTokenAccountB = await mintB.createAccount(initializerMainAccount.publicKey);
    takerTokenAccountB = await mintB.createAccount(takerMainAccount.publicKey);

    await mintA.mintTo(
      initializerTokenAccountA,
      mintAuthority.publicKey,
      [mintAuthority],
      initializerAmount
    );

    await mintB.mintTo(
      takerTokenAccountB,
      mintAuthority.publicKey,
      [mintAuthority],
      takerAmount
    );

    let _initializerTokenAccountA = await mintA.getAccountInfo(initializerTokenAccountA);
    let _takerTokenAccountB = await mintB.getAccountInfo(takerTokenAccountB);

    assert.ok(_initializerTokenAccountA.amount.toNumber() == initializerAmount);
    assert.ok(_takerTokenAccountB.amount.toNumber() == takerAmount);
  });

  it("Initialize escrow", async () => {
    let [_pool, _pool_bump] = await PublicKey.findProgramAddress([Buffer.from(anchor.utils.bytes.utf8.encode("sw_game_seeds"))], program.programId);
    pool_account_pda = _pool;
    pool_account_bump = _pool_bump;

    const [_vault_account_pda, _vault_account_bump] = await PublicKey.findProgramAddress(
      [Buffer.from(anchor.utils.bytes.utf8.encode("sw_token-seed"))],
      program.programId
    );
    vault_account_pda = _vault_account_pda;
    vault_account_bump = _vault_account_bump;

    const [_vault_authority_pda, _vault_authority_bump] = await PublicKey.findProgramAddress(
      [Buffer.from(anchor.utils.bytes.utf8.encode("escrow"))],
      program.programId
    );
    vault_authority_pda = _vault_authority_pda;

    console.log('initialize start...');


    // let randomPubkey = anchor.web3.Keypair.generate().publicKey;
    // let [_pool111, _bump111] = await PublicKey.findProgramAddress(
    //   [randomPubkey.toBuffer()],
    //   program.programId
    // );


    const [_pool111, _bump111] = await PublicKey.findProgramAddress([Buffer.from(anchor.utils.bytes.utf8.encode("sw_game_vault_auth"))], program.programId);

    vault_account_pda = _pool111;

    pool_account_pda = await PublicKey.createWithSeed(
      initializerMainAccount.publicKey,
      "user-lottery-pool",
      program.programId,
    );

    await program.rpc.initialize(
      _bump111,
      {
        accounts: {
          initializer: initializerMainAccount.publicKey,
          pool: _pool111,
          state: pool_account_pda,
          systemProgram: anchor.web3.SystemProgram.programId,
        },
        signers: [initializerMainAccount],
        preInstructions: [
          SystemProgram.createAccountWithSeed({
            fromPubkey: initializerMainAccount.publicKey,
            basePubkey: initializerMainAccount.publicKey,
            seed: "user-lottery-pool",
            newAccountPubkey: pool_account_pda,
            lamports: await provider.connection.getMinimumBalanceForRentExemption(4975),
            space: 4975,
            programId: program.programId,
          })
        ]
      }
    );

    
    testPoolAcc = await mintA.createAccount(_pool111);
    await mintA.mintTo(
      testPoolAcc,
      mintAuthority.publicKey,
      [mintAuthority],
      100
    );

    console.log('initialize end...');
  });

  it("withdraw", async () => {
    console.log('Start to withdraw');

    var myToken = new Token(
      provider.connection,
      mintA.publicKey,
      TOKEN_PROGRAM_ID,
      payer
    );
    console.log('0000000000', vault_account_pda.toBase58());

    var sourceAccount = testPoolAcc; // await myToken.getOrCreateAssociatedAccountInfo(vault_account_pda);
    let tokenAmount = await provider.connection.getTokenAccountBalance(sourceAccount);
    console.log('1111111', tokenAmount);
    let amount = tokenAmount.value.amount;

    var destAccount = await myToken.getOrCreateAssociatedAccountInfo(initializerMainAccount.publicKey);
    tokenAmount = await provider.connection.getTokenAccountBalance(destAccount.address);
    console.log('2222222', tokenAmount);

    await program.rpc.withdrawPaidTokens(
      new anchor.BN(amount),
      {
        accounts: {
          pool: vault_account_pda,
          sourceAccount: sourceAccount,
          destAccount: destAccount.address,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        // signers: [payer]
      });

    tokenAmount = await provider.connection.getTokenAccountBalance(sourceAccount);
    console.log('3333333333', tokenAmount);
    tokenAmount = await provider.connection.getTokenAccountBalance(destAccount.address);
    console.log('4444444444', tokenAmount);

    console.log('End to withdraw...');
  });

  return;

  it("Set Item", async () => {
    console.log('Start to Set Item...');

    let randomPubkey = anchor.web3.Keypair.generate().publicKey;
    let [_token_vault, _token_vault_bump] = await PublicKey.findProgramAddress([Buffer.from(randomPubkey.toBuffer())], program.programId);

    token_vault_list.push({ vault: _token_vault, bump: _token_vault_bump });

    let ratio_list = [];
    let amount_list = [];
    for (let i = 0; i < 15; i++) {
      if (i >= 0 && i <= 4) {
        ratio_list.push(2);
      } else if (i >= 5 && i <= 9) {
        ratio_list.push(10);
      } else {
        ratio_list.push(8);
      }
      amount_list.push(new anchor.BN(2));
    }

    let mintkeys = [];
    for (let i = 0; i < 10; i++) {
      mintkeys.push(anchor.web3.Keypair.generate().publicKey);
    }
    let item_mint_list = [mintkeys, 10];

    for (let i = 0; i < 15; i++) {
      await program.rpc.setItem(
        i,
        mintkeys,
        10,
        i == 14 ? 2 : 7,
        new anchor.BN(3),
        {
          accounts: {
            state: pool_account_pda,
          },
          // signers: [initializerMainAccount]
        }
      );
    }

    console.log('End to Set Item...');
  });

  it("spin_wheel", async () => {
    console.log('Start to spin_wheel...');
    await program.rpc.spinWheel({
      accounts: {
        state: pool_account_pda,
      }
    });

    let _state = await program.account.spinItemList.fetch(
      pool_account_pda
    );

    let t_vault_account = token_vault_list[0];
    console.log('spin token vault : ', t_vault_account);

    console.log('last spin index : ', _state.lastSpinindex);
    // await program.rpc.transferRewards(
    //   _state.lastSpinindex,
    //   {
    //     accounts: {
    //       owner: initializerMainAccount.publicKey,
    //       state: pool_account_pda,
    //       tokenMint: mintA.publicKey,
    //       tokenVault: t_vault_account.vault,
    //       destAccount: initializerTokenAccountA,
    //       systemProgram: anchor.web3.SystemProgram.programId,
    //       tokenProgram: TOKEN_PROGRAM_ID,
    //     },
    //     signers: [initializerMainAccount]
    //   });

    console.log('End to spin_wheel...');
  });

  it("claim rewards", async () => {
    console.log('Start to claim rewards...');

    var myToken = new Token(
      provider.connection,
      mintA.publicKey,
      TOKEN_PROGRAM_ID,
      payer
    );

    let _state1 = await program.account.spinItemList.fetch(
      pool_account_pda
    );

    let rewardPDA = await anchor.web3.PublicKey.findProgramAddress(
      [pool_account_pda.toBuffer(), TOKEN_PROGRAM_ID.toBuffer(), mintA.publicKey.toBuffer()],
      ASSOCIATED_TOKEN_PROGRAM_ID
    );
    if ((await provider.connection.getAccountInfo(rewardPDA[0])) == null) {
      await provider.send(
        (() => {
          const keys = [
            { pubkey: payer.publicKey, isSigner: true, isWritable: true },
            { pubkey: rewardPDA[0], isSigner: false, isWritable: true },
            { pubkey: pool_account_pda, isSigner: false, isWritable: false },
            { pubkey: mintA.publicKey, isSigner: false, isWritable: false },
            {
              pubkey: anchor.web3.SystemProgram.programId,
              isSigner: false,
              isWritable: false,
            },
            { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
            {
              pubkey: anchor.web3.SYSVAR_RENT_PUBKEY,
              isSigner: false,
              isWritable: false,
            },
          ];
          let transaction = new Transaction();
          transaction.add(
            new anchor.web3.TransactionInstruction({
              keys,
              programId: ASSOCIATED_TOKEN_PROGRAM_ID,
              data: Buffer.from([]),
            }));
          return transaction;
        })(),
        [payer]
      );
    }

    // var sourceAccount = await myToken.getOrCreateAssociatedAccountInfo(pool_account_pda);
    let sourceAccount = rewardPDA[0];

    await mintA.mintTo(
      sourceAccount,
      mintAuthority.publicKey,
      [mintAuthority],
      100
    );
    let tokenAmount = await provider.connection.getTokenAccountBalance(sourceAccount);
    console.log('444xxx444444444444444', tokenAmount);

    var destAccount = await myToken.getOrCreateAssociatedAccountInfo(initializerMainAccount.publicKey);
    tokenAmount = await provider.connection.getTokenAccountBalance(destAccount.address);
    console.log('~~~~~~~~~~~~~~~~~~~~~~~111', tokenAmount);

    await program.rpc.claim(
      new anchor.BN(20),
      {
        accounts: {
          owner: initializerMainAccount.publicKey,
          state: pool_account_pda,
          sourceRewardAccount: sourceAccount,
          destRewardAccount: destAccount.address,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [initializerMainAccount]
      });

    tokenAmount = await provider.connection.getTokenAccountBalance(destAccount.address);
    console.log('~~~~~~~~~~~~~~~~~~~~~~~222', tokenAmount);

    // await program.rpc.transferRewards(
    //   _state.lastSpinindex,
    //   {
    //     accounts: {
    //       owner: initializerMainAccount.publicKey,
    //       state: pool_account_pda,
    //       tokenMint: mintA.publicKey,
    //       tokenVault: t_vault_account.vault,
    //       destAccount: initializerTokenAccountA,
    //       systemProgram: anchor.web3.SystemProgram.programId,
    //       tokenProgram: TOKEN_PROGRAM_ID,
    //     },
    //     signers: [initializerMainAccount]
    //   });

    console.log('End to spin_wheel...');
  });
});

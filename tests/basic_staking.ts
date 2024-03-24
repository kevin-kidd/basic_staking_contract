import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { BasicStaking } from "../target/types/basic_staking";
import { PublicKey, SystemProgram, Keypair } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, createMint, createAccount, mintTo, getAccount } from "@solana/spl-token";
import { expect } from "chai";
import assert from "assert";

describe("basic_staking", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.BasicStaking as Program<BasicStaking>;

  // Create a new keypair for testing
  const testKeypair = Keypair.generate();

  // Define test variables
  let rewardMintAddress: PublicKey;
  let staker: PublicKey;
  let nftMintAddress: PublicKey;
  let nftTokenAccount: PublicKey;
  let stakedNftTokenAccount: PublicKey;
  let rewardTokenAccount: PublicKey;

  before(async () => {
    // Airdrop SOL to the test keypair
    const airdropSignature = await provider.connection.requestAirdrop(testKeypair.publicKey, 10 * anchor.web3.LAMPORTS_PER_SOL);
    await provider.connection.confirmTransaction(airdropSignature, "confirmed");

    // Initialize test variables before running the tests
    rewardMintAddress = await createMint(provider.connection, testKeypair, testKeypair.publicKey, null, 9);
    staker = testKeypair.publicKey;

    // Create dummy NFT mint and token accounts
    nftMintAddress = await createMint(provider.connection, testKeypair, testKeypair.publicKey, null, 0);
    nftTokenAccount = await createAccount(provider.connection, testKeypair, nftMintAddress, staker);
    await mintTo(provider.connection, testKeypair, nftMintAddress, nftTokenAccount, testKeypair.publicKey, 1);

    stakedNftTokenAccount = await createAccount(provider.connection, testKeypair, nftMintAddress, program.programId);
    rewardTokenAccount = await createAccount(provider.connection, testKeypair, rewardMintAddress, staker);
  });

  it("Initialize staking data", async () => {
    // Test initialization of staking data
    const [stakingDataPda, _] = PublicKey.findProgramAddressSync(
      [Buffer.from("staking_data"), provider.wallet.publicKey.toBuffer()],
      program.programId
    );

    await program.methods
      .initialize(new anchor.BN(100), new anchor.BN(86400))
      .accounts({
        stakingData: stakingDataPda,
        rewardMintAddress: rewardMintAddress,
        authority: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();


    // Assert that the staking data is initialized correctly
    const stakingData = await program.account["stakingData"].fetch(stakingDataPda);
    assert.strictEqual(stakingData.authority, provider.wallet.publicKey);
    assert.strictEqual(stakingData.rewardMintAddress, rewardMintAddress);
    assert.strictEqual(stakingData.rewardRate.toNumber(), 100);
    assert.strictEqual(stakingData.unstakingPeriod.toNumber(), 86400);
  });

  it("Stake NFT", async () => {
    // Test staking an NFT
    const [stakedNftPda, _] = PublicKey.findProgramAddressSync(
      [Buffer.from("staked_nft"), nftMintAddress.toBuffer(), staker.toBuffer()],
      program.programId
    );

    await program.methods
      .stakeNft(nftMintAddress)
      .accounts({
        staker: staker,
        nftMintAddress: nftMintAddress,
        nftTokenAccount: nftTokenAccount,
        stakedNft: stakedNftPda,
        stakedNftTokenAccount: stakedNftTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    // Assert that the NFT is staked correctly
    const stakedNft = await program.account["stakedNft"].fetch(stakedNftPda);
    assert.strictEqual(stakedNft.nftMint, nftMintAddress);
    assert.strictEqual(stakedNft.staker, staker);

    const stakedNftTokenAccountInfo = await getAccount(provider.connection, stakedNftTokenAccount);
    assert.strictEqual(stakedNftTokenAccountInfo.amount, 1);
  });

  it("Unstake NFT", async () => {
    // Test unstaking an NFT
    const [stakingDataPda, _a] = PublicKey.findProgramAddressSync(
      [Buffer.from("staking_data"), provider.wallet.publicKey.toBuffer()],
      program.programId
    );

    const [stakedNftPda, _b] = PublicKey.findProgramAddressSync(
      [Buffer.from("staked_nft"), nftMintAddress.toBuffer(), staker.toBuffer()],
      program.programId
    );

    // Wait for the unstaking period to pass
    await new Promise((resolve) => setTimeout(resolve, 86400 * 1000));

    await program.methods
      .unstakeNft(nftMintAddress)
      .accounts({
        staker: staker,
        nftMintAddress: nftMintAddress,
        nftTokenAccount: nftTokenAccount,
        stakedNft: stakedNftPda,
        stakedNftTokenAccount: stakedNftTokenAccount,
        stakingData: stakingDataPda,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    // Assert that the NFT is unstaked correctly
    const nftTokenAccountInfo = await getAccount(provider.connection, nftTokenAccount);
    assert.strictEqual(nftTokenAccountInfo.amount, 1);
  });

  it("Claim rewards", async () => {
    // Test claiming rewards
    const [stakingDataPda, _c] = PublicKey.findProgramAddressSync(
      [Buffer.from("staking_data"), provider.wallet.publicKey.toBuffer()],
      program.programId
    );

    const [stakedNftPda, _d] = PublicKey.findProgramAddressSync(
      [Buffer.from("staked_nft"), nftMintAddress.toBuffer(), staker.toBuffer()],
      program.programId
    );

    await program.methods
      .claimRewards()
      .accounts({
        staker: staker,
        nftMintAddress: nftMintAddress,
        stakedNft: stakedNftPda,
        rewardTokenAccount: rewardTokenAccount,
        stakingData: stakingDataPda,
        rewardMintAddress: rewardMintAddress,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    // Assert that the rewards are claimed correctly
    const rewardTokenAccountInfo = await getAccount(provider.connection, rewardTokenAccount);
    expect(rewardTokenAccountInfo.amount).to.be.greaterThan(0);
  });
});
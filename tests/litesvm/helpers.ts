import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { LiteSVMProvider, fromWorkspace } from "anchor-litesvm";
import { assert } from "chai";
import {
  Keypair,
  PublicKey,
  SendTransactionError,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountIdempotentInstruction,
  createInitializeMintInstruction,
  createMintToInstruction,
  getAccount,
  getAssociatedTokenAddressSync,
  MINT_SIZE,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";

import { Swap } from "../../target/types/swap";

const idl = require("../../target/idl/swap.json");

export const INITIAL_MAKER_MINT_AMOUNT = 5_000_000;
export const OFFER_AMOUNT_GIVES = new anchor.BN(1_000_000);
export const OFFER_AMOUNT_WANTS = new anchor.BN(2_000_000);

export const ERROR_CODES = {
  InvalidAmount: 6000,
  SameMintNotAllowed: 6001,
  MintMismatch: 6005,
  MakerCannotBeTaker: 6007,
} as const;

export type TestContext = {
  provider: LiteSVMProvider;
  program: Program<Swap>;
  maker: Keypair;
  taker: Keypair;
  mintMakerGives: PublicKey;
  mintMakerWants: PublicKey;
  makerAtaGives: PublicKey;
};

export function createProvider(): LiteSVMProvider {
  const client = fromWorkspace(process.cwd())
    .withDefaultPrograms()
    .withSysvars();
  const provider = new LiteSVMProvider(client);

  anchor.setProvider(provider);

  return provider;
}

export async function sendInstructions(
  provider: LiteSVMProvider,
  instructions: TransactionInstruction[],
  signers: Keypair[] = [],
): Promise<string> {
  const tx = new anchor.web3.Transaction().add(...instructions);
  return provider.sendAndConfirm!(tx, signers);
}

export function airdrop(
  provider: LiteSVMProvider,
  pubkey: PublicKey,
  solAmount = 2,
): void {
  provider.client.airdrop(
    pubkey,
    BigInt(solAmount * anchor.web3.LAMPORTS_PER_SOL),
  );
}

export async function createMint(
  provider: LiteSVMProvider,
  authority: PublicKey,
  decimals = 6,
): Promise<PublicKey> {
  const mint = Keypair.generate();
  const lamports =
    await provider.connection.getMinimumBalanceForRentExemption(MINT_SIZE);

  await sendInstructions(
    provider,
    [
      SystemProgram.createAccount({
        fromPubkey: provider.wallet.publicKey,
        newAccountPubkey: mint.publicKey,
        lamports,
        space: MINT_SIZE,
        programId: TOKEN_PROGRAM_ID,
      }),
      createInitializeMintInstruction(
        mint.publicKey,
        decimals,
        authority,
        null,
        TOKEN_PROGRAM_ID,
      ),
    ],
    [mint],
  );

  return mint.publicKey;
}

export async function createAta(
  provider: LiteSVMProvider,
  mint: PublicKey,
  owner: PublicKey,
  allowOwnerOffCurve = false,
): Promise<PublicKey> {
  const ata = getAssociatedTokenAddressSync(
    mint,
    owner,
    allowOwnerOffCurve,
    TOKEN_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID,
  );

  await sendInstructions(provider, [
    createAssociatedTokenAccountIdempotentInstruction(
      provider.wallet.publicKey,
      ata,
      owner,
      mint,
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID,
    ),
  ]);

  return ata;
}

export async function mintTokens(
  provider: LiteSVMProvider,
  mint: PublicKey,
  destination: PublicKey,
  authority: Keypair,
  amount: number,
): Promise<string> {
  return sendInstructions(
    provider,
    [
      createMintToInstruction(
        mint,
        destination,
        authority.publicKey,
        amount,
        [],
        TOKEN_PROGRAM_ID,
      ),
    ],
    [authority],
  );
}

export function deriveOfferAccounts(
  maker: PublicKey,
  offerId: anchor.BN,
  mintMakerGives: PublicKey,
  programId: PublicKey,
): { offerPda: PublicKey; vaultAta: PublicKey } {
  const [offerPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("offer"),
      maker.toBuffer(),
      offerId.toArrayLike(Buffer, "le", 8),
    ],
    programId,
  );

  const vaultAta = getAssociatedTokenAddressSync(
    mintMakerGives,
    offerPda,
    true,
    TOKEN_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID,
  );

  return { offerPda, vaultAta };
}

export async function expectAnchorError(
  promise: Promise<unknown>,
  expectedCode: keyof typeof ERROR_CODES,
): Promise<void> {
  try {
    await promise;
    assert.fail(`Expected ${expectedCode} but transaction succeeded`);
  } catch (error) {
    const directAnchorError =
      error instanceof anchor.AnchorError ? error : null;
    const sendError = error as SendTransactionError;
    const parsedAnchorError =
      directAnchorError ??
      (Array.isArray(sendError.logs)
        ? anchor.AnchorError.parse(sendError.logs)
        : null);

    assert.isNotNull(parsedAnchorError, `Expected AnchorError, got: ${error}`);
    assert.strictEqual(parsedAnchorError!.error.errorCode.code, expectedCode);
    assert.strictEqual(
      parsedAnchorError!.error.errorCode.number,
      ERROR_CODES[expectedCode],
    );
  }
}

export async function expectFailure(promise: Promise<unknown>): Promise<void> {
  try {
    await promise;
    assert.fail("Expected transaction to fail");
  } catch (_error) {
    assert.isTrue(true);
  }
}

export async function getTokenAmount(
  provider: LiteSVMProvider,
  tokenAccount: PublicKey,
): Promise<string> {
  const tokenAccountInfo = await getAccount(
    provider.connection as anchor.web3.Connection,
    tokenAccount,
    undefined,
    TOKEN_PROGRAM_ID,
  );

  return tokenAccountInfo.amount.toString();
}

export function expectMissingAccount(
  provider: LiteSVMProvider,
  pubkey: PublicKey,
): void {
  const account = provider.client.getAccount(pubkey);

  if (account == null) {
    assert.isTrue(true);
    return;
  }

  assert.strictEqual(account.data.length, 0);
}

export async function createTestContext(): Promise<TestContext> {
  const provider = createProvider();
  const program = new Program<Swap>(idl as Swap, provider);
  const maker = Keypair.generate();
  const taker = Keypair.generate();

  airdrop(provider, maker.publicKey, 2);
  airdrop(provider, taker.publicKey, 2);

  const mintMakerGives = await createMint(provider, maker.publicKey);
  const mintMakerWants = await createMint(provider, maker.publicKey);
  const makerAtaGives = await createAta(
    provider,
    mintMakerGives,
    maker.publicKey,
  );

  await mintTokens(
    provider,
    mintMakerGives,
    makerAtaGives,
    maker,
    INITIAL_MAKER_MINT_AMOUNT,
  );

  return {
    provider,
    program,
    maker,
    taker,
    mintMakerGives,
    mintMakerWants,
    makerAtaGives,
  };
}

export async function makeOffer(
  ctx: TestContext,
  offerId: anchor.BN,
  amountMakerGives = OFFER_AMOUNT_GIVES,
  amountMakerWants = OFFER_AMOUNT_WANTS,
): Promise<{ offerPda: PublicKey; vaultAta: PublicKey }> {
  const { offerPda, vaultAta } = deriveOfferAccounts(
    ctx.maker.publicKey,
    offerId,
    ctx.mintMakerGives,
    ctx.program.programId,
  );

  await ctx.program.methods
    .makeOffer(offerId, amountMakerGives, amountMakerWants)
    .accountsPartial({
      maker: ctx.maker.publicKey,
      mintMakerGives: ctx.mintMakerGives,
      mintMakerWants: ctx.mintMakerWants,
      makerAtaGives: ctx.makerAtaGives,
      vault: vaultAta,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      offer: offerPda,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
    .signers([ctx.maker])
    .rpc();

  return { offerPda, vaultAta };
}

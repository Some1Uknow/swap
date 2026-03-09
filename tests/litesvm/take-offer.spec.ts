import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";
import {
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";

import {
  createAta,
  createMint,
  createTestContext,
  expectAnchorError,
  expectFailure,
  expectMissingAccount,
  getTokenAmount,
  makeOffer,
  mintTokens,
  OFFER_AMOUNT_GIVES,
  OFFER_AMOUNT_WANTS,
} from "./helpers";

describe("take_offer (litesvm)", () => {
  it("settles the trade and closes escrow accounts", async () => {
    const ctx = await createTestContext();
    const offerId = new anchor.BN(1);
    const { offerPda, vaultAta } = await makeOffer(ctx, offerId);

    const makerAtaWants = await createAta(
      ctx.provider,
      ctx.mintMakerWants,
      ctx.maker.publicKey,
    );
    const takerAtaWants = await createAta(
      ctx.provider,
      ctx.mintMakerWants,
      ctx.taker.publicKey,
    );
    const takerAtaGives = await createAta(
      ctx.provider,
      ctx.mintMakerGives,
      ctx.taker.publicKey,
    );

    await mintTokens(
      ctx.provider,
      ctx.mintMakerWants,
      takerAtaWants,
      ctx.maker,
      OFFER_AMOUNT_WANTS.toNumber(),
    );

    await ctx.program.methods
      .takeOffer(offerId)
      .accountsPartial({
        taker: ctx.taker.publicKey,
        maker: ctx.maker.publicKey,
        mintMakerGives: ctx.mintMakerGives,
        mintMakerWants: ctx.mintMakerWants,
        makerAtaWants,
        takerAtaWants,
        takerAtaGives,
        vault: vaultAta,
        offer: offerPda,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([ctx.taker])
      .rpc();

    expectMissingAccount(ctx.provider, offerPda);
    expectMissingAccount(ctx.provider, vaultAta);
    assert.strictEqual(await getTokenAmount(ctx.provider, takerAtaWants), "0");
    assert.strictEqual(
      await getTokenAmount(ctx.provider, makerAtaWants),
      OFFER_AMOUNT_WANTS.toString(),
    );
    assert.strictEqual(
      await getTokenAmount(ctx.provider, takerAtaGives),
      OFFER_AMOUNT_GIVES.toString(),
    );
  });

  it("fails when already filled", async () => {
    const ctx = await createTestContext();
    const offerId = new anchor.BN(2);
    const { offerPda, vaultAta } = await makeOffer(ctx, offerId);

    const makerAtaWants = await createAta(
      ctx.provider,
      ctx.mintMakerWants,
      ctx.maker.publicKey,
    );
    const takerAtaWants = await createAta(
      ctx.provider,
      ctx.mintMakerWants,
      ctx.taker.publicKey,
    );
    const takerAtaGives = await createAta(
      ctx.provider,
      ctx.mintMakerGives,
      ctx.taker.publicKey,
    );

    await mintTokens(
      ctx.provider,
      ctx.mintMakerWants,
      takerAtaWants,
      ctx.maker,
      OFFER_AMOUNT_WANTS.toNumber(),
    );

    await ctx.program.methods
      .takeOffer(offerId)
      .accountsPartial({
        taker: ctx.taker.publicKey,
        maker: ctx.maker.publicKey,
        mintMakerGives: ctx.mintMakerGives,
        mintMakerWants: ctx.mintMakerWants,
        makerAtaWants,
        takerAtaWants,
        takerAtaGives,
        vault: vaultAta,
        offer: offerPda,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([ctx.taker])
      .rpc();

    await expectFailure(
      ctx.program.methods
        .takeOffer(offerId)
        .accountsPartial({
          taker: ctx.taker.publicKey,
          maker: ctx.maker.publicKey,
          mintMakerGives: ctx.mintMakerGives,
          mintMakerWants: ctx.mintMakerWants,
          makerAtaWants,
          takerAtaWants,
          takerAtaGives,
          vault: vaultAta,
          offer: offerPda,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([ctx.taker])
        .rpc(),
    );
  });

  it("fails when maker tries to take their own offer", async () => {
    const ctx = await createTestContext();
    const offerId = new anchor.BN(3);
    const { offerPda, vaultAta } = await makeOffer(ctx, offerId);

    const makerAtaWants = await createAta(
      ctx.provider,
      ctx.mintMakerWants,
      ctx.maker.publicKey,
    );

    await expectAnchorError(
      ctx.program.methods
        .takeOffer(offerId)
        .accountsPartial({
          taker: ctx.maker.publicKey,
          maker: ctx.maker.publicKey,
          mintMakerGives: ctx.mintMakerGives,
          mintMakerWants: ctx.mintMakerWants,
          makerAtaWants,
          takerAtaWants: makerAtaWants,
          takerAtaGives: ctx.makerAtaGives,
          vault: vaultAta,
          offer: offerPda,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([ctx.maker])
        .rpc(),
      "MakerCannotBeTaker",
    );
  });

  it("fails when taker passes the wrong wants mint", async () => {
    const ctx = await createTestContext();
    const offerId = new anchor.BN(4);
    const { offerPda, vaultAta } = await makeOffer(ctx, offerId);
    const wrongWantsMint = await createMint(ctx.provider, ctx.maker.publicKey);
    const wrongMakerAtaWants = await createAta(
      ctx.provider,
      wrongWantsMint,
      ctx.maker.publicKey,
    );
    const wrongTakerAtaWants = await createAta(
      ctx.provider,
      wrongWantsMint,
      ctx.taker.publicKey,
    );
    const takerAtaGives = await createAta(
      ctx.provider,
      ctx.mintMakerGives,
      ctx.taker.publicKey,
    );

    await mintTokens(
      ctx.provider,
      wrongWantsMint,
      wrongTakerAtaWants,
      ctx.maker,
      OFFER_AMOUNT_WANTS.toNumber(),
    );

    await expectAnchorError(
      ctx.program.methods
        .takeOffer(offerId)
        .accountsPartial({
          taker: ctx.taker.publicKey,
          maker: ctx.maker.publicKey,
          mintMakerGives: ctx.mintMakerGives,
          mintMakerWants: wrongWantsMint,
          makerAtaWants: wrongMakerAtaWants,
          takerAtaWants: wrongTakerAtaWants,
          takerAtaGives,
          vault: vaultAta,
          offer: offerPda,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([ctx.taker])
        .rpc(),
      "MintMismatch",
    );
  });

  it("fails when taker does not have enough tokens and leaves offer intact", async () => {
    const ctx = await createTestContext();
    const offerId = new anchor.BN(5);
    const { offerPda, vaultAta } = await makeOffer(ctx, offerId);
    const poorTaker = anchor.web3.Keypair.generate();

    ctx.provider.client.airdrop(
      poorTaker.publicKey,
      BigInt(2 * anchor.web3.LAMPORTS_PER_SOL),
    );

    const makerAtaWants = await createAta(
      ctx.provider,
      ctx.mintMakerWants,
      ctx.maker.publicKey,
    );
    const poorTakerAtaWants = await createAta(
      ctx.provider,
      ctx.mintMakerWants,
      poorTaker.publicKey,
    );
    const poorTakerAtaGives = await createAta(
      ctx.provider,
      ctx.mintMakerGives,
      poorTaker.publicKey,
    );

    await mintTokens(
      ctx.provider,
      ctx.mintMakerWants,
      poorTakerAtaWants,
      ctx.maker,
      OFFER_AMOUNT_WANTS.toNumber() - 1,
    );

    await expectFailure(
      ctx.program.methods
        .takeOffer(offerId)
        .accountsPartial({
          taker: poorTaker.publicKey,
          maker: ctx.maker.publicKey,
          mintMakerGives: ctx.mintMakerGives,
          mintMakerWants: ctx.mintMakerWants,
          makerAtaWants,
          takerAtaWants: poorTakerAtaWants,
          takerAtaGives: poorTakerAtaGives,
          vault: vaultAta,
          offer: offerPda,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([poorTaker])
        .rpc(),
    );

    const offer = await ctx.program.account.offer.fetch(offerPda);
    assert.strictEqual(offer.status, 0);
    assert.strictEqual(
      await getTokenAmount(ctx.provider, vaultAta),
      OFFER_AMOUNT_GIVES.toString(),
    );
  });
});

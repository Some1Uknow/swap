import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";

import {
  createAta,
  createTestContext,
  expectFailure,
  expectMissingAccount,
  getTokenAmount,
  INITIAL_MAKER_MINT_AMOUNT,
  makeOffer,
  OFFER_AMOUNT_GIVES,
} from "./helpers";

describe("cancel_offer (litesvm)", () => {
  it("refunds the maker and closes escrow accounts", async () => {
    const ctx = await createTestContext();
    const offerId = new anchor.BN(1);
    const { offerPda, vaultAta } = await makeOffer(ctx, offerId);

    await ctx.program.methods
      .cancelOffer(offerId)
      .accountsPartial({
        maker: ctx.maker.publicKey,
        mintMakerGives: ctx.mintMakerGives,
        makerAtaGives: ctx.makerAtaGives,
        vault: vaultAta,
        offer: offerPda,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([ctx.maker])
      .rpc();

    expectMissingAccount(ctx.provider, offerPda);
    expectMissingAccount(ctx.provider, vaultAta);
    assert.strictEqual(
      await getTokenAmount(ctx.provider, ctx.makerAtaGives),
      INITIAL_MAKER_MINT_AMOUNT.toString(),
    );
  });

  it("fails for a non-maker", async () => {
    const ctx = await createTestContext();
    const offerId = new anchor.BN(2);
    const { offerPda, vaultAta } = await makeOffer(ctx, offerId);
    const takerAtaGives = await createAta(
      ctx.provider,
      ctx.mintMakerGives,
      ctx.taker.publicKey,
    );

    await expectFailure(
      ctx.program.methods
        .cancelOffer(offerId)
        .accountsPartial({
          maker: ctx.taker.publicKey,
          mintMakerGives: ctx.mintMakerGives,
          makerAtaGives: takerAtaGives,
          vault: vaultAta,
          offer: offerPda,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([ctx.taker])
        .rpc(),
    );
  });

  it("fails after the offer was already cancelled", async () => {
    const ctx = await createTestContext();
    const offerId = new anchor.BN(3);
    const { offerPda, vaultAta } = await makeOffer(ctx, offerId);

    await ctx.program.methods
      .cancelOffer(offerId)
      .accountsPartial({
        maker: ctx.maker.publicKey,
        mintMakerGives: ctx.mintMakerGives,
        makerAtaGives: ctx.makerAtaGives,
        vault: vaultAta,
        offer: offerPda,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([ctx.maker])
      .rpc();

    await expectFailure(
      ctx.program.methods
        .cancelOffer(offerId)
        .accountsPartial({
          maker: ctx.maker.publicKey,
          mintMakerGives: ctx.mintMakerGives,
          makerAtaGives: ctx.makerAtaGives,
          vault: vaultAta,
          offer: offerPda,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([ctx.maker])
        .rpc(),
    );
  });

  it("prevents taking an offer after it was cancelled", async () => {
    const ctx = await createTestContext();
    const offerId = new anchor.BN(4);
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

    await ctx.program.methods
      .cancelOffer(offerId)
      .accountsPartial({
        maker: ctx.maker.publicKey,
        mintMakerGives: ctx.mintMakerGives,
        makerAtaGives: ctx.makerAtaGives,
        vault: vaultAta,
        offer: offerPda,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([ctx.maker])
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
});

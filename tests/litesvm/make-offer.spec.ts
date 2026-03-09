import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";

import {
  createTestContext,
  deriveOfferAccounts,
  expectAnchorError,
  expectFailure,
  getTokenAmount,
  INITIAL_MAKER_MINT_AMOUNT,
  makeOffer,
  OFFER_AMOUNT_GIVES,
  OFFER_AMOUNT_WANTS,
} from "./helpers";

describe("make_offer (litesvm)", () => {
  it("stores offer state and moves funds into the vault", async () => {
    const ctx = await createTestContext();
    const offerId = new anchor.BN(1);
    const { offerPda, vaultAta } = await makeOffer(ctx, offerId);

    const offer = await ctx.program.account.offer.fetch(offerPda);
    assert.strictEqual(offer.status, 0);
    assert.strictEqual(offer.maker.toBase58(), ctx.maker.publicKey.toBase58());
    assert.strictEqual(
      offer.mintMakerGives.toBase58(),
      ctx.mintMakerGives.toBase58(),
    );
    assert.strictEqual(
      offer.mintMakerWants.toBase58(),
      ctx.mintMakerWants.toBase58(),
    );
    assert.ok(offer.amountMakerGives.eq(OFFER_AMOUNT_GIVES));
    assert.ok(offer.amountMakerWants.eq(OFFER_AMOUNT_WANTS));

    assert.strictEqual(
      await getTokenAmount(ctx.provider, vaultAta),
      OFFER_AMOUNT_GIVES.toString(),
    );
    assert.strictEqual(
      await getTokenAmount(ctx.provider, ctx.makerAtaGives),
      new anchor.BN(INITIAL_MAKER_MINT_AMOUNT)
        .sub(OFFER_AMOUNT_GIVES)
        .toString(),
    );
  });

  it("fails with zero give amount", async () => {
    const ctx = await createTestContext();
    const offerId = new anchor.BN(2);
    const { offerPda, vaultAta } = deriveOfferAccounts(
      ctx.maker.publicKey,
      offerId,
      ctx.mintMakerGives,
      ctx.program.programId,
    );

    await expectAnchorError(
      ctx.program.methods
        .makeOffer(offerId, new anchor.BN(0), OFFER_AMOUNT_WANTS)
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
        .rpc(),
      "InvalidAmount",
    );
  });

  it("fails with zero want amount", async () => {
    const ctx = await createTestContext();
    const offerId = new anchor.BN(3);
    const { offerPda, vaultAta } = deriveOfferAccounts(
      ctx.maker.publicKey,
      offerId,
      ctx.mintMakerGives,
      ctx.program.programId,
    );

    await expectAnchorError(
      ctx.program.methods
        .makeOffer(offerId, OFFER_AMOUNT_GIVES, new anchor.BN(0))
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
        .rpc(),
      "InvalidAmount",
    );
  });

  it("fails when give and want mint are the same", async () => {
    const ctx = await createTestContext();
    const offerId = new anchor.BN(4);
    const { offerPda, vaultAta } = deriveOfferAccounts(
      ctx.maker.publicKey,
      offerId,
      ctx.mintMakerGives,
      ctx.program.programId,
    );

    await expectAnchorError(
      ctx.program.methods
        .makeOffer(offerId, OFFER_AMOUNT_GIVES, OFFER_AMOUNT_WANTS)
        .accountsPartial({
          maker: ctx.maker.publicKey,
          mintMakerGives: ctx.mintMakerGives,
          mintMakerWants: ctx.mintMakerGives,
          makerAtaGives: ctx.makerAtaGives,
          vault: vaultAta,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          offer: offerPda,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([ctx.maker])
        .rpc(),
      "SameMintNotAllowed",
    );
  });

  it("fails when maker does not have enough tokens", async () => {
    const ctx = await createTestContext();
    const poorMaker = anchor.web3.Keypair.generate();

    ctx.provider.client.airdrop(
      poorMaker.publicKey,
      BigInt(2 * anchor.web3.LAMPORTS_PER_SOL),
    );

    const poorMakerAtaGives = await (
      await import("./helpers")
    ).createAta(ctx.provider, ctx.mintMakerGives, poorMaker.publicKey);

    await (await import("./helpers")).mintTokens(
      ctx.provider,
      ctx.mintMakerGives,
      poorMakerAtaGives,
      ctx.maker,
      OFFER_AMOUNT_GIVES.toNumber() - 1,
    );

    const offerId = new anchor.BN(5);
    const { offerPda, vaultAta } = deriveOfferAccounts(
      poorMaker.publicKey,
      offerId,
      ctx.mintMakerGives,
      ctx.program.programId,
    );

    await expectFailure(
      ctx.program.methods
        .makeOffer(offerId, OFFER_AMOUNT_GIVES, OFFER_AMOUNT_WANTS)
        .accountsPartial({
          maker: poorMaker.publicKey,
          mintMakerGives: ctx.mintMakerGives,
          mintMakerWants: ctx.mintMakerWants,
          makerAtaGives: poorMakerAtaGives,
          vault: vaultAta,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          offer: offerPda,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([poorMaker])
        .rpc(),
    );
  });

  it("fails when the same offer id is reused by the same maker", async () => {
    const ctx = await createTestContext();
    const offerId = new anchor.BN(6);
    const { offerPda, vaultAta } = await makeOffer(ctx, offerId);

    await expectFailure(
      ctx.program.methods
        .makeOffer(offerId, OFFER_AMOUNT_GIVES, OFFER_AMOUNT_WANTS)
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
        .rpc(),
    );
  });
});

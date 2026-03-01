import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Swap } from "../target/types/swap";
import { assert } from "chai";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createMint,
  getAssociatedTokenAddressSync,
  getOrCreateAssociatedTokenAccount,
  TOKEN_PROGRAM_ID,
  mintTo,
  getAccount,
} from "@solana/spl-token";

describe("swap", () => {
  const INITIAL_MAKER_MINT_AMOUNT = 5_000_000;

  anchor.setProvider(anchor.AnchorProvider.env());

  const provider = anchor.getProvider() as anchor.AnchorProvider;
  const connection = provider.connection;
  const program = anchor.workspace.swap as Program<Swap>;
  let mintMakerGives: anchor.web3.PublicKey;
  let mintMakerWants: anchor.web3.PublicKey;
  let makerAtaGives: anchor.web3.PublicKey;
  let offerId: anchor.BN;
  let offerPda: anchor.web3.PublicKey;
  let vaultAta: anchor.web3.PublicKey;

  // Step 1 helper (we will implement this first).
  async function airdropSol(
    pubkey: anchor.web3.PublicKey,
    solAmount = 2,
  ): Promise<void> {
    const signature = await connection.requestAirdrop(
      pubkey,
      solAmount * anchor.web3.LAMPORTS_PER_SOL,
    );

    const latest = await connection.getLatestBlockhash();

    await connection.confirmTransaction(
      {
        signature,
        blockhash: latest.blockhash,
        lastValidBlockHeight: latest.lastValidBlockHeight,
      },
      "confirmed",
    );
  }

  const maker = anchor.web3.Keypair.generate();

  before(async () => {
    await airdropSol(maker.publicKey, 2);
    const makerBalance = await connection.getBalance(maker.publicKey);
    assert.ok(makerBalance > 0);

    mintMakerGives = await createMint(
      connection,
      maker,
      maker.publicKey,
      null,
      6,
      undefined,
      undefined,
      TOKEN_PROGRAM_ID,
    );

    console.log("Mint for what maker gives:", mintMakerGives.toBase58());

    mintMakerWants = await createMint(
      connection,
      maker,
      maker.publicKey,
      null,
      6,
      undefined,
      undefined,
      TOKEN_PROGRAM_ID,
    );

    console.log("Mint for what maker wants:", mintMakerWants.toBase58());
    assert.notStrictEqual(mintMakerGives.toBase58(), mintMakerWants.toBase58());

    const makerAta = await getOrCreateAssociatedTokenAccount(
      connection,
      maker,
      mintMakerGives,
      maker.publicKey,
    );
    makerAtaGives = makerAta.address;

    console.log("Maker ATA for what maker gives:", makerAtaGives.toBase58());

    await mintTo(
      connection,
      maker,
      mintMakerGives,
      makerAtaGives,
      maker,
      INITIAL_MAKER_MINT_AMOUNT,
    );

    const makerAtaInfo = await getAccount(connection, makerAtaGives);
    assert.strictEqual(
      makerAtaInfo.amount.toString(),
      INITIAL_MAKER_MINT_AMOUNT.toString(),
    );

    offerId = new anchor.BN(1);

    [offerPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("offer"),
        maker.publicKey.toBuffer(),
        offerId.toArrayLike(Buffer, "le", 8),
      ],
      program.programId,
    );

    vaultAta = getAssociatedTokenAddressSync(
      mintMakerGives,
      offerPda,
      true,
      TOKEN_PROGRAM_ID,
    );

    console.log("Vault ATA for what maker gives:", vaultAta.toBase58());
  });

  it("make_offer", async () => {
    const amountMakerGives = new anchor.BN(1_000_000);
    const amountMakerWants = new anchor.BN(2_000_000);

    const makeOfferTx = await program.methods
      .makeOffer(offerId, amountMakerGives, amountMakerWants)
      .accountsPartial({
        maker: maker.publicKey,
        mintMakerGives,
        mintMakerWants,
        makerAtaGives,
        vault: vaultAta,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        offer: offerPda,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([maker])
      .rpc();

    console.log("make_offer transaction signature:", makeOfferTx);

    const offer = await program.account.offer.fetch(offerPda);
    assert.strictEqual(offer.status, 0);
    assert.strictEqual(offer.maker.toBase58(), maker.publicKey.toBase58());
    assert.strictEqual(
      offer.mintMakerGives.toBase58(),
      mintMakerGives.toBase58(),
    );
    assert.strictEqual(
      offer.mintMakerWants.toBase58(),
      mintMakerWants.toBase58(),
    );
    assert.ok(offer.amountMakerGives.eq(amountMakerGives));
    assert.ok(offer.amountMakerWants.eq(amountMakerWants));

    const vaultAtaInfo = await getAccount(connection, vaultAta);
    assert.strictEqual(
      vaultAtaInfo.amount.toString(),
      amountMakerGives.toString(),
    );

    const makerAtaAfter = await getAccount(connection, makerAtaGives);
    const expectedMakerLeft = new anchor.BN(INITIAL_MAKER_MINT_AMOUNT).sub(
      amountMakerGives,
    );

    assert.strictEqual(
      makerAtaAfter.amount.toString(),
      expectedMakerLeft.toString(),
    );
  });
});

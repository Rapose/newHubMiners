import * as anchor from "@coral-xyz/anchor";
import { PublicKey, SystemProgram } from "@solana/web3.js";

const SEED_CONFIG = Buffer.from("config");
const SEED_LB_LAND = Buffer.from("lb_land");
const SEED_LAND = Buffer.from("land");

function u64Le(n: bigint) {
  const b = Buffer.alloc(8);
  b.writeBigUInt64LE(n);
  return b;
}

function readBool(buf: Buffer, offset: number): boolean {
  return buf.readUInt8(offset) !== 0;
}

function readU8(buf: Buffer, offset: number): number {
  return buf.readUInt8(offset);
}

function readU64(buf: Buffer, offset: number): bigint {
  return buf.readBigUInt64LE(offset);
}

function readPubkey(buf: Buffer, offset: number): PublicKey {
  return new PublicKey(buf.subarray(offset, offset + 32));
}

function readBytes32(buf: Buffer, offset: number): Buffer {
  return Buffer.from(buf.subarray(offset, offset + 32));
}

type RawLootboxLandState = {
  lootboxId: bigint;
  owner: PublicKey;
  committed: boolean;
  revealed: boolean;
  commitSlot: bigint;
  commitment: Buffer;
  rarity: number;
  element: number;
  slots: number;
  bump: number;
};

function parseLootboxLandState(data: Buffer): RawLootboxLandState {
  const o = 8;

  return {
    lootboxId: readU64(data, o),
    owner: readPubkey(data, o + 8),
    committed: readBool(data, o + 40),
    revealed: readBool(data, o + 41),
    commitSlot: readU64(data, o + 42),
    commitment: readBytes32(data, o + 50),
    rarity: readU8(data, o + 82),
    element: readU8(data, o + 83),
    slots: readU8(data, o + 84),
    bump: readU8(data, o + 85),
  };
}

async function fetchRawLootboxLandState(
  connection: anchor.web3.Connection,
  pda: PublicKey
): Promise<RawLootboxLandState> {
  const info = await connection.getAccountInfo(pda, "confirmed");
  if (!info) throw new Error("lootbox land account não encontrada");
  return parseLootboxLandState(Buffer.from(info.data));
}

describe("lootbox_land_flow", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.MoeAnchorV1;
  const owner = provider.wallet.publicKey;

  it("land lootbox full flow", async function () {
    this.timeout(120000);

    const lootboxId = BigInt(Date.now());

    const [config] = PublicKey.findProgramAddressSync(
      [SEED_CONFIG],
      program.programId
    );

    const cfgInfo = await provider.connection.getAccountInfo(config, "confirmed");
    if (!cfgInfo) {
      console.log("Config not found. Initializing...");
      await program.methods
        .initializeConfig()
        .accounts({
          admin: owner,
          config,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    }

    const [lootbox] = PublicKey.findProgramAddressSync(
      [SEED_LB_LAND, owner.toBuffer(), u64Le(lootboxId)],
      program.programId
    );

    const cfgBefore = await program.account.config.fetch(config);
    const nextLandId = BigInt(cfgBefore.nextLandId.toString());

    const [landState] = PublicKey.findProgramAddressSync(
      [SEED_LAND, owner.toBuffer(), u64Le(nextLandId)],
      program.programId
    );

    const salt = new Uint8Array(32);

    console.log("debug PDAs:", {
      config: config.toBase58(),
      lootboxId: lootboxId.toString(),
      lootbox: lootbox.toBase58(),
      nextLandId: nextLandId.toString(),
      landState: landState.toBase58(),
    });

    await program.methods
      .lootboxLandInit(new anchor.BN(lootboxId.toString()))
      .accounts({
        owner,
        config,
        lootbox,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("land init ok");

    await program.methods
      .lootboxLandCommit(new anchor.BN(lootboxId.toString()), [...salt])
      .accounts({
        owner,
        lootbox,
      })
      .rpc();

    const lbAfterCommit = await fetchRawLootboxLandState(provider.connection, lootbox);

    console.log("lootbox land after commit:", {
      lootboxId: lbAfterCommit.lootboxId.toString(),
      owner: lbAfterCommit.owner.toBase58(),
      committed: lbAfterCommit.committed,
      revealed: lbAfterCommit.revealed,
      commitSlot: lbAfterCommit.commitSlot.toString(),
      rarity: lbAfterCommit.rarity,
      element: lbAfterCommit.element,
      slots: lbAfterCommit.slots,
    });

    await new Promise((r) => setTimeout(r, 2000));

    await program.methods
      .lootboxLandReveal(new anchor.BN(lootboxId.toString()), [...salt])
      .accounts({
        owner,
        config,
        lootbox,
        landState,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const lbAfterReveal = await fetchRawLootboxLandState(provider.connection, lootbox);

    console.log("\n=== LOOTBOX LAND FINAL ===");
    console.log({
      lootboxPda: lootbox.toBase58(),
      lootboxId: lbAfterReveal.lootboxId.toString(),
      owner: lbAfterReveal.owner.toBase58(),
      committed: lbAfterReveal.committed,
      revealed: lbAfterReveal.revealed,
      commitSlot: lbAfterReveal.commitSlot.toString(),
      rarity: lbAfterReveal.rarity,
      element: lbAfterReveal.element,
      slots: lbAfterReveal.slots,
      bump: lbAfterReveal.bump,
      commitmentHex: lbAfterReveal.commitment.toString("hex"),
    });

    console.log("\n=== LAND CHARACTERISTICS ===");
    console.log({
      rarity: lbAfterReveal.rarity,
      element: lbAfterReveal.element,
      slots: lbAfterReveal.slots,
      committed: lbAfterReveal.committed,
      revealed: lbAfterReveal.revealed,
    });

    try {
      const land: any = await (program.account as any).landState.fetch(landState);

      console.log("\n=== LAND STATE ===");
      console.log({
        landPda: landState.toBase58(),
        lootboxPda: lootbox.toBase58(),
        owner: land.owner?.toBase58?.() ?? null,
        rarity: land.rarity,
        element: land.element,
        slots: land.slots ?? land.maxSlots ?? land.max_slots ?? null,
      });
    } catch (e: any) {
      console.log("[WARN] could not decode landState via IDL:", e?.message ?? e);
    }

    console.log("Land revealed");
  });
});
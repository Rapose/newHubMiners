import * as anchor from "@coral-xyz/anchor";
import { PublicKey, SystemProgram } from "@solana/web3.js";

const SEED_CONFIG = Buffer.from("config");
const SEED_LB_MINER = Buffer.from("lb_miner");
const SEED_MINER = Buffer.from("miner");
const SEED_MINER_PROGRESS = Buffer.from("miner_progress_v1");

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

type RawLootboxMinerState = {
  lootboxId: bigint;
  owner: PublicKey;
  committed: boolean;
  revealed: boolean;
  commitSlot: bigint;
  commitment: Buffer;
  rarity: number;
  element: number;
  hashBase: bigint;
  face: number;
  helmet: number;
  backpack: number;
  jacket: number;
  item: number;
  background: number;
  bump: number;
};

function parseLootboxMinerState(data: Buffer): RawLootboxMinerState {
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
    hashBase: readU64(data, o + 84),
    face: readU8(data, o + 92),
    helmet: readU8(data, o + 93),
    backpack: readU8(data, o + 94),
    jacket: readU8(data, o + 95),
    item: readU8(data, o + 96),
    background: readU8(data, o + 97),
    bump: readU8(data, o + 98),
  };
}

async function fetchRawLootboxMinerState(
  connection: anchor.web3.Connection,
  pda: PublicKey
): Promise<RawLootboxMinerState> {
  const info = await connection.getAccountInfo(pda, "confirmed");
  if (!info) throw new Error("lootbox miner account não encontrada");
  return parseLootboxMinerState(Buffer.from(info.data));
}

describe("lootbox_miner_flow", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.MoeAnchorV1;
  const owner = provider.wallet.publicKey;

  it("miner lootbox full flow", async function () {
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
      [SEED_LB_MINER, owner.toBuffer(), u64Le(lootboxId)],
      program.programId
    );

    const cfgBefore = await program.account.config.fetch(config);
    const nextMinerId = BigInt(cfgBefore.nextMinerId.toString());

    const [minerState] = PublicKey.findProgramAddressSync(
      [SEED_MINER, owner.toBuffer(), u64Le(nextMinerId)],
      program.programId
    );

    const [minerProgress] = PublicKey.findProgramAddressSync(
      [SEED_MINER_PROGRESS, minerState.toBuffer()],
      program.programId
    );

    const salt = new Uint8Array(32);

    console.log("debug PDAs:", {
      config: config.toBase58(),
      lootboxId: lootboxId.toString(),
      lootbox: lootbox.toBase58(),
      nextMinerId: nextMinerId.toString(),
      minerState: minerState.toBase58(),
      minerProgress: minerProgress.toBase58(),
    });

    await program.methods
      .lootboxMinerInit(new anchor.BN(lootboxId.toString()))
      .accounts({
        owner,
        config,
        lootbox,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("miner init ok");

    await program.methods
      .lootboxMinerCommit(new anchor.BN(lootboxId.toString()), [...salt])
      .accounts({
        owner,
        lootbox,
      })
      .rpc();

    const lbAfterCommit = await fetchRawLootboxMinerState(provider.connection, lootbox);

    console.log("lootbox miner after commit:", {
      lootboxId: lbAfterCommit.lootboxId.toString(),
      owner: lbAfterCommit.owner.toBase58(),
      committed: lbAfterCommit.committed,
      revealed: lbAfterCommit.revealed,
      commitSlot: lbAfterCommit.commitSlot.toString(),
      rarity: lbAfterCommit.rarity,
      element: lbAfterCommit.element,
      hashBase: lbAfterCommit.hashBase.toString(),
    });

    await new Promise((r) => setTimeout(r, 3000));

    await program.methods
      .lootboxMinerReveal(new anchor.BN(lootboxId.toString()), [...salt])
      .accounts({
        owner,
        config,
        lootbox,
        minerState,
        minerProgress,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const lbAfterReveal = await fetchRawLootboxMinerState(provider.connection, lootbox);

    console.log("\n=== LOOTBOX MINER FINAL ===");
    console.log({
      lootboxPda: lootbox.toBase58(),
      lootboxId: lbAfterReveal.lootboxId.toString(),
      owner: lbAfterReveal.owner.toBase58(),
      committed: lbAfterReveal.committed,
      revealed: lbAfterReveal.revealed,
      commitSlot: lbAfterReveal.commitSlot.toString(),
      rarity: lbAfterReveal.rarity,
      element: lbAfterReveal.element,
      hashBase: lbAfterReveal.hashBase.toString(),
      face: lbAfterReveal.face,
      helmet: lbAfterReveal.helmet,
      backpack: lbAfterReveal.backpack,
      jacket: lbAfterReveal.jacket,
      item: lbAfterReveal.item,
      background: lbAfterReveal.background,
      bump: lbAfterReveal.bump,
      commitmentHex: lbAfterReveal.commitment.toString("hex"),
    });

    console.log("\n=== MINER CHARACTERISTICS ===");
    console.log({
      rarity: lbAfterReveal.rarity,
      element: lbAfterReveal.element,
      hashBase: lbAfterReveal.hashBase.toString(),
      face: lbAfterReveal.face,
      helmet: lbAfterReveal.helmet,
      backpack: lbAfterReveal.backpack,
      jacket: lbAfterReveal.jacket,
      item: lbAfterReveal.item,
      background: lbAfterReveal.background,
      committed: lbAfterReveal.committed,
      revealed: lbAfterReveal.revealed,
    });

    try {
      const miner: any = await (program.account as any).minerState.fetch(minerState);

      console.log("\n=== MINER STATE ===");
      console.log({
        minerPda: minerState.toBase58(),
        lootboxPda: lootbox.toBase58(),
        owner: miner.owner?.toBase58?.() ?? null,
        rarity: miner.rarity,
        element: miner.element,
        hashBase:
          miner.hashBase?.toString?.() ??
          miner.hashBase ??
          miner.hash_base?.toString?.() ??
          miner.hash_base ??
          null,
        face: miner.face ?? null,
        helmet: miner.helmet ?? null,
        backpack: miner.backpack ?? null,
        jacket: miner.jacket ?? null,
        item: miner.item ?? null,
        background: miner.background ?? null,
        allocatedLand:
          miner.allocatedLand?.toBase58?.() ??
          miner.allocated_land?.toBase58?.() ??
          null,
      });
    } catch (e: any) {
      console.log("[WARN] could not decode minerState via IDL:", e?.message ?? e);
    }

    try {
      const progress: any = await (program.account as any).minerProgress.fetch(minerProgress);

      console.log("\n=== MINER PROGRESS ===");
      console.log({
        minerProgressPda: minerProgress.toBase58(),
        miner: progress.miner?.toBase58?.() ?? null,
        owner: progress.owner?.toBase58?.() ?? null,
        level: progress.level?.toString?.() ?? progress.level,
        exp: progress.exp?.toString?.() ?? progress.exp ?? null,
      });
    } catch (e: any) {
      console.log("[WARN] could not decode minerProgress via IDL:", e?.message ?? e);
    }

    console.log("Miner revealed");
  });
});
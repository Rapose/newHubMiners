use solana_program::hash::hashv;

pub fn rng32(slot_hash_32: &[u8; 32], commitment_32: &[u8; 32], extra: u64) -> [u8; 32] {
    let extra_le = extra.to_le_bytes();
    hashv(&[&slot_hash_32[..], &commitment_32[..], &extra_le[..]]).to_bytes()
}

pub fn pick_u8(bytes: &[u8; 32], idx: usize, max: u8) -> u8 {
    if max == 0 {
        return 0;
    }
    bytes[idx % 32] % max
}

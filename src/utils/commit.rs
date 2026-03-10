use solana_program::hash::hashv;

pub fn commitment(domain: &[u8], lootbox_id: u64, salt32: &[u8; 32]) -> [u8; 32] {
    let id_le = lootbox_id.to_le_bytes();
    let h = hashv(&[domain, &id_le, salt32]);
    h.to_bytes()
}

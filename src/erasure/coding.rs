use std::lazy::Lazy;

use anyhow::bail;
use lazy_static::lazy_static;
use reed_solomon_erasure::galois_8::ReedSolomon;
use reed_solomon_erasure::Error::*;

pub struct Erasure {
    encoder: Lazy<ReedSolomon, Box<dyn FnOnce() -> ReedSolomon>>,
    data_blocks: usize,
    parity_blocks: usize,
    block_size: usize,
}

impl Erasure {
    pub fn new<F: FnOnce() -> ReedSolomon>(
        data_blocks: usize,
        parity_blocks: usize,
        block_size: usize,
    ) -> anyhow::Result<Erasure> {
        if data_blocks == 0 {
            return Err(TooFewDataShards.into());
        }
        if parity_blocks == 0 {
            return Err(TooFewParityShards.into());
        }
        if data_blocks + parity_blocks > 256 {
            return Err(TooManyShards.into());
        }
        Ok(Erasure {
            encoder: Lazy::new(Box::new(move || {
                // Safety: parameters validated.
                ReedSolomon::new(data_blocks, parity_blocks).unwrap()
            })),
            data_blocks,
            parity_blocks,
            block_size,
        })
    }

    pub fn shard_size(&self) -> usize {
        crate::utils::ceil_frac(self.block_size as isize, self.data_blocks as isize) as usize
    }

    pub fn shard_file_size(&self, total_length: usize) -> usize {
        if total_length == 0 {
            return 0;
        }
        let num_shards = total_length / self.block_size;
        let last_block_size = total_length % self.block_size;
        let last_shard_size =
            crate::utils::ceil_frac(last_block_size as isize, self.data_blocks as isize);
        return num_shards * self.shard_size() + last_shard_size as usize;
    }

    pub fn shard_file_offset(
        &self,
        start_offset: usize,
        length: usize,
        total_length: usize,
    ) -> usize {
        let shard_size = self.shard_size();
        let shard_file_size = self.shard_file_size(total_length);
        let end_shard = (start_offset + length) / self.block_size;
        let mut till_offset = end_shard * shard_size + shard_size;
        if till_offset > shard_file_size {
            till_offset = shard_file_size;
        }
        till_offset
    }

    pub fn encode_data<'a>(&self, data: &'a mut Vec<u8>) -> anyhow::Result<Vec<&'a mut [u8]>> {
        let mut data = self.split(data)?;
        self.encoder.encode(&mut data)?;
        Ok(data)
    }

    pub fn decode_data_blocks(&self, data: &mut Vec<Option<Vec<u8>>>) -> anyhow::Result<()> {
        if !data.iter().any(|v| v.is_none()) {
            // No need to reconstruct.
            return Ok(());
        }
        self.encoder.reconstruct_data(data)?;
        Ok(())
    }

    pub fn decode_data_and_parity_blocks(
        &self,
        data: &mut Vec<Option<Vec<u8>>>,
    ) -> anyhow::Result<()> {
        self.encoder.reconstruct(data)?;
        Ok(())
    }

    fn split<'a>(&self, data: &'a mut Vec<u8>) -> anyhow::Result<Vec<&'a mut [u8]>> {
        if data.len() == 0 {
            bail!("Not enough data to fill the number of requested shards");
        }
        // Calculate number of bytes per data shard.
        let per_shard = (data.len() + self.data_blocks - 1) / self.data_blocks;

        let shards = self.encoder.total_shard_count();
        // Only allocate memory if necessary.
        if data.len() < per_shard * shards {
            data.resize(per_shard * shards, 0u8);
        }

        // Split into equal-length shards.
        let mut data = &mut data[..];
        let mut dst = Vec::with_capacity(shards);
        for _ in 0..shards {
            let (d1, d2) = data.split_at_mut(per_shard);
            dst.push(d1);
            data = d2;
        }
        debug_assert_eq!(dst.len(), shards);
        Ok(dst)
    }
}

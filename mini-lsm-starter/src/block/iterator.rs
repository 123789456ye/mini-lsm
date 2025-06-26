// Copyright (c) 2022-2025 Alex Chi Z
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![allow(unused_variables)] // TODO(you): remove this lint after implementing this mod
#![allow(dead_code)] // TODO(you): remove this lint after implementing this mod

use std::sync::Arc;

use crate::key::{KeySlice, KeyVec};

use super::Block;

/// Iterates on a block.
pub struct BlockIterator {
    /// The internal `Block`, wrapped by an `Arc`
    block: Arc<Block>,
    /// The current key, empty represents the iterator is invalid
    key: KeyVec,
    /// the current value range in the block.data, corresponds to the current key
    value_range: (usize, usize),
    /// Current index of the key-value pair, should be in range of [0, num_of_elements)
    idx: usize,
    /// The first key in the block
    first_key: KeyVec,
}

impl BlockIterator {
    fn new(block: Arc<Block>) -> Self {
        Self {
            block,
            key: KeyVec::new(),
            value_range: (0, 0),
            idx: 0,
            first_key: KeyVec::new(),
        }
    }

    fn fetch_first_key(&mut self) {
        if self.first_key.is_empty() {
            (self.first_key, ..) = self.decode_entry(&self.block.data, 0);
        }
    }

    /// Creates a block iterator and seek to the first entry.
    pub fn create_and_seek_to_first(block: Arc<Block>) -> Self {
        let mut it = BlockIterator::new(block);
        it.seek_to_first();
        it
    }

    /// Creates a block iterator and seek to the first key that >= `key`.
    pub fn create_and_seek_to_key(block: Arc<Block>, key: KeySlice) -> Self {
        let mut it = BlockIterator::new(block);
        it.seek_to_key(key);
        it
    }

    /// Returns the key of the current entry.
    pub fn key(&self) -> KeySlice {
        self.key.as_key_slice()
    }

    /// Returns the value of the current entry.
    pub fn value(&self) -> &[u8] {
        &self.block.data[self.value_range.0..self.value_range.1]
    }

    /// Returns true if the iterator is valid.
    /// Note: You may want to make use of `key`
    pub fn is_valid(&self) -> bool {
        !self.key.is_empty()
    }

    /// Seeks to the first key in the block.
    pub fn seek_to_first(&mut self) {
        if self.block.offsets.is_empty() {
            self.key = KeyVec::new();
            self.value_range = (0, 0);
            self.idx = 0;
            return;
        }
        self.fetch_first_key();
        let offset = self.block.offsets[0] as usize;
        let (key, val_range) = self.decode_entry(&self.block.data, offset);

        self.key = key;
        self.value_range = val_range;
        self.idx = 0;
    }

    /// Move to the next key in the block.
    pub fn next(&mut self) {
        self.idx += 1;
        if self.idx >= self.block.offsets.len() {
            self.key = KeyVec::new();
            self.value_range = (0, 0);
            return;
        }
        let offset = self.block.offsets[self.idx] as usize;
        let (key, value_range) = self.decode_entry(&self.block.data, offset);
        self.key = key;
        self.value_range = value_range;
    }

    /// Seek to the first key that >= `key`.
    /// Note: You should assume the key-value pairs in the block are sorted when being added by
    /// callers.
    pub fn seek_to_key(&mut self, key: KeySlice) {
        self.fetch_first_key();

        let mut l = 0;
        let mut r = self.block.offsets.len();

        while l < r {
            let m = (l + r) / 2;
            let offset = self.block.offsets[m] as usize;
            let (mid_key, _) = self.decode_entry(&self.block.data, offset);
            match mid_key.key_ref().cmp(key.key_ref()) {
                std::cmp::Ordering::Less => {
                    l = m + 1;
                }
                _ => {
                    r = m;
                }
            }
        }
        if l < self.block.offsets.len() {
            let offset = self.block.offsets[l] as usize;
            let (key, val_range) = self.decode_entry(&self.block.data, offset);
            self.idx = l;
            self.key = key;
            self.value_range = val_range;
        } else {
            self.key = KeyVec::new();
            self.value_range = (0, 0);
            self.idx = l;
        }
    }

    fn decode_entry(&self, data: &[u8], offset: usize) -> (KeyVec, (usize, usize)) {
        let overlap = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
        let rest_len = u16::from_le_bytes([data[offset + 2], data[offset + 3]]) as usize;
        let rest_start = offset + 4;
        let rest_end = rest_start + rest_len;

        let mut full_key = self.first_key.key_ref()[..overlap].to_vec();
        full_key.extend_from_slice(&data[rest_start..rest_end]);
        let ts = u64::from_le_bytes(data[rest_end..rest_end + 8].try_into().unwrap());
        let rest_end = rest_end + 8;

        let vlen_offset = rest_end;
        let vlen = u16::from_le_bytes([data[vlen_offset], data[vlen_offset + 1]]) as usize;
        let value_start = vlen_offset + 2;
        let value_end = value_start + vlen;

        (
            KeyVec::from_vec_with_ts(full_key, ts),
            (value_start, value_end),
        )
    }
}

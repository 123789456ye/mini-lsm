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

use anyhow::Result;

use super::StorageIterator;

/// Merges two iterators of different types into one. If the two iterators have the same key, only
/// produce the key once and prefer the entry from A.
pub struct TwoMergeIterator<A: StorageIterator, B: StorageIterator> {
    a: A,
    b: B,
    a_valid: bool,
    b_valid: bool,
    choose_a: bool,
    // Add fields as need
}

impl<
    A: 'static + StorageIterator,
    B: 'static + for<'a> StorageIterator<KeyType<'a> = A::KeyType<'a>>,
> TwoMergeIterator<A, B>
{
    pub fn create(a: A, b: B) -> Result<Self> {
        let a_valid = a.is_valid();
        let b_valid = b.is_valid();
        Ok(TwoMergeIterator {
            a_valid,
            b_valid,
            choose_a: match (a_valid, b_valid) {
                (true, true) => a.key() <= b.key(),
                (true, false) => true,
                (false, true) => false,
                (false, false) => false,
            },
            a,
            b,
        })
    }
}

impl<
    A: 'static + StorageIterator,
    B: 'static + for<'a> StorageIterator<KeyType<'a> = A::KeyType<'a>>,
> StorageIterator for TwoMergeIterator<A, B>
{
    type KeyType<'a> = A::KeyType<'a>;

    fn key(&self) -> Self::KeyType<'_> {
        if self.choose_a {
            self.a.key()
        } else {
            self.b.key()
        }
    }

    fn value(&self) -> &[u8] {
        if self.choose_a {
            self.a.value()
        } else {
            self.b.value()
        }
    }

    fn is_valid(&self) -> bool {
        self.a_valid || self.b_valid
    }

    fn next(&mut self) -> Result<()> {
        match (self.a_valid, self.b_valid) {
            (true, true) => {
                if self.a.key() < self.b.key() {
                    self.a.next()?;
                    self.a_valid = self.a.is_valid();
                } else if self.a.key() > self.b.key() {
                    self.b.next()?;
                    self.b_valid = self.b.is_valid();
                } else {
                    self.a.next()?;
                    self.a_valid = self.a.is_valid();
                    self.b.next()?;
                    self.b_valid = self.b.is_valid();
                }
            }
            (true, false) => {
                self.a.next()?;
                self.a_valid = self.a.is_valid();
            }
            (false, true) => {
                self.b.next()?;
                self.b_valid = self.b.is_valid();
            }
            (false, false) => {}
        }
        self.choose_a = match (self.a_valid, self.b_valid) {
            (true, true) => self.a.key() <= self.b.key(),
            (true, false) => true,
            (false, true) => false,
            (false, false) => false,
        };

        Ok(())
    }
}

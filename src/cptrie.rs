// Copyright Mozilla Foundation
//
// Licensed under the Apache License (Version 2.0), or the MIT license,
// (the "Licenses") at your option. You may not use this file except in
// compliance with one of the Licenses. You may obtain copies of the
// Licenses at:
//
//    https://www.apache.org/licenses/LICENSE-2.0
//    https://opensource.org/licenses/MIT
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the Licenses is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the Licenses for the specific language governing permissions and
// limitations under the Licenses.

use crate::generic::{GenericUtf8Iter, Utf8Action};
use crate::Utf8CharIndicesWithTrie;
use core::iter::FusedIterator;
use core::marker::PhantomData;
use icu_collections::codepointtrie::AbstractCodePointTrie;
use icu_collections::codepointtrie::TrieValue;
use icu_collections::codepointtrie::WithTrie;

#[derive(Debug)]
struct Utf8CharsWithTrieAction<'trie, T, V>
where
    V: TrieValue,
    T: AbstractCodePointTrie<'trie, V>,
{
    trie: &'trie T,
    phantom: PhantomData<V>,
}

impl<'trie, T, V> Clone for Utf8CharsWithTrieAction<'trie, T, V>
where
    V: TrieValue,
    T: AbstractCodePointTrie<'trie, V>,
{
    fn clone(&self) -> Self {
        Self {
            trie: self.trie,
            phantom: PhantomData,
        }
    }
}

impl<'trie, T, V> Utf8Action for Utf8CharsWithTrieAction<'trie, T, V>
where
    V: TrieValue,
    T: AbstractCodePointTrie<'trie, V>,
{
    type Output = (char, V);

    #[inline(always)]
    unsafe fn handle_ascii(&mut self, ascii: u8) -> Self::Output {
        // SAFETY: The `Utf8Action` trait contract guarantees that `ascii` is a valid
        // ASCII byte (range `00..=7F`).
        (char::from(ascii), unsafe { self.trie.ascii(ascii) })
    }

    #[inline(always)]
    unsafe fn handle_2_byte(&mut self, b1: u8, b2: u8) -> Self::Output {
        // SAFETY: The `Utf8Action` trait contract guarantees that `b1` and `b2`
        // form a valid 2-byte UTF-8 sequence.
        let high_five = u32::from(b1) & 0b11_111;
        let low_six = u32::from(b2) & 0b111_111;
        // SAFETY: `high_five` and `low_six` conform to the
        // precondition of `utf8_two_byte` by construction.
        let v = unsafe { self.trie.utf8_two_byte(high_five, low_six) };
        let point = (high_five << 6) | low_six;
        // SAFETY: The resulting `point` is a valid code point by the trait contract.
        (unsafe { char::from_u32_unchecked(point) }, v)
    }

    #[inline(always)]
    unsafe fn handle_3_byte(&mut self, b1: u8, b2: u8, b3: u8) -> Self::Output {
        // SAFETY: The `Utf8Action` trait contract guarantees that `b1`, `b2`, and `b3`
        // form a valid 3-byte UTF-8 sequence.
        let high_ten = ((u32::from(b1) & 0b1111) << 6) | (u32::from(b2) & 0b111_111);
        let low_six = u32::from(b3) & 0b111_111;
        // SAFETY: `high_ten` and `low_six` conform to the
        // precondition of `utf8_three_byte` by construction.
        let v = unsafe { self.trie.utf8_three_byte(high_ten, low_six) };
        let point = (high_ten << 6) | low_six;
        // SAFETY: The resulting `point` is a valid code point by the trait contract.
        (unsafe { char::from_u32_unchecked(point) }, v)
    }

    #[inline(always)]
    unsafe fn handle_4_byte(&mut self, b1: u8, b2: u8, b3: u8, b4: u8) -> Self::Output {
        // SAFETY: The `Utf8Action` trait contract guarantees that `b1`, `b2`, `b3`, and `b4`
        // form a valid 4-byte UTF-8 sequence.
        let point = ((u32::from(b1) & 0x7) << 18)
            | ((u32::from(b2) & 0x3F) << 12)
            | ((u32::from(b3) & 0x3F) << 6)
            | (u32::from(b4) & 0x3F);
        // SAFETY: The resulting `point` is a valid code point by the trait contract.
        (
            unsafe { char::from_u32_unchecked(point) },
            self.trie.supplementary(point),
        )
    }

    #[inline(always)]
    fn handle_error(&mut self) -> Self::Output {
        ('\u{FFFD}', self.trie.bmp(0xFFFD))
    }
}

/// Iterator by `char` and `icu_collections::codepointtrie::TrieValue`
/// over `&[u8]` that contains potentially-invalid UTF-8. See the
/// crate documentation.
#[derive(Debug)]
pub struct Utf8CharsWithTrie<'slice, 'trie, T, V>
where
    V: TrieValue,
    T: AbstractCodePointTrie<'trie, V>,
{
    inner: GenericUtf8Iter<'slice, Utf8CharsWithTrieAction<'trie, T, V>>,
}

impl<'slice, 'trie, T, V> Clone for Utf8CharsWithTrie<'slice, 'trie, T, V>
where
    V: TrieValue,
    T: AbstractCodePointTrie<'trie, V>,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<'slice, 'trie, T, V> Utf8CharsWithTrie<'slice, 'trie, T, V>
where
    V: TrieValue,
    T: AbstractCodePointTrie<'trie, V>,
{
    #[inline(always)]
    /// Creates the iterator from a byte slice.
    pub fn new(bytes: &'slice [u8], trie: &'trie T) -> Self {
        Self {
            inner: GenericUtf8Iter::new(
                bytes,
                Utf8CharsWithTrieAction {
                    trie,
                    phantom: PhantomData,
                },
            ),
        }
    }

    /// Views the current remaining data in the iterator as a subslice
    /// of the original slice.
    #[inline(always)]
    pub fn as_slice(&self) -> &'slice [u8] {
        self.inner.as_slice()
    }
}

impl<'slice, 'trie, T, V> WithTrie<'trie, T, V> for Utf8CharsWithTrie<'slice, 'trie, T, V>
where
    V: TrieValue,
    T: AbstractCodePointTrie<'trie, V>,
{
    #[inline]
    fn trie(&self) -> &'trie T {
        self.inner.action.trie
    }
}

impl<'slice, 'trie, T, V> Iterator for Utf8CharsWithTrie<'slice, 'trie, T, V>
where
    V: TrieValue,
    T: AbstractCodePointTrie<'trie, V>,
{
    type Item = (char, V);

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'slice, 'trie, T, V> DoubleEndedIterator for Utf8CharsWithTrie<'slice, 'trie, T, V>
where
    V: TrieValue,
    T: AbstractCodePointTrie<'trie, V>,
{
    #[inline(always)]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

impl<'slice, 'trie, T, V> FusedIterator for Utf8CharsWithTrie<'slice, 'trie, T, V>
where
    V: TrieValue,
    T: AbstractCodePointTrie<'trie, V>,
{
}

/// Convenience trait that adds `chars_with_trie()` and `char_indices_with_trie()` methods
/// similar to the ones `icu_collections::codepointtrie::CharsWithTrieEx` adds to string
/// slices to `u8` slices.
pub trait Utf8CharsWithTrieEx<'slice, 'trie, T, V>
where
    V: TrieValue,
    T: AbstractCodePointTrie<'trie, V>,
{
    /// Convenience method for creating an UTF-16 iterator
    /// with trie values for the slice.
    fn chars_with_trie(&'slice self, trie: &'trie T) -> Utf8CharsWithTrie<'slice, 'trie, T, V>;
    /// Convenience method for creating a code unit index and
    /// UTF-16 iterator with trie values for the slice.
    fn char_indices_with_trie(
        &'slice self,
        trie: &'trie T,
    ) -> Utf8CharIndicesWithTrie<'slice, 'trie, T, V>;
}

impl<'slice, 'trie, T, V> Utf8CharsWithTrieEx<'slice, 'trie, T, V> for [u8]
where
    V: TrieValue,
    T: AbstractCodePointTrie<'trie, V>,
{
    /// Convenience method for creating an UTF-16 iterator
    /// with trie values for the slice.
    #[inline]
    fn chars_with_trie(&'slice self, trie: &'trie T) -> Utf8CharsWithTrie<'slice, 'trie, T, V> {
        Utf8CharsWithTrie::new(self, trie)
    }

    /// Convenience method for creating a code unit index and
    /// UTF-16 iterator with trie values for the slice.
    #[inline]
    fn char_indices_with_trie(
        &'slice self,
        trie: &'trie T,
    ) -> Utf8CharIndicesWithTrie<'slice, 'trie, T, V> {
        Utf8CharIndicesWithTrie::new(self, trie)
    }
}

// --

#[derive(Debug)]
struct Utf8CharsWithTrieDefaultForAsciiAction<'trie, T, V>
where
    V: TrieValue + Default,
    T: AbstractCodePointTrie<'trie, V>,
{
    trie: &'trie T,
    phantom: PhantomData<V>,
}

impl<'trie, T, V> Clone for Utf8CharsWithTrieDefaultForAsciiAction<'trie, T, V>
where
    V: TrieValue + Default,
    T: AbstractCodePointTrie<'trie, V>,
{
    fn clone(&self) -> Self {
        Self {
            trie: self.trie,
            phantom: PhantomData,
        }
    }
}

impl<'trie, T, V> Utf8Action for Utf8CharsWithTrieDefaultForAsciiAction<'trie, T, V>
where
    V: TrieValue + Default,
    T: AbstractCodePointTrie<'trie, V>,
{
    type Output = (char, V);

    #[inline(always)]
    unsafe fn handle_ascii(&mut self, ascii: u8) -> Self::Output {
        // SAFETY: The `Utf8Action` trait contract guarantees that `ascii` is a valid
        // ASCII byte (range `00..=7F`).
        (char::from(ascii), V::default())
    }

    #[inline(always)]
    unsafe fn handle_2_byte(&mut self, b1: u8, b2: u8) -> Self::Output {
        // SAFETY: The `Utf8Action` trait contract guarantees that `b1` and `b2`
        // form a valid 2-byte UTF-8 sequence.
        let high_five = u32::from(b1) & 0b11_111;
        let low_six = u32::from(b2) & 0b111_111;
        // SAFETY: `high_five` and `low_six` conform to the
        // precondition of `utf8_two_byte` by construction.
        let v = unsafe { self.trie.utf8_two_byte(high_five, low_six) };
        let point = (high_five << 6) | low_six;
        // SAFETY: The resulting `point` is a valid code point by the trait contract.
        (unsafe { char::from_u32_unchecked(point) }, v)
    }

    #[inline(always)]
    unsafe fn handle_3_byte(&mut self, b1: u8, b2: u8, b3: u8) -> Self::Output {
        // SAFETY: The `Utf8Action` trait contract guarantees that `b1`, `b2`, and `b3`
        // form a valid 3-byte UTF-8 sequence.
        let high_ten = ((u32::from(b1) & 0b1111) << 6) | (u32::from(b2) & 0b111_111);
        let low_six = u32::from(b3) & 0b111_111;
        // SAFETY: `high_ten` and `low_six` conform to the
        // precondition of `utf8_three_byte` by construction.
        let v = unsafe { self.trie.utf8_three_byte(high_ten, low_six) };
        let point = (high_ten << 6) | low_six;
        // SAFETY: The resulting `point` is a valid code point by the trait contract.
        (unsafe { char::from_u32_unchecked(point) }, v)
    }

    #[inline(always)]
    unsafe fn handle_4_byte(&mut self, b1: u8, b2: u8, b3: u8, b4: u8) -> Self::Output {
        // SAFETY: The `Utf8Action` trait contract guarantees that `b1`, `b2`, `b3`, and `b4`
        // form a valid 4-byte UTF-8 sequence.
        let point = ((u32::from(b1) & 0x7) << 18)
            | ((u32::from(b2) & 0x3F) << 12)
            | ((u32::from(b3) & 0x3F) << 6)
            | (u32::from(b4) & 0x3F);
        // SAFETY: The resulting `point` is a valid code point by the trait contract.
        (
            unsafe { char::from_u32_unchecked(point) },
            self.trie.supplementary(point),
        )
    }

    #[inline(always)]
    fn handle_error(&mut self) -> Self::Output {
        ('\u{FFFD}', self.trie.bmp(0xFFFD))
    }
}

/// Iterator by `char` and `icu_collections::codepointtrie::TrieValue`
/// over `&[u8]` that contains potentially-invalid UTF-8. Uses `V::default()`
/// for ASCII instead of reading from the trie. See the
/// crate documentation.
#[derive(Debug)]
pub struct Utf8CharsWithTrieDefaultForAscii<'slice, 'trie, T, V>
where
    V: TrieValue + Default,
    T: AbstractCodePointTrie<'trie, V>,
{
    inner: GenericUtf8Iter<'slice, Utf8CharsWithTrieDefaultForAsciiAction<'trie, T, V>>,
}

impl<'slice, 'trie, T, V> Clone for Utf8CharsWithTrieDefaultForAscii<'slice, 'trie, T, V>
where
    V: TrieValue + Default,
    T: AbstractCodePointTrie<'trie, V>,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<'slice, 'trie, T, V> Utf8CharsWithTrieDefaultForAscii<'slice, 'trie, T, V>
where
    V: TrieValue + Default,
    T: AbstractCodePointTrie<'trie, V>,
{
    #[inline(always)]
    /// Creates the iterator from a byte slice.
    pub fn new(bytes: &'slice [u8], trie: &'trie T) -> Self {
        Self {
            inner: GenericUtf8Iter::new(
                bytes,
                Utf8CharsWithTrieDefaultForAsciiAction {
                    trie,
                    phantom: PhantomData,
                },
            ),
        }
    }

    /// Views the current remaining data in the iterator as a subslice
    /// of the original slice.
    #[inline(always)]
    pub fn as_slice(&self) -> &'slice [u8] {
        self.inner.as_slice()
    }
}

impl<'slice, 'trie, T, V> WithTrie<'trie, T, V>
    for Utf8CharsWithTrieDefaultForAscii<'slice, 'trie, T, V>
where
    V: TrieValue + Default,
    T: AbstractCodePointTrie<'trie, V>,
{
    #[inline]
    fn trie(&self) -> &'trie T {
        self.inner.action.trie
    }
}

impl<'slice, 'trie, T, V> Iterator for Utf8CharsWithTrieDefaultForAscii<'slice, 'trie, T, V>
where
    V: TrieValue + Default,
    T: AbstractCodePointTrie<'trie, V>,
{
    type Item = (char, V);

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'slice, 'trie, T, V> DoubleEndedIterator
    for Utf8CharsWithTrieDefaultForAscii<'slice, 'trie, T, V>
where
    V: TrieValue + Default,
    T: AbstractCodePointTrie<'trie, V>,
{
    #[inline(always)]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

impl<'slice, 'trie, T, V> FusedIterator for Utf8CharsWithTrieDefaultForAscii<'slice, 'trie, T, V>
where
    V: TrieValue + Default,
    T: AbstractCodePointTrie<'trie, V>,
{
}

/// Convenience trait that adds `chars_with_trie_default_for_ascii()` and `char_indices_with_trie_default_for_ascii()` methods
/// similar to the ones `icu_collections::codepointtrie::CharsWithTrieEx` adds to string
/// slices to `u8` slices.
pub trait Utf8CharsWithTrieDefaultForAsciiEx<'slice, 'trie, T, V>
where
    V: TrieValue + Default,
    T: AbstractCodePointTrie<'trie, V>,
{
    /// Convenience method for creating an UTF-16 iterator
    /// with trie values for the slice.
    fn chars_with_trie_default_for_ascii(
        &'slice self,
        trie: &'trie T,
    ) -> Utf8CharsWithTrieDefaultForAscii<'slice, 'trie, T, V>;
    /// Convenience method for creating a code unit index and
    /// UTF-16 iterator with trie values for the slice.
    fn char_indices_with_trie_default_for_ascii(
        &'slice self,
        trie: &'trie T,
    ) -> Utf8CharIndicesWithTrie<'slice, 'trie, T, V>;
}

impl<'slice, 'trie, T, V> Utf8CharsWithTrieDefaultForAsciiEx<'slice, 'trie, T, V> for [u8]
where
    V: TrieValue + Default,
    T: AbstractCodePointTrie<'trie, V>,
{
    /// Convenience method for creating an UTF-16 iterator
    /// with trie values for the slice.
    #[inline]
    fn chars_with_trie_default_for_ascii(
        &'slice self,
        trie: &'trie T,
    ) -> Utf8CharsWithTrieDefaultForAscii<'slice, 'trie, T, V> {
        Utf8CharsWithTrieDefaultForAscii::new(self, trie)
    }

    /// Convenience method for creating a code unit index and
    /// UTF-16 iterator with trie values for the slice.
    #[inline]
    fn char_indices_with_trie_default_for_ascii(
        &'slice self,
        trie: &'trie T,
    ) -> Utf8CharIndicesWithTrie<'slice, 'trie, T, V> {
        Utf8CharIndicesWithTrie::new(self, trie)
    }
}

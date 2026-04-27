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

#![no_std]

//! Provides iteration by `char` over `&[u8]` containing potentially-invalid
//! UTF-8 such that errors are handled according to the [WHATWG Encoding
//! Standard](https://encoding.spec.whatwg.org/#utf-8-decoder) (i.e. the same
//! way as in `String::from_utf8_lossy`).
//!
//! The trait `Utf8CharsEx` provides the convenience method `chars()` on
//! byte slices themselves instead of having to use the more verbose
//! `Utf8Chars::new(slice)`.
//!
//! ```rust
//! use utf8_iter::Utf8CharsEx;
//! let data = b"\xFF\xC2\xE2\xE2\x98\xF0\xF0\x9F\xF0\x9F\x92\xE2\x98\x83";
//! let from_iter: String = data.chars().collect();
//! let from_std = String::from_utf8_lossy(data);
//! assert_eq!(from_iter, from_std);
//! ```

#[cfg(feature = "icu_collections")]
mod cptrie;
#[cfg(feature = "icu_collections")]
mod cptrie_indices;
mod generic;
mod indices;
mod report;

pub use crate::generic::{GenericUtf8Iter, Utf8Action};

#[cfg(feature = "icu_collections")]
pub use crate::cptrie::Utf8CharsWithTrie;
#[cfg(feature = "icu_collections")]
pub use crate::cptrie::Utf8CharsWithTrieDefaultForAscii;
#[cfg(feature = "icu_collections")]
pub use crate::cptrie::Utf8CharsWithTrieDefaultForAsciiEx;
#[cfg(feature = "icu_collections")]
pub use crate::cptrie::Utf8CharsWithTrieEx;
#[cfg(feature = "icu_collections")]
pub use crate::cptrie_indices::Utf8CharIndicesWithTrie;
#[cfg(feature = "icu_collections")]
pub use crate::cptrie_indices::Utf8CharIndicesWithTrieDefaultForAscii;
pub use crate::indices::Utf8CharIndices;
pub use crate::report::ErrorReportingUtf8Chars;
pub use crate::report::Utf8CharsError;
use core::iter::FusedIterator;

#[repr(align(64))] // Align to cache lines
struct Utf8Data {
    pub table: [u8; 384],
}

// This is generated code copied and pasted from utf_8.rs of encoding_rs.
// Please don't edit by hand but instead regenerate as instructed in that
// file.

pub(crate) static UTF8_DATA: Utf8Data = Utf8Data {
    table: [
        252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252,
        252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252,
        252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252,
        252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252,
        252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252,
        252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252,
        252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252,
        252, 252, 84, 84, 84, 84, 84, 84, 84, 84, 84, 84, 84, 84, 84, 84, 84, 84, 148, 148, 148,
        148, 148, 148, 148, 148, 148, 148, 148, 148, 148, 148, 148, 148, 164, 164, 164, 164, 164,
        164, 164, 164, 164, 164, 164, 164, 164, 164, 164, 164, 164, 164, 164, 164, 164, 164, 164,
        164, 164, 164, 164, 164, 164, 164, 164, 164, 252, 252, 252, 252, 252, 252, 252, 252, 252,
        252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252,
        252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252,
        252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252,
        252, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
        8, 8, 8, 8, 8, 8, 8, 16, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 32, 8, 8, 64, 8, 8, 8, 128, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
    ],
};

// End manually copypasted generated code.

#[inline(always)]
pub(crate) fn in_inclusive_range8(i: u8, start: u8, end: u8) -> bool {
    i.wrapping_sub(start) <= (end - start)
}

/// Iterator by `char` over `&[u8]` that contains
/// potentially-invalid UTF-8. See the crate documentation.
#[derive(Debug, Clone)]
pub struct Utf8Chars<'a> {
    inner: GenericUtf8Iter<'a, Utf8CharsAction>,
}

#[derive(Debug, Clone, Copy)]
struct Utf8CharsAction;

impl Utf8Action for Utf8CharsAction {
    type Output = char;

    #[inline(always)]
    unsafe fn handle_ascii(&mut self, ascii: u8) -> Self::Output {
        debug_assert!(ascii < 0x80);
        char::from(ascii)
    }

    #[inline(always)]
    unsafe fn handle_2_byte(&mut self, b1: u8, b2: u8) -> Self::Output {
        // SAFETY: The `Utf8Action` trait contract guarantees that `b1` and `b2`
        // form a valid 2-byte UTF-8 sequence, which means the resulting scalar
        // value is a valid code point in the range `U+0080..=U+07FF`.
        let point = ((u32::from(b1) & 0x1F) << 6) | (u32::from(b2) & 0x3F);
        debug_assert!(core::char::from_u32(point).is_some());
        unsafe { char::from_u32_unchecked(point) }
    }

    #[inline(always)]
    unsafe fn handle_3_byte(&mut self, b1: u8, b2: u8, b3: u8) -> Self::Output {
        // SAFETY: The `Utf8Action` trait contract guarantees that `b1`, `b2`, and `b3`
        // form a valid 3-byte UTF-8 sequence, which means the resulting scalar
        // value is a valid code point in the range `U+0800..=U+FFFF` (excluding surrogates).
        let point = ((u32::from(b1) & 0xF) << 12)
            | ((u32::from(b2) & 0x3F) << 6)
            | (u32::from(b3) & 0x3F);
        debug_assert!(core::char::from_u32(point).is_some());
        unsafe { char::from_u32_unchecked(point) }
    }

    #[inline(always)]
    unsafe fn handle_4_byte(&mut self, b1: u8, b2: u8, b3: u8, b4: u8) -> Self::Output {
        // SAFETY: The `Utf8Action` trait contract guarantees that `b1`, `b2`, `b3`, and `b4`
        // form a valid 4-byte UTF-8 sequence, which means the resulting scalar
        // value is a valid code point in the range `U+10000..=U+10FFFF`.
        let point = ((u32::from(b1) & 0x7) << 18)
            | ((u32::from(b2) & 0x3F) << 12)
            | ((u32::from(b3) & 0x3F) << 6)
            | (u32::from(b4) & 0x3F);
        debug_assert!(core::char::from_u32(point).is_some());
        unsafe { char::from_u32_unchecked(point) }
    }

    #[inline(always)]
    fn handle_error(&mut self) -> Self::Output {
        '\u{FFFD}'
    }
}

impl<'a> Utf8Chars<'a> {
    #[inline(always)]
    /// Creates the iterator from a byte slice.
    pub fn new(bytes: &'a [u8]) -> Self {
        Utf8Chars::<'a> {
            inner: GenericUtf8Iter::new(bytes, Utf8CharsAction),
        }
    }

    /// Views the current remaining data in the iterator as a subslice
    /// of the original slice.
    #[inline(always)]
    pub fn as_slice(&self) -> &'a [u8] {
        self.inner.as_slice()
    }
}

impl<'a> Iterator for Utf8Chars<'a> {
    type Item = char;

    #[inline(always)]
    fn next(&mut self) -> Option<char> {
        self.inner.next()
    }
}

impl<'a> DoubleEndedIterator for Utf8Chars<'a> {
    #[inline(always)]
    fn next_back(&mut self) -> Option<char> {
        self.inner.next_back()
    }
}

impl FusedIterator for Utf8Chars<'_> {}

/// Convenience trait that adds `chars()` and `char_indices()` methods
/// similar to the ones on string slices to byte slices.
pub trait Utf8CharsEx {
    fn chars(&self) -> Utf8Chars<'_>;
    fn char_indices(&self) -> Utf8CharIndices<'_>;
}

impl Utf8CharsEx for [u8] {
    /// Convenience method for creating an UTF-8 iterator
    /// for the slice.
    #[inline]
    fn chars(&self) -> Utf8Chars<'_> {
        Utf8Chars::new(self)
    }
    /// Convenience method for creating a byte index and
    /// UTF-8 iterator for the slice.
    #[inline]
    fn char_indices(&self) -> Utf8CharIndices<'_> {
        Utf8CharIndices::new(self)
    }
}

// No manually-written tests for forward-iteration, because the code passed multiple
// days of fuzzing comparing with known-good behavior.

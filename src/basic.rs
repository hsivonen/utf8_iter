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

use crate::helpers::four_bytes_to_char;
use crate::helpers::three_bytes_to_char;
use crate::helpers::two_bytes_to_char;

use crate::Utf8Handler;

/// The basic `Output = char` case.
#[derive(Debug, Clone)]
pub(crate) struct DefaultHandler;

impl DefaultHandler {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl Utf8Handler for DefaultHandler {
    type Output = char;

    /// Map a single-byte UTF-8 sequence to `Output`.
    ///
    /// When `Output` is `char`, `char::from(ascii)`
    /// is the appropriate implementation.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `ascii` is
    /// below 0x80. The callers in `utf8_iter` guarantee
    /// this, but this is `unsafe` in case the trait
    /// implementation is used with other callers. The
    /// implementation of this method is expected to be
    /// declared `#[inline(always)]` and to rely on this
    /// invariant without checking it on release builds.
    #[inline(always)]
    unsafe fn single_byte(&self, ascii: u8) -> Self::Output {
        char::from(ascii)
    }

    /// Map a two-byte UTF-8 sequence to `Output`.
    ///
    /// When `Output` is `char`,
    /// `two_bytes_to_char(first, second)`
    /// is the appropriate implementation.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `first` and `second`
    /// form a valid two-byte UTF-8 sequence.
    /// The callers in `utf8_iter` guarantee
    /// this, but this is `unsafe` in case the trait
    /// implementation is used with other callers. The
    /// implementation of this method is expected to be
    /// declared `#[inline(always)]` and to rely on this
    /// invariant without checking it on release builds.
    #[inline(always)]
    unsafe fn two_byte(&self, first: u8, second: u8) -> Self::Output {
        two_bytes_to_char(first, second)
    }

    /// Map a three-byte UTF-8 sequence to `Output`.
    ///
    /// When `Output` is `char`,
    /// `three_bytes_to_char(first, second, third)`
    /// is the appropriate implementation.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `first`, `second`,
    /// and `third` form a valid three-byte UTF-8 sequence.
    /// The callers in `utf8_iter` guarantee
    /// this, but this is `unsafe` in case the trait
    /// implementation is used with other callers. The
    /// implementation of this method is expected to be
    /// declared `#[inline(always)]` and to rely on this
    /// invariant without checking it on release builds.
    #[inline(always)]
    unsafe fn three_byte(&self, first: u8, second: u8, third: u8) -> Self::Output {
        // SAFETY: We rely on the safety invariant of this method to hold.
        // The safety invariant of `three_bytes_to_char` is the same invariant.
        unsafe { three_bytes_to_char(first, second, third) }
    }

    /// Map a four-byte UTF-8 sequence to `Output`.
    ///
    /// When `Output` is `char`,
    /// `unsafe { four_bytes_to_char(first, second, third, fourth) }`
    /// is the appropriate implementation.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `first`, `second`,
    /// `third`, and `fourth` form a valid four-byte UTF-8 sequence.
    /// The callers in `utf8_iter` guarantee
    /// this, but this is `unsafe` in case the trait
    /// implementation is used with other callers. The
    /// implementation of this method is expected to be
    /// declared `#[inline(always)]` and to rely on this
    /// invariant without checking it on release builds.
    #[inline(always)]
    unsafe fn four_byte(&self, first: u8, second: u8, third: u8, fourth: u8) -> Self::Output {
        // SAFETY: We rely on the safety invariant of this method to hold.
        // The safety invariant of `four_bytes_to_char` is the same invariant.
        unsafe { four_bytes_to_char(first, second, third, fourth) }
    }

    /// Map a singe UTF-8 error to `Output`.
    ///
    /// What constitutes a single error is defined by the
    /// WHATWG Encoding Standard.
    ///
    /// When `Output` is `char`,
    /// `char::REPLACEMENT_CHARACTER`
    /// is the appropriate implementation.
    ///
    /// The implementation of this method is expected to
    /// be declared `#[inline(always)]`.
    #[inline(always)]
    fn error(&self) -> Self::Output {
        char::REPLACEMENT_CHARACTER
    }
}

crate::macros::named_iterators_from_no_argument_handler!(
    DefaultHandler,
    char,
    /// Iterator by `char` over `&[u8]` that contains
    /// potentially-invalid UTF-8. See the crate documentation.
    ,
    Utf8Chars,
    /// Iterator by `char` and their indices over `&[u8]` that contains
    /// potentially-invalid UTF-8. See the crate documentation.
    ,
    Utf8CharIndices,
);

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

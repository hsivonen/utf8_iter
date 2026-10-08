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

use utf_types::utf8::Ascii;
use utf_types::utf8::FourByteSequence;
use utf_types::utf8::ThreeByteSequence;
use utf_types::utf8::TwoByteSequence;
use utf_types::utf8::Utf8ByteSequence;

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
    /// When `Output` is `char`, `ascii.to_char()`
    /// is the appropriate implementation.
    ///
    /// The implementation is expected to be marked
    /// `#[inline(always)]`.
    #[inline(always)]
    fn single_byte(&self, ascii: Ascii) -> Self::Output {
        ascii.to_char()
    }

    /// Map a two-byte UTF-8 sequence to `Output`.
    ///
    /// When `Output` is `char`, `sequence.to_char()`
    /// is the appropriate implementation.
    ///
    /// The implementation is expected to be marked
    /// `#[inline(always)]`.
    #[inline(always)]
    fn two_byte(&self, sequence: TwoByteSequence) -> Self::Output {
        sequence.to_char()
    }

    /// Map a three-byte UTF-8 sequence to `Output`.
    ///
    /// When `Output` is `char`, `sequence.to_char()`
    /// is the appropriate implementation.
    ///
    /// The implementation is expected to be marked
    /// `#[inline(always)]`.
    #[inline(always)]
    fn three_byte(&self, sequence: ThreeByteSequence) -> Self::Output {
        sequence.to_char()
    }

    /// Map a four-byte UTF-8 sequence to `Output`.
    ///
    /// When `Output` is `char`, `sequence.to_char()`
    /// is the appropriate implementation.
    ///
    /// The implementation is expected to be marked
    /// `#[inline(always)]`.
    #[inline(always)]
    fn four_byte(&self, sequence: FourByteSequence) -> Self::Output {
        sequence.to_char()
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

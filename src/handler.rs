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

//! UTF-8 decoding with pluggable handler for the four byte sequence
//! lengths and error.

use core::iter::FusedIterator;

use crate::helpers::below_four_byte;
use crate::helpers::below_three_byte;
use crate::helpers::multi_byte_lead;
use crate::helpers::single_byte;
use crate::helpers::three_byte_prefix;
use crate::helpers::two_byte_lead;
use crate::helpers::two_byte_prefix;
use crate::helpers::unconstrained_continuation;

use utf_types::utf8::Ascii;
use utf_types::utf8::FourByteSequence;
use utf_types::utf8::PreparedThreeBytes;
use utf_types::utf8::PreparedTwoBytes;
use utf_types::utf8::ThreeByteSequence;
use utf_types::utf8::TwoByteSequence;

const SEQUENCE_FOR_REPLACEMENT_CHARACTER: ThreeByteSequence = const {
    if let Ok(sequence) = ThreeByteSequence::try_new(0xEF, 0xBF, 0xBD) {
        sequence
    } else {
        panic!()
    }
};

/// Mapping from the four kinds of byte sequences or error
/// to output.
pub trait Utf8Handler {
    /// The per-scalar-value output type. (In the common case,
    /// this is `char`.)
    type Output;

    /// Map a single-byte UTF-8 sequence to `Output`.
    ///
    /// When `Output` is `char`, `ascii.to_char()`
    /// is the appropriate implementation.
    ///
    /// The implementation is expected to be marked
    /// `#[inline(always)]`.
    fn single_byte(&self, ascii: Ascii) -> Self::Output;

    /// Map a two-byte UTF-8 sequence to `Output`.
    ///
    /// When `Output` is `char`, `sequence.to_char()`
    /// is the appropriate implementation.
    ///
    /// The implementation is expected to be marked
    /// `#[inline(always)]`.
    fn two_byte(&self, sequence: TwoByteSequence) -> Self::Output;

    /// Map a three-byte UTF-8 sequence to `Output`.
    ///
    /// When `Output` is `char`, `sequence.to_char()`
    /// is the appropriate implementation.
    ///
    /// The implementation is expected to be marked
    /// `#[inline(always)]`.
    fn three_byte(&self, sequence: ThreeByteSequence) -> Self::Output;

    /// Map a four-byte UTF-8 sequence to `Output`.
    ///
    /// When `Output` is `char`, `sequence.to_char()`
    /// is the appropriate implementation.
    ///
    /// The implementation is expected to be marked
    /// `#[inline(always)]`.
    fn four_byte(&self, sequence: FourByteSequence) -> Self::Output;

    /// Map a single UTF-8 error to `Output`.
    ///
    /// What constitutes a single error is defined by the
    /// WHATWG Encoding Standard.
    ///
    /// When `Output` is `char`,
    /// `char::REPLACEMENT_CHARACTER`
    /// is the appropriate implementation. The provided
    /// implementation delegates to `three_byte` by
    /// passing the three bytes that represent the
    /// REPLACEMENT CHARACTER.
    ///
    /// The implementation of this method is expected to
    /// be declared `#[inline(always)]`.
    #[inline(always)]
    fn error(&self) -> Self::Output {
        self.three_byte(SEQUENCE_FOR_REPLACEMENT_CHARACTER)
    }
}

/// Iterator by `char` over `&[u8]` that contains
/// potentially-invalid UTF-8 with a handler for the four
/// kinds of byte sequences (or error). See the crate documentation.
#[derive(Debug, Clone)]
pub struct Utf8CharsWithHandler<'a, H>
where
    H: Utf8Handler,
{
    remaining: &'a [u8],
    handler: H,
}

impl<'a, H> Utf8CharsWithHandler<'a, H>
where
    H: Utf8Handler,
{
    #[inline(always)]
    /// Creates the iterator from a byte slice and handler.
    pub fn new(bytes: &'a [u8], handler: H) -> Self {
        Utf8CharsWithHandler::<'a, H> {
            remaining: bytes,
            handler,
        }
    }

    /// Views the current remaining data in the iterator as a subslice
    /// of the original slice.
    #[inline(always)]
    pub fn as_slice(&self) -> &'a [u8] {
        self.remaining
    }

    /// Obtains a reference to the handler.
    #[inline(always)]
    pub fn handler(&self) -> &H {
        &self.handler
    }

    #[cold]
    fn error(&self) -> H::Output {
        self.handler.error()
    }

    #[cold]
    #[inline(never)]
    fn next_fallback(&mut self) -> Option<H::Output> {
        let (&first, after_one) = self.remaining.split_first()?;
        if let Ok(ascii) = Ascii::try_new(first) {
            self.remaining = after_one;
            return Some(self.handler.single_byte(ascii));
        }
        if let Some((&second, after_two)) = after_one.split_first() {
            let prepared_two = PreparedTwoBytes::new(first, second);
            if let Ok(two_byte) = TwoByteSequence::try_new_with_prepared(prepared_two) {
                self.remaining = after_two;
                return Some(self.handler.two_byte(two_byte));
            }
            if let Some((&third, after_three)) = after_two.split_first() {
                let prepared_three = PreparedThreeBytes::new_with_prepared(prepared_two, third);
                if let Ok(three_byte) = ThreeByteSequence::try_new_with_prepared(prepared_three) {
                    self.remaining = after_three;
                    return Some(self.handler.three_byte(three_byte));
                }
                // We have already determined that we don't have a single-byte,
                // two-byte, or three-byte sequence.
                // We cannot have a well-formed four-byte sequence, because if
                // we had one, `next` would have consumed it. Consume three bytes
                // if we have a three-byte prefix of a four-byte sequence.
                // Otherwise, fall through to consuming one byte.
                if prepared_three.sequence_prefix_assuming_not_well_formed_single_or_two_byte() {
                    self.remaining = after_three;
                    return Some(self.error());
                }
            }
            // We have already determined
            // that `first` is not ASCII and that `prepared_two` is not
            // a two-byte UTF-8 sequence. If it is a prefix of a three-byte
            // sequence or of a four-byte sequence, consume both bytes.
            // Otherwise, fall through to consuming one byte.
            if prepared_two.sequence_prefix_assuming_first_not_ascii() {
                self.remaining = after_two;
                return Some(self.error());
            }
        }
        self.remaining = after_one;
        Some(self.error())
    }

    #[cold]
    #[inline(never)]
    fn next_back_fallback(&mut self) -> Option<H::Output> {
        let (&last, before_one) = self.remaining.split_last()?;
        if let Ok(ascii) = Ascii::try_new(last) {
            self.remaining = before_one;
            return Some(self.handler.single_byte(ascii));
        }
        if let Some((&second_last, before_two)) = before_one.split_last() {
            let prepared_two = PreparedTwoBytes::new(second_last, last);
            if let Ok(two_byte) = TwoByteSequence::try_new_with_prepared(prepared_two) {
                self.remaining = before_two;
                return Some(self.handler.two_byte(two_byte));
            }
            if let Some((&third_last, before_three)) = before_two.split_last() {
                let prepared_three = PreparedThreeBytes::new(third_last, second_last, last);
                if let Ok(three_byte) = ThreeByteSequence::try_new_with_prepared(prepared_three) {
                    self.remaining = before_three;
                    return Some(self.handler.three_byte(three_byte));
                }
                // Consume three bytes if we have a three-byte prefix of a four-byte sequence.
                if prepared_three.prefix_of_four_byte() {
                    self.remaining = before_three;
                    return Some(self.error());
                }
            }
            // We've established that the two last bytes don't form a two-byte
            // UTF-8 sequence, but they could form a two-byte prefix of a
            // three-byte or four-byte sequence, in which case we need to consume
            // two bytes instead of falling through to consuming just one byte.
            // We have not established whether `second_last` is ASCII!
            if prepared_two.sequence_prefix() {
                self.remaining = before_two;
                return Some(self.error());
            }
        }
        self.remaining = before_one;
        Some(self.error())
    }
}

impl<'a, H> Iterator for Utf8CharsWithHandler<'a, H>
where
    H: Utf8Handler,
{
    type Item = H::Output;

    #[inline(always)]
    fn next(&mut self) -> Option<H::Output> {
        if self.remaining.len() >= 4 {
            // UNWRAP: Length checked at the start of the method.
            let (&first, after_one) = self.remaining.split_first().unwrap();
            if let Ok(ascii) = Ascii::try_new(first) {
                self.remaining = after_one;
                return Some(self.handler.single_byte(ascii));
            }
            // UNWRAP: Length checked at the start of the method.
            let (&second, after_two) = after_one.split_first().unwrap();
            if let Ok(two_byte) = TwoByteSequence::try_new(first, second) {
                self.remaining = after_two;
                return Some(self.handler.two_byte(two_byte));
            }
            // UNWRAP: Length checked at the start of the method.
            let (&third, after_three) = after_two.split_first().unwrap();
            let prepared_three = PreparedThreeBytes::new(first, second, third);
            if let Ok(three_byte) = ThreeByteSequence::try_new_with_prepared(prepared_three) {
                self.remaining = after_three;
                return Some(self.handler.three_byte(three_byte));
            }
            // UNWRAP: Length checked at the start of the method.
            let (&fourth, after_four) = after_three.split_first().unwrap();
            if let Ok(four_byte) = FourByteSequence::try_new_with_prepared(prepared_three, fourth) {
                self.remaining = after_four;
                return Some(self.handler.four_byte(four_byte));
            }
        }
        self.next_fallback()
    }
}

impl<'a, H> DoubleEndedIterator for Utf8CharsWithHandler<'a, H>
where
    H: Utf8Handler,
{
    #[inline(always)]
    fn next_back(&mut self) -> Option<H::Output> {
        if self.remaining.len() >= 4 {
            // UNWRAP: Length checked at the start of the method.
            let (&last, before_one) = self.remaining.split_last().unwrap();
            if let Ok(ascii) = Ascii::try_new(last) {
                self.remaining = before_one;
                return Some(self.handler.single_byte(ascii));
            }
            // UNWRAP: Length checked at the start of the method.
            let (&second_last, before_two) = before_one.split_last().unwrap();
            if let Ok(two_byte) = TwoByteSequence::try_new(second_last, last) {
                self.remaining = before_two;
                return Some(self.handler.two_byte(two_byte));
            }
            // UNWRAP: Length checked at the start of the method.
            let (&third_last, before_three) = before_two.split_last().unwrap();
            if let Ok(three_byte) = ThreeByteSequence::try_new(third_last, second_last, last) {
                self.remaining = before_three;
                return Some(self.handler.three_byte(three_byte));
            }
            // UNWRAP: Length checked at the start of the method.
            let (&fourth_last, before_four) = before_three.split_last().unwrap();
            if let Ok(four_byte) =
                FourByteSequence::try_new(fourth_last, third_last, second_last, last)
            {
                self.remaining = before_four;
                return Some(self.handler.four_byte(four_byte));
            }
        }
        self.next_back_fallback()
    }
}

impl<H> FusedIterator for Utf8CharsWithHandler<'_, H> where H: Utf8Handler {}

// No manually-written tests for forward-iteration, because the code passed multiple
// days of fuzzing comparing with known-good behavior.

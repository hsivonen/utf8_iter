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
        if self.remaining.is_empty() {
            return None;
        }
        let first = self.remaining[0];
        if let Ok(ascii) = Ascii::try_new(first) {
            self.remaining = &self.remaining[1..];
            return Some(self.handler.single_byte(ascii));
        }
        if !multi_byte_lead(first) || self.remaining.len() == 1 {
            self.remaining = &self.remaining[1..];
            return Some(self.error());
        }
        let second = self.remaining[1];
        if !two_byte_prefix(first, second) {
            self.remaining = &self.remaining[1..];
            return Some(self.error());
        }
        if below_three_byte(first) {
            self.remaining = &self.remaining[2..];
            // SAFETY: We checked the invariant of
            // `self.handler.two_byte` with the combination of
            // `single_byte(first)`, `two_byte_prefix(first, second)`,
            // and `below_three_byte(first)`.
            return Some(unsafe {
                self.handler
                    .two_byte(TwoByteSequence::new_unchecked(first, second))
            });
        }
        if self.remaining.len() == 2 {
            self.remaining = &self.remaining[2..];
            return Some(self.error());
        }
        let third = self.remaining[2];
        if !unconstrained_continuation(third) {
            self.remaining = &self.remaining[2..];
            return Some(self.error());
        }
        if below_four_byte(first) {
            self.remaining = &self.remaining[3..];
            // SAFETY: We checked the invariant of
            // `self.handler.three_byte` with the combination of
            // `single_byte(first)`, `two_byte_prefix(first, second)`,
            // `below_three_byte(first)`, and `below_four_byte(first)`.
            return Some(unsafe {
                self.handler
                    .three_byte(ThreeByteSequence::new_unchecked(first, second, third))
            });
        }
        // At this point, we have a valid 3-byte prefix of a
        // four-byte sequence that has to be incomplete, because
        // otherwise `next()` would have succeeded.
        self.remaining = &self.remaining[3..];
        Some(self.error())
    }
}

impl<'a, H> Iterator for Utf8CharsWithHandler<'a, H>
where
    H: Utf8Handler,
{
    type Item = H::Output;

    #[inline]
    fn next(&mut self) -> Option<H::Output> {
        // This loop is only broken out of as goto forward
        #[allow(clippy::never_loop)]
        loop {
            if self.remaining.len() < 4 {
                break;
            }
            let first = self.remaining[0];
            if single_byte(first) {
                self.remaining = &self.remaining[1..];
                // SAFETY: We checked the invariant of
                // `self.handler.single_byte` above with
                // `single_byte(first)`.
                return Some(unsafe { self.handler.single_byte(Ascii::new_unchecked(first)) });
            }
            let second = self.remaining[1];
            if two_byte_lead(first) {
                if !unconstrained_continuation(second) {
                    break;
                }
                self.remaining = &self.remaining[2..];
                // SAFETY: We checked the invariant of
                // `self.handler.two_byte` with the combination of
                // `two_byte_lead(first)` and `unconstrained_continuation(second)`.
                return Some(unsafe {
                    self.handler
                        .two_byte(TwoByteSequence::new_unchecked(first, second))
                });
            }
            let third = self.remaining[2];
            if !three_byte_prefix(first, second, third) {
                break;
            }
            if below_four_byte(first) {
                self.remaining = &self.remaining[3..];
                // SAFETY: We checked the invariant of
                // `self.handler.three_byte` with the combination of
                // `single_byte(first)`, `two_byte_lead(first)`,
                // `three_byte_prefix(first, second, third)`, and `below_four_byte(first)`.
                return Some(unsafe {
                    self.handler
                        .three_byte(ThreeByteSequence::new_unchecked(first, second, third))
                });
            }
            let fourth = self.remaining[3];
            if !unconstrained_continuation(fourth) {
                break;
            }
            self.remaining = &self.remaining[4..];
            // SAFETY: We checked the invariant of
            // `self.handler.four_byte` with the combination of
            // `single_byte(first)`, `two_byte_lead(first)`,
            // `three_byte_prefix(first, second, third)`, `below_four_byte(first)`,
            // and `unconstrained_continuation(fourth)`.
            return Some(unsafe {
                self.handler.four_byte(FourByteSequence::new_unchecked(
                    first, second, third, fourth,
                ))
            });
        }
        self.next_fallback()
    }
}

impl<'a, H> DoubleEndedIterator for Utf8CharsWithHandler<'a, H>
where
    H: Utf8Handler + Clone,
{
    #[inline]
    fn next_back(&mut self) -> Option<H::Output> {
        if self.remaining.is_empty() {
            return None;
        }
        for (attempt, b) in (1..).zip(self.remaining.iter().rev()) {
            if !unconstrained_continuation(*b) {
                let (head, tail) = self.remaining.split_at(self.remaining.len() - attempt);
                let mut inner = Utf8CharsWithHandler {
                    remaining: tail,
                    handler: self.handler.clone(),
                };
                let candidate = inner.next();
                if inner.as_slice().is_empty() {
                    self.remaining = head;
                    return candidate;
                }
                break;
            }
            if attempt == 4 {
                break;
            }
        }

        self.remaining = &self.remaining[..self.remaining.len() - 1];
        Some(self.error())
    }
}

impl<H> FusedIterator for Utf8CharsWithHandler<'_, H> where H: Utf8Handler {}

// No manually-written tests for forward-iteration, because the code passed multiple
// days of fuzzing comparing with known-good behavior.

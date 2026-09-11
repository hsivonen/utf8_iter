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

/// Mapping from the four kinds of byte sequences or error
/// to output.
pub trait Utf8Handler {
    /// The per-scalar-value output type. (In the common case,
    /// this is `char`.)
    type Output;

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
    unsafe fn single_byte(&self, ascii: u8) -> Self::Output;

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
    unsafe fn two_byte(&self, first: u8, second: u8) -> Self::Output;

    /// Map a three-byte UTF-8 sequence to `Output`.
    ///
    /// When `Output` is `char`,
    /// `unsafe { three_bytes_to_char(first, second, third) }`
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
    unsafe fn three_byte(&self, first: u8, second: u8, third: u8) -> Self::Output;

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
    unsafe fn four_byte(&self, first: u8, second: u8, third: u8, fourth: u8) -> Self::Output;

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
    fn error(&self) -> Self::Output;
}

/// Iterator by `char` over `&[u8]` that contains
/// potentially-invalid UTF-8. See the crate documentation.
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
    /// Creates the iterator from a byte slice.
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
        if single_byte(first) {
            self.remaining = &self.remaining[1..];
            // SAFETY: We checked the invariant of
            // `self.handler.single_byte` above with
            // `single_byte(first)`.
            return Some(unsafe { self.handler.single_byte(first) });
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
            return Some(unsafe { self.handler.two_byte(first, second) });
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
            return Some(unsafe { self.handler.three_byte(first, second, third) });
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
        // Not delegating directly to `ErrorReportingUtf8CharsWithHandler` to avoid
        // an extra branch in the common case based on a cursory inspection
        // of generated code in a similar case. Be sure to inspect the
        // generated code as inlined into an actual usage site carefully
        // if attempting to consolidate the source code here.

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
                return Some(unsafe { self.handler.single_byte(first) });
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
                return Some(unsafe { self.handler.two_byte(first, second) });
            }
            let third = self.remaining[2];
            if !three_byte_prefix(first, second, third) {
                break;
            }
            if below_four_byte(first) {
                // SAFETY: We checked the invariant of
                // `self.handler.three_byte` with the combination of
                // `single_byte(first)`, `two_byte_lead(first)`,
                // `three_byte_prefix(first, second, third)`, and `below_four_byte(first)`.
                return Some(unsafe { self.handler.three_byte(first, second, third) });
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
            return Some(unsafe { self.handler.four_byte(first, second, third, fourth) });
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
        let mut attempt = 1;
        for b in self.remaining.iter().rev() {
            if b & 0xC0 != 0x80 {
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
            attempt += 1;
        }

        self.remaining = &self.remaining[..self.remaining.len() - 1];
        Some(self.error())
    }
}

impl<H> FusedIterator for Utf8CharsWithHandler<'_, H> where H: Utf8Handler {}

// No manually-written tests for forward-iteration, because the code passed multiple
// days of fuzzing comparing with known-good behavior.

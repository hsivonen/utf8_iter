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

use crate::helpers::below_four_byte;
use crate::helpers::below_three_byte;
use crate::helpers::single_byte;
use crate::helpers::unconstrained_continuation;
use crate::Utf8Handler;

/// Iterator over guaranteed-well-formed UTF-8 with `Utf8Handler`.
/// The `Utf8Handler::error()` is, of course, never called.
#[derive(Debug)]
pub struct CharsWithHandler<'a, H>
where
    H: Utf8Handler,
{
    /// # Safety-usable invariant
    ///
    /// When entering a `next` or `next_back` call, this slice
    /// is well-formed UTF-8, and when those methods return, this
    /// is again well-formed UTF-8.
    inner: core::slice::Iter<'a, u8>,
    handler: H,
}

impl<'a, H> CharsWithHandler<'a, H>
where
    H: Utf8Handler,
{
    #[inline(always)]
    /// Creates the iterator from a `&str` and handler.
    pub fn new(s: &'a str, handler: H) -> Self {
        CharsWithHandler::<'a, H> {
            inner: s.as_bytes().iter(),
            handler,
        }
    }

    /// Views the current remaining data in the iterator as a subslice
    /// of the original slice.
    #[inline(always)]
    pub fn as_str(&self) -> &'a str {
        // SAFETY: OK per the safety-usable invariant of `self.inner`.
        unsafe { core::str::from_utf8_unchecked(self.inner.as_slice()) }
    }

    /// Obtains a reference to the handler.
    #[inline(always)]
    pub fn handler(&self) -> &H {
        &self.handler
    }
}

impl<'a, H> Clone for CharsWithHandler<'a, H>
where
    H: Utf8Handler + Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            handler: self.handler.clone(),
        }
    }
}

impl<'a, H> Iterator for CharsWithHandler<'a, H>
where
    H: Utf8Handler,
{
    type Item = H::Output;

    #[inline]
    fn next(&mut self) -> Option<H::Output> {
        let first = *self.inner.next()?;
        if single_byte(first) {
            // SAFETY: We checked the invariant of
            // `self.handler.single_byte` above with
            // `single_byte(first)`.
            //
            // INVARIANT UPHELD for `self.inner`:
            // We consumed a complete UTF-8 sequence.
            return Some(unsafe { self.handler.single_byte(first) });
        }
        // SAFETY: Since we don't have a single-byte sequence (per above)
        // and we know `self.inner` represented well-formed UTF-8 upon
        // entry into this method, we know that the second byte exists.
        let second = *unsafe { self.inner.next().unwrap_unchecked() };
        if below_three_byte(first) {
            // SAFETY: We checked the invariant of
            // `self.handler.two_byte` with the combination of
            // `single_byte(first)` and `below_three_byte(first)` given
            // that `self.inner` represented well-formed UTF-8 upon
            // entry into this method.
            //
            // INVARIANT UPHELD for `self.inner`:
            // We consumed a complete UTF-8 sequence.
            return Some(unsafe { self.handler.two_byte(first, second) });
        }
        // SAFETY: Since we don't have a single-byte sequence or a two-byte
        // sequence (per above) and we know `self.inner` represented
        // well-formed UTF-8 upon entry into this method, we know that
        // the third byte exists.
        let third = *unsafe { self.inner.next().unwrap_unchecked() };
        if below_four_byte(first) {
            // SAFETY: We checked the invariant of
            // `self.handler.two_byte` with the combination of
            // `single_byte(first)`, `below_three_byte(first)`, and
            // `below_four_byte(first)` given that `self.inner`
            // represented well-formed UTF-8 upon entry into this method.
            //
            // INVARIANT UPHELD for `self.inner`:
            // We consumed a complete UTF-8 sequence.
            return Some(unsafe { self.handler.three_byte(first, second, third) });
        }
        // SAFETY: Since we don't have a single-byte sequence, a two-byte
        // sequence, or a three-byte sequence (per above) and we know
        // `self.inner` represented well-formed UTF-8 upon entry into this
        // method, we know that the fourth byte exists.
        let fourth = *unsafe { self.inner.next().unwrap_unchecked() };
        // SAFETY: We checked the invariant of
        // `self.handler.four_byte` given that `self.inner` represented
        // well-formed UTF-8 upon entry into this method by having
        // ruled out the three other sequence types.
        //
        // INVARIANT UPHELD for `self.inner`:
        // We consumed a complete UTF-8 sequence.
        Some(unsafe { self.handler.four_byte(first, second, third, fourth) })
    }
}

impl<'a, H> DoubleEndedIterator for CharsWithHandler<'a, H>
where
    H: Utf8Handler,
{
    #[inline]
    fn next_back(&mut self) -> Option<H::Output> {
        let last = *self.inner.next_back()?;
        if single_byte(last) {
            // SAFETY: We checked the invariant of
            // `self.handler.single_byte` above with
            // `single_byte(last)`.
            //
            // INVARIANT UPHELD for `self.inner`:
            // We consumed a complete UTF-8 sequence.
            return Some(unsafe { self.handler.single_byte(last) });
        }
        debug_assert!(unconstrained_continuation(last));
        // SAFETY: Since the last byte was not a single-byte sequence
        // given that `self.inner` represented well-formed UTF-8 upon
        // entry into this method, an earlier byte must exist.
        let second_last = *unsafe { self.inner.next_back().unwrap_unchecked() };
        if !unconstrained_continuation(second_last) {
            // SAFETY: Given that `self.inner` represented well-formed
            // UTF-8 upon entry into this method and that `last` is a continuation
            // but `second_last` is not, they must form a two-byte UTF-8
            // sequence.
            //
            // INVARIANT UPHELD for `self.inner`:
            // We consumed a complete UTF-8 sequence.
            return Some(unsafe { self.handler.two_byte(second_last, last) });
        }
        // SAFETY: Given that `self.inner` represented well-formed
        // UTF-8 upon entry into this method and we've ruled out a
        // single-byte sequence and a two-byte sequence, an earlier
        // byte must exist.
        let third_last = *unsafe { self.inner.next_back().unwrap_unchecked() };
        if !unconstrained_continuation(third_last) {
            // SAFETY: Given that `self.inner` represented well-formed
            // UTF-8 upon entry into this method and that `last` and `second_last`
            // are continuations but `third_last` is not, they must form a three-byte UTF-8
            // sequence.
            //
            // INVARIANT UPHELD for `self.inner`:
            // We consumed a complete UTF-8 sequence.
            return Some(unsafe { self.handler.three_byte(third_last, second_last, last) });
        }
        // SAFETY: Given that `self.inner` represented well-formed
        // UTF-8 upon entry into this method and we've ruled out a
        // single-byte sequence, a two-byte sequence, and a three-byte
        // sequences, an earlier byte must exist.
        let fourth_last = *unsafe { self.inner.next_back().unwrap_unchecked() };
        // SAFETY: Given that `self.inner` represented well-formed
        // UTF-8 upon entry into this method and we've ruled out a
        // single-byte sequence, a two-byte sequence, and a three-byte
        // sequences, this must be a four-byte sequence.
        //
        // INVARIANT UPHELD for `self.inner`:
        // We consumed a complete UTF-8 sequence.
        Some(unsafe {
            self.handler
                .four_byte(fourth_last, third_last, second_last, last)
        })
    }
}

impl<H> core::iter::FusedIterator for CharsWithHandler<'_, H> where H: Utf8Handler {}

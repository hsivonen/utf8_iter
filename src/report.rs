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

use core::fmt::Formatter;
use core::iter::FusedIterator;

use crate::helpers::four_bytes_to_char;
use crate::helpers::three_bytes_to_char;
use crate::helpers::two_bytes_to_char;
use crate::Utf8CharsWithHandler;
use crate::Utf8Handler;

/// A type for signaling UTF-8 errors.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub struct Utf8CharsError;

impl core::fmt::Display for Utf8CharsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), core::fmt::Error> {
        write!(f, "byte sequence not well-formed UTF-8")
    }
}

impl core::error::Error for Utf8CharsError {}

/// The `Output = Result<char, Utf8CharsError>` case.
#[derive(Debug, Clone)]
pub(crate) struct ErrorReportingHandler;

impl ErrorReportingHandler {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl Utf8Handler for ErrorReportingHandler {
    type Output = Result<char, Utf8CharsError>;

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
        Ok(char::from(ascii))
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
        Ok(two_bytes_to_char(first, second))
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
        Ok(unsafe { three_bytes_to_char(first, second, third) })
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
        Ok(unsafe { four_bytes_to_char(first, second, third, fourth) })
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
        Err(Utf8CharsError)
    }
}

/// Iterator by `Result<char,Utf8CharsError>` over `&[u8]` that contains
/// potentially-invalid UTF-8. There is exactly one `Utf8CharsError` per
/// each error as defined by the WHATWG Encoding Standard.
///
/// ```
/// let s = b"a\xFFb\xFF\x80c\xF0\x9F\xA4\xA6\xF0\x9F\xA4\xF0\x9F\xF0d";
/// let plain = utf8_iter::Utf8Chars::new(s);
/// let reporting = utf8_iter::ErrorReportingUtf8Chars::new(s);
/// assert!(plain.eq(reporting.map(|r| r.unwrap_or('\u{FFFD}'))));
/// ```
#[derive(Debug, Clone)]
pub struct ErrorReportingUtf8Chars<'a> {
    inner: Utf8CharsWithHandler<'a, ErrorReportingHandler>,
}

impl<'a> ErrorReportingUtf8Chars<'a> {
    #[inline(always)]
    /// Creates the iterator from a byte slice.
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            inner: Utf8CharsWithHandler::new(bytes, ErrorReportingHandler::new()),
        }
    }

    /// Views the current remaining data in the iterator as a subslice
    /// of the original slice.
    #[inline(always)]
    pub fn as_slice(&self) -> &'a [u8] {
        self.inner.as_slice()
    }
}

impl<'a> Iterator for ErrorReportingUtf8Chars<'a> {
    type Item = Result<char, Utf8CharsError>;

    #[inline]
    fn next(&mut self) -> Option<Result<char, Utf8CharsError>> {
        self.inner.next()
    }
}

impl<'a> DoubleEndedIterator for ErrorReportingUtf8Chars<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<Result<char, Utf8CharsError>> {
        self.inner.next_back()
    }
}

impl FusedIterator for ErrorReportingUtf8Chars<'_> {}

#[cfg(test)]
mod tests {
    use crate::ErrorReportingUtf8Chars;

    // Should be a static assert, but not taking a dependency for this.
    #[test]
    fn test_size() {
        assert_eq!(
            core::mem::size_of::<Option<<ErrorReportingUtf8Chars<'_> as Iterator>::Item>>(),
            core::mem::size_of::<Option<char>>()
        );
    }

    #[test]
    fn test_eq() {
        let a: <ErrorReportingUtf8Chars<'_> as Iterator>::Item = Ok('a');
        let a_again: <ErrorReportingUtf8Chars<'_> as Iterator>::Item = Ok('a');
        assert_eq!(a, a_again);
    }
}

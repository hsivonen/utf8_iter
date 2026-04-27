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

use crate::{GenericUtf8Iter, Utf8Action};
use core::fmt::Formatter;
use core::iter::FusedIterator;

/// A type for signaling UTF-8 errors.
///
/// Note: `core::error::Error` is not implemented due to implementing it
/// being an [unstable feature][1] at the time of writing.
///
/// [1]: https://github.com/rust-lang/rust/issues/103765
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub struct Utf8CharsError;

impl core::fmt::Display for Utf8CharsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), core::fmt::Error> {
        write!(f, "byte sequence not well-formed UTF-8")
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
    inner: GenericUtf8Iter<'a, ErrorReportingUtf8CharsAction>,
}

#[derive(Debug, Clone, Copy)]
struct ErrorReportingUtf8CharsAction;

impl Utf8Action for ErrorReportingUtf8CharsAction {
    type Output = Result<char, Utf8CharsError>;

    #[inline(always)]
    unsafe fn handle_ascii(&mut self, ascii: u8) -> Self::Output {
        debug_assert!(ascii < 0x80);
        Ok(char::from(ascii))
    }

    #[inline(always)]
    unsafe fn handle_2_byte(&mut self, b1: u8, b2: u8) -> Self::Output {
        // SAFETY: The `Utf8Action` trait contract guarantees that `b1` and `b2`
        // form a valid 2-byte UTF-8 sequence, which means the resulting scalar
        // value is a valid code point in the range `U+0080..=U+07FF`.
        let point = ((u32::from(b1) & 0x1F) << 6) | (u32::from(b2) & 0x3F);
        debug_assert!(core::char::from_u32(point).is_some());
        Ok(unsafe { char::from_u32_unchecked(point) })
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
        Ok(unsafe { char::from_u32_unchecked(point) })
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
        Ok(unsafe { char::from_u32_unchecked(point) })
    }

    #[inline(always)]
    fn handle_error(&mut self) -> Self::Output {
        Err(Utf8CharsError)
    }
}

impl<'a> ErrorReportingUtf8Chars<'a> {
    #[inline(always)]
    /// Creates the iterator from a byte slice.
    pub fn new(bytes: &'a [u8]) -> Self {
        ErrorReportingUtf8Chars::<'a> {
            inner: GenericUtf8Iter::new(bytes, ErrorReportingUtf8CharsAction),
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

    #[inline(always)]
    fn next(&mut self) -> Option<Result<char, Utf8CharsError>> {
        self.inner.next()
    }
}

impl<'a> DoubleEndedIterator for ErrorReportingUtf8Chars<'a> {
    #[inline(always)]
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

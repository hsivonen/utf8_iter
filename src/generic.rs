use crate::{in_inclusive_range8, UTF8_DATA};

/// A trait that defines the behavior for handling different UTF-8 sequence lengths
/// for use in [`GenericUtf8Iter`].
///
/// Implementors of this trait can be used with `GenericUtf8Iter` to decode UTF-8 bytes
/// and produce different outputs (e.g., `char`, `Result<char, Error>`, `(char, TrieValue)`).
///
/// # Safety
///
/// While this trait is not marked `unsafe`, its methods are called by `GenericUtf8Iter`
/// with the guarantee that the input bytes or decoded scalar values are valid UTF-8.
/// Implementors may choose to rely on this guarantee for performance (e.g., by using
/// `core::char::from_u32_unchecked`), but doing so requires an `unsafe` block within the
/// implementation.
pub trait Utf8Action {
    /// The type of item yielded by the iterator.
    type Output;

    /// Called when a valid ASCII byte (0-127) is encountered.
    ///
    /// # Safety
    ///
    /// The byte is guaranteed to be in the range `00..=7F`.
    unsafe fn handle_ascii(&mut self, ascii: u8) -> Self::Output;

    /// Called when a valid 2-byte UTF-8 sequence is encountered.
    ///
    /// Precise byte range: `C2..=DF` followed by `80..=BF`.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `b1` and `b2` form a valid 2-byte UTF-8 sequence.
    unsafe fn handle_2_byte(&mut self, b1: u8, b2: u8) -> Self::Output;

    /// Called when a valid 3-byte UTF-8 sequence is encountered.
    ///
    /// Precise byte range:
    /// - `E0` followed by `A0..=BF`, `80..=BF`
    /// - `E1..=EC` followed by `80..=BF`, `80..=BF`
    /// - `ED` followed by `80..=9F`, `80..=BF`
    /// - `EE..=EF` followed by `80..=BF`, `80..=BF`
    ///
    /// # Safety
    ///
    /// The caller must ensure that `b1`, `b2`, and `b3` form a valid 3-byte UTF-8 sequence.
    unsafe fn handle_3_byte(&mut self, b1: u8, b2: u8, b3: u8) -> Self::Output;

    /// Called when a valid 4-byte UTF-8 sequence is encountered.
    ///
    /// Precise byte range:
    /// - `F0` followed by `90..=BF`, `80..=BF`, `80..=BF`
    /// - `F1..=F3` followed by `80..=BF`, `80..=BF`, `80..=BF`
    /// - `F4` followed by `80..=8F`, `80..=BF`, `80..=BF`
    ///
    /// # Safety
    ///
    /// The caller must ensure that `b1`, `b2`, `b3`, and `b4` form a valid 4-byte UTF-8 sequence.
    unsafe fn handle_4_byte(&mut self, b1: u8, b2: u8, b3: u8, b4: u8) -> Self::Output;

    /// Called when an invalid sequence (error) is encountered.
    ///
    /// The `GenericUtf8Iter` automatically steps over the invalid bytes before calling this.
    fn handle_error(&mut self) -> Self::Output;

    /// Called when an invalid sequence (error) is encountered while iterating backwards.
    fn handle_error_reverse(&mut self) -> Self::Output {
        self.handle_error()
    }
}

/// A generic UTF-8 iterator that delegates to a `Utf8Action`.
#[derive(Debug, Clone)]
pub struct GenericUtf8Iter<'a, A> {
    pub(crate) remaining: &'a [u8],
    pub(crate) action: A,
}

impl<'a, A: Utf8Action> GenericUtf8Iter<'a, A> {
    #[inline(always)]
    pub fn new(remaining: &'a [u8], action: A) -> Self {
        Self { remaining, action }
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &'a [u8] {
        self.remaining
    }

    #[inline(never)]
    fn next_fallback(&mut self) -> Option<A::Output> {
        if self.remaining.is_empty() {
            return None;
        }
        let first = self.remaining[0];
        if first < 0x80 {
            self.remaining = &self.remaining[1..];
            // SAFETY: `first` was just checked to be `< 0x80`, which is the range for ASCII.
            return Some(unsafe { self.action.handle_ascii(first) });
        }
        if !in_inclusive_range8(first, 0xC2, 0xF4) || self.remaining.len() == 1 {
            self.remaining = &self.remaining[1..];
            return Some(self.action.handle_error());
        }
        let second = self.remaining[1];
        let (lower_bound, upper_bound) = match first {
            0xE0 => (0xA0, 0xBF),
            0xED => (0x80, 0x9F),
            0xF0 => (0x90, 0xBF),
            0xF4 => (0x80, 0x8F),
            _ => (0x80, 0xBF),
        };
        if !in_inclusive_range8(second, lower_bound, upper_bound) {
            self.remaining = &self.remaining[1..];
            return Some(self.action.handle_error());
        }
        if first < 0xE0 {
            self.remaining = &self.remaining[2..];
            // SAFETY: `first` is in `C2..=DF` (due to the `in_inclusive_range8(first, 0xC2, 0xF4)`
            // check above and the `first < 0xE0` check here) and `second` is in `80..=BF`
            // (due to the `match first` and `in_inclusive_range8(second, lower_bound, upper_bound)`
            // checks above), which together form a valid 2-byte UTF-8 sequence.
            return Some(unsafe { self.action.handle_2_byte(first, second) });
        }
        if self.remaining.len() == 2 {
            self.remaining = &self.remaining[2..];
            return Some(self.action.handle_error());
        }
        let third = self.remaining[2];
        if !in_inclusive_range8(third, 0x80, 0xBF) {
            self.remaining = &self.remaining[2..];
            return Some(self.action.handle_error());
        }
        if first < 0xF0 {
            self.remaining = &self.remaining[3..];
            // SAFETY: `first` is in `E0..=EF` (due to the `first < 0xE0` check failure and the
            // `first < 0xF0` check here), `second` has been validated against `lower_bound`
            // and `upper_bound` for this lead byte, and `third` is in `80..=BF`. Together
            // these form a valid 3-byte UTF-8 sequence.
            return Some(unsafe { self.action.handle_3_byte(first, second, third) });
        }
        // At this point, we have a valid 3-byte prefix of a
        // four-byte sequence that has to be incomplete.
        self.remaining = &self.remaining[3..];
        Some(self.action.handle_error())
    }
}

impl<'a, A: Utf8Action> Iterator for GenericUtf8Iter<'a, A> {
    type Item = A::Output;

    #[inline]
    fn next(&mut self) -> Option<A::Output> {
        #[allow(clippy::never_loop)]
        loop {
            if self.remaining.len() < 4 {
                break;
            }
            let first = self.remaining[0];
            if first < 0x80 {
                self.remaining = &self.remaining[1..];
                // SAFETY: `first` was just checked to be `< 0x80`, which is the range for ASCII.
                return Some(unsafe { self.action.handle_ascii(first) });
            }
            let second = self.remaining[1];
            if in_inclusive_range8(first, 0xC2, 0xDF) {
                if !in_inclusive_range8(second, 0x80, 0xBF) {
                    break;
                }
                self.remaining = &self.remaining[2..];
                // SAFETY: `first` is in `C2..=DF` and `second` is in `80..=BF`
                // (checked immediately above), which together form a valid 2-byte UTF-8 sequence.
                return Some(unsafe { self.action.handle_2_byte(first, second) });
            }
            let third = self.remaining[2];
            if first < 0xF0 {
                if ((UTF8_DATA.table[usize::from(second)]
                    & UTF8_DATA.table[usize::from(first) + 0x80])
                    | (third >> 6))
                    != 2
                {
                    break;
                }
                self.remaining = &self.remaining[3..];
                // SAFETY: `first` is in `E0..=EF` (due to the check failure for `first < 0x80`
                // and `in_inclusive_range8(first, 0xC2, 0xDF)`, and the `first < 0xF0` check here).
                // The table-based check against `UTF8_DATA.table` validates that `first` and
                // `second` form a valid prefix for a 3-byte sequence (including overlong
                // and surrogate checks), and `(third >> 6) == 2` validates that `third`
                // is a continuation byte (`80..=BF`).
                return Some(unsafe { self.action.handle_3_byte(first, second, third) });
            }
            let fourth = self.remaining[3];
            if (u16::from(
                UTF8_DATA.table[usize::from(second)] & UTF8_DATA.table[usize::from(first) + 0x80],
            ) | u16::from(third >> 6)
                | (u16::from(fourth & 0xC0) << 2))
                != 0x202
            {
                break;
            }
            self.remaining = &self.remaining[4..];
            // SAFETY: `first` is in `F0..=F4` (due to the `first < 0xF0` check failure and the
            // table-based check's implicit lead byte range). The table-based check validates
            // that `first` and `second` form a valid prefix for a 4-byte sequence (including
            // overlong and out-of-range checks), `(third >> 6) == 2` (implied by the `0x202`
            // mask/check) validates that `third` is a continuation byte, and `(fourth & 0xC0) == 0x80`
            // (also implied by the `0x202` mask/check) validates that `fourth` is a continuation byte.
            return Some(unsafe { self.action.handle_4_byte(first, second, third, fourth) });
        }
        self.next_fallback()
    }
}

// In order for `DoubleEndedIterator` to be implemented via `GenericUtf8Iter`'s 
// `next_back`, `A` must implement `Clone` because the back iteration relies 
// on repeatedly instantiating a temporary inner iterator and consuming it.
impl<'a, A: Utf8Action + Clone> DoubleEndedIterator for GenericUtf8Iter<'a, A> {
    #[inline]
    fn next_back(&mut self) -> Option<A::Output> {
        if self.remaining.is_empty() {
            return None;
        }
        let mut attempt = 1;
        for b in self.remaining.iter().rev() {
            if b & 0xC0 != 0x80 {
                let (head, tail) = self.remaining.split_at(self.remaining.len() - attempt);
                // Important: we pass a clone of the action.
                let mut inner = GenericUtf8Iter::new(tail, self.action.clone());
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
        Some(self.action.handle_error_reverse())
    }
}

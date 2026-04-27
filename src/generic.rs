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
    /// The caller must ensure `ascii` is in the range `00..=7F`.
    unsafe fn handle_ascii(&mut self, ascii: u8) -> Self::Output;

    /// Called when a valid 2-byte UTF-8 sequence is encountered.
    ///
    /// Precise byte range: `C2..=DF` followed by `80..=BF`.
    ///
    /// # Safety
    ///
    /// The caller must ensure `point` is a valid 2-byte UTF-8 sequence
    /// (scalar values in the range `U+0080..=U+07FF`).
    unsafe fn handle_2_byte(&mut self, point: u32) -> Self::Output;

    /// Called when a valid 3-byte UTF-8 sequence is encountered.
    ///
    /// Precise byte range:
    /// - `E0` followed by `A0..=BF`, `80..=BF`
    /// - `E1..=EC` followed by `80..=BF`, `80..=BF`
    /// - `ED` followed by `80..=9F`, `80..=BF`
    /// - `EE..=EF` followed by `80..=BF`, `80..=BF`
    ///
    /// A different way of looking at this is `E0 A0 BF` to `EF BF BF`
    /// with continuation bytes always staying in the range `80..=BF`,
    /// removing the surrogates in `ED A0 80` to `ED BF BF`.
    ///
    /// # Safety
    ///
    /// The caller must ensure `point` is a valid 3-byte UTF-8 sequence
    /// (scalar values in the range `U+0800..=U+FFFF`,
    /// excluding surrogates `U+D800..=U+DFFF`).
    unsafe fn handle_3_byte(&mut self, point: u32) -> Self::Output;

    /// Called when a valid 4-byte UTF-8 sequence is encountered.
    ///
    /// Precise byte range:
    /// - `F0` followed by `90..=BF`, `80..=BF`, `80..=BF`
    /// - `F1..=F3` followed by `80..=BF`, `80..=BF`, `80..=BF`
    /// - `F4` followed by `80..=8F`, `80..=BF`, `80..=BF`
    ///
    /// A different way of looking at this is `F0 90 80 80` to `F4 8F BF BF`
    /// with continuation bytes always staying in the range `80..=BF`.
    ///
    /// # Safety
    ///
    /// The caller must ensure `point` is a valid 2-byte UTF-8 sequence
    /// (scalar values in the range `U+0080..=U+07FF`).
    unsafe fn handle_4_byte(&mut self, point: u32) -> Self::Output;

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
            // SAFETY: `first` was just checked to be `< 0x80`.
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
            let point = ((u32::from(first) & 0x1F) << 6) | (u32::from(second) & 0x3F);
            // SAFETY: `first` and `second` have been validated to form a correct 2-byte sequence.
            return Some(unsafe { self.action.handle_2_byte(point) });
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
            let point = ((u32::from(first) & 0xF) << 12)
                | ((u32::from(second) & 0x3F) << 6)
                | (u32::from(third) & 0x3F);
            // SAFETY: `first`, `second`, and `third` have been validated to form a correct 3-byte sequence.
            return Some(unsafe { self.action.handle_3_byte(point) });
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
                // SAFETY: `first` was just checked to be `< 0x80`.
                return Some(unsafe { self.action.handle_ascii(first) });
            }
            let second = self.remaining[1];
            if in_inclusive_range8(first, 0xC2, 0xDF) {
                if !in_inclusive_range8(second, 0x80, 0xBF) {
                    break;
                }
                let point = ((u32::from(first) & 0x1F) << 6) | (u32::from(second) & 0x3F);
                self.remaining = &self.remaining[2..];
                // SAFETY: `first` and `second` have been validated to form a correct 2-byte sequence.
                return Some(unsafe { self.action.handle_2_byte(point) });
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
                let point = ((u32::from(first) & 0xF) << 12)
                    | ((u32::from(second) & 0x3F) << 6)
                    | (u32::from(third) & 0x3F);
                self.remaining = &self.remaining[3..];
                // SAFETY: `first`, `second`, and `third` have been validated to form a correct 3-byte sequence.
                return Some(unsafe { self.action.handle_3_byte(point) });
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
            let point = ((u32::from(first) & 0x7) << 18)
                | ((u32::from(second) & 0x3F) << 12)
                | ((u32::from(third) & 0x3F) << 6)
                | (u32::from(fourth) & 0x3F);
            self.remaining = &self.remaining[4..];
            // SAFETY: `first`, `second`, `third`, and `fourth` have been validated to form a correct 4-byte sequence.
            return Some(unsafe { self.action.handle_4_byte(point) });
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

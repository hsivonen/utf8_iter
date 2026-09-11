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

//! Helper functions for building UTF-8 decoders and validators.

#[repr(align(64))] // Align to cache lines
struct Utf8Data {
    table: [u8; 384],
}

static UTF8_DATA: Utf8Data = Utf8Data {
    table: const {
        // The first 256 entries are for looking up `second`.
        // The rest as for looking up `first` on the assumption
        // that `first` is not ASCII.
        let mut table = [0u8; 384];
        // Lanes 0 and 1 are left unused to allow the tag
        // bits of the third byte in a three-byte sequence
        // to be shifted there for a single-branch comparison.

        // Trails
        let mut i = 0u16;
        while i < 0x100 {
            let second = i as u8;
            // If a `second` is valid given a lane, we leave a lane as zero.
            // We put 1 on the lane if the `second` is invalid given a lane.
            let mut combined = 1 << 2; // invalid lead
            if second < 0x80 || second > 0xBF {
                combined |= 1 << 3; // normal trail
            }
            if second < 0xA0 || second > 0xBF {
                combined |= 1 << 4; // three-byte special lower bound
            }
            if second < 0x80 || second > 0x9F {
                combined |= 1 << 5; // three-byte special upper bound
            }
            if second < 0x90 || second > 0xBF {
                combined |= 1 << 6; // four-byte special lower bound
            }
            if second < 0x80 || second > 0x8F {
                combined |= 1 << 7; // four-byte special upper bound
            }
            table[second as usize] = combined;
            i += 1;
        }

        // Leads
        i = 0x80; // We don't cover ASCII for leads.
        while i < 0x100 {
            let first = i as u8;
            let lane = match first {
                0xC2..=0xDF | 0xE1..=0xEC | 0xEE..=0xEF | 0xF1..=0xF3 => {
                    1 << 3 // normal trail
                }
                0xE0 => {
                    1 << 4 // three-byte special lower bound
                }
                0xED => {
                    1 << 5 // three-byte special upper bound
                }
                0xF0 => {
                    1 << 6 // four-byte special lower bound
                }
                0xF4 => {
                    1 << 7 // four-byte special upper bound
                }
                _ => {
                    // invalid lead
                    1 << 2
                }
            };
            table[0x80 + first as usize] = lane;
            i += 1;
        }

        table
    },
};

/// `true` iff `continuation` is valid as:
/// * The second byte of a two-byte sequence.
/// * The third byte of a three-byte sequence.
/// * The third byte of a four-byte sequence.
/// * The fourth byte of a four-byte sequence.
///
/// Or, alternatively, `true` iff `continuation` is a continuation
/// byte in general. That is, the second byte of a three-byte
/// sequence or a four-byte sequence satisfies this check, but
/// satisfying this check isn't sufficient for the byte to be
/// valid for those positions.
#[inline(always)]
pub fn unconstrained_continuation(continuation: u8) -> bool {
    in_inclusive_range8(continuation, 0x80, 0xBF)
}

/// `true` iff `first` is in the ASCII range.
#[inline(always)]
pub fn single_byte(first: u8) -> bool {
    first < 0x80
}

/// `true` iff `first` is a valid lead byte for a multi-byte
/// sequence.
#[inline(always)]
pub fn multi_byte_lead(first: u8) -> bool {
    in_inclusive_range8(first, 0xC2, 0xF4)
}

/// `true` iff `first` is a valid lead byte for a two-byte
/// sequence.
#[inline(always)]
pub fn two_byte_lead(first: u8) -> bool {
    in_inclusive_range8(first, 0xC2, 0xDF)
}

/// `true` iff `first` is a valid lead byte for a three-byte
/// sequence.
#[inline(always)]
pub fn three_byte_lead(first: u8) -> bool {
    in_inclusive_range8(first, 0xE0, 0xEF)
}

/// `true` iff `first` is a valid lead byte for a four-byte
/// sequence.
#[inline(always)]
pub fn four_byte_lead(first: u8) -> bool {
    in_inclusive_range8(first, 0xF0, 0xF4)
}

/// `true` iff `first` and `second` form a valid two-byte UTF-8
/// sequence.
#[inline(always)]
pub fn two_byte(first: u8, second: u8) -> bool {
    two_byte_lead(first) && unconstrained_continuation(second)
}

/// `true` iff `first`, `second`, and `third` form a valid three-byte UTF-8
/// sequence.
#[inline(always)]
pub fn three_byte(first: u8, second: u8, third: u8) -> bool {
    three_byte_lead(first) && three_byte_prefix(first, second, third)
}

/// `true` iff `first`, `second`, `third`, and `fourth` form a valid
/// four-byte UTF-8 sequence.
#[inline(always)]
pub fn four_byte(first: u8, second: u8, third: u8, fourth: u8) -> bool {
    !below_four_byte(first)
        && (u16::from(table_lookup(first, second))
            | u16::from(third >> 6)
            | (u16::from(fourth & 0xC0) << 2)
            == 0x202)
}

/// `true` iff `first` is less than the lowest lead byte for a three-byte sequence.
#[inline(always)]
pub fn below_three_byte(first: u8) -> bool {
    first < 0xE0
}

/// `true` iff `first` is less than the lowest lead byte for a four-byte sequence.
#[inline(always)]
pub fn below_four_byte(first: u8) -> bool {
    first < 0xF0
}

/// Assuming that `first` is not ASCII, `true` iff
/// `first` and `second` for a two-byte prefix of a valid multibyte
/// UTF-8 sequence.
///
/// # Panics
///
/// With debug assertions enabled panics if `first` is ASCII.
#[inline(always)]
pub fn two_byte_prefix(first: u8, second: u8) -> bool {
    table_lookup(first, second) == 0
}

/// Assuming that `first` is neither ASCII nor a two-byte lead, `true` iff
/// `first`, `second`, and `third` form either a valid three-byte UTF-8
/// sequence or a three-byte prefix of a four-byte UTF-8 sequence.
///
/// # Panics
///
/// With debug assertions enabled panics if `first` is either ASCII or a
/// two-byte lead.
#[inline(always)]
pub fn three_byte_prefix(first: u8, second: u8, third: u8) -> bool {
    debug_assert!(!two_byte_lead(first));
    table_lookup(first, second) | (third >> 6) == 2
}

/// The table lookup for backing prefix checks.
///
/// # Panics
///
/// With debug assertions enabled panics if `first` is ASCII.
#[inline(always)]
fn table_lookup(first: u8, second: u8) -> u8 {
    debug_assert!(!first.is_ascii());
    UTF8_DATA.table[usize::from(second)] & UTF8_DATA.table[usize::from(first) + 0x80]
}

/// Converts a valid two-byte UTF-8 sequence to a `char`.
///
/// # Panics
///
/// If the input does not actually represent a valid two-byte
/// UTF-8 sequence, this function panics if debug assertions
/// are enabled. If debug assertions are disabled, the output
/// is bogus but in the `char` range, which is why this function
/// isn't marked `unsafe`.
#[inline(always)]
pub fn two_bytes_to_char(first: u8, second: u8) -> char {
    debug_assert!(two_byte(first, second));
    // SAFETY: We take the low 5 bits of `first` and the low 6 bits of `second`,
    // which satisfies the precondition of `bits_to_char`.
    unsafe { bits_to_char(low_five(first), low_six(second)) }
}

/// Converts a valid three-byte UTF-8 sequence to a `char`.
///
/// # Safety
///
/// The three bytes must form a valid UTF-8 sequence.
///
/// # Panics
///
/// If the input does not actually represent a valid three-byte
/// UTF-8 sequence, this function panics if debug assertions
/// are enabled.
#[inline(always)]
pub unsafe fn three_bytes_to_char(first: u8, second: u8, third: u8) -> char {
    debug_assert!(three_byte(first, second, third));
    let scalar = (low_four(first) << 12) | (low_six(second) << 6) | low_six(third);
    debug_assert!(char::from_u32(scalar).is_some());
    // SAFETY: We distribute the bits in a way that formed
    // a valid scalar on the assumption that the input
    // satisfied the safety precondition.
    unsafe { char::from_u32_unchecked(scalar) }
}

/// Converts a valid four-byte UTF-8 sequence to a `char`.
///
/// # Safety
///
/// The four bytes must form a valid UTF-8 sequence.
///
/// # Panics
///
/// If the input does not actually represent a valid four-byte
/// UTF-8 sequence, this function panics if debug assertions
/// are enabled.
#[inline(always)]
pub unsafe fn four_bytes_to_char(first: u8, second: u8, third: u8, fourth: u8) -> char {
    debug_assert!(four_byte(first, second, third, fourth));
    let scalar = (low_three(first) << 18)
        | (low_six(second) << 12)
        | (low_six(third) << 6)
        | low_six(fourth);
    debug_assert!(char::from_u32(scalar).is_some());
    // SAFETY: We distribute the bits in a way that formed
    // a valid scalar on the assumption that the input
    // satisfied the safety precondition.
    unsafe { char::from_u32_unchecked(scalar) }
}

/// Returns the low six bits of the input.
///
/// # Panics
///
/// If debug assertions are enabled, panics if `continuation`
/// is not an UTF-8 continuation byte.
#[inline(always)]
pub fn low_six(continuation: u8) -> u32 {
    debug_assert!(unconstrained_continuation(continuation));
    u32::from(continuation & 0b111_111)
}

/// Returns the low five bits of the input.
///
/// # Panics
///
/// If debug assertions are enabled, panics if `first`
/// is not a lead byte for a two-byte UTF-8 sequence.
#[inline(always)]
pub fn low_five(first: u8) -> u32 {
    debug_assert!(two_byte_lead(first));
    u32::from(first & 0b11_111)
}

/// Returns the low four bits of the input.
///
/// # Panics
///
/// If debug assertions are enabled, panics if `first`
/// is not a lead byte for a three-byte UTF-8 sequence.
#[inline(always)]
pub fn low_four(first: u8) -> u32 {
    debug_assert!(three_byte_lead(first));
    u32::from(first & 0b1111)
}

/// Returns the low three bits of the input.
///
/// # Panics
///
/// If debug assertions are enabled, panics if `first`
/// is not a lead byte for a four-byte UTF-8 sequence.
#[inline(always)]
pub fn low_three(first: u8) -> u32 {
    debug_assert!(four_byte_lead(first));
    u32::from(first & 0b111)
}

/// Combines the non-tag bits from the first and second byte
/// of a three-byte UTF-8 sequence.
///
/// # Panics
///
/// If debug assertions are enabled, panics if `first`
/// is not a lead byte for a three-byte UTF-8 sequence
/// of if `second` isn't a valid continuation for that
/// lead byte.
#[inline(always)]
pub fn high_ten(first: u8, second: u8) -> u32 {
    debug_assert!(three_byte_lead(first));
    debug_assert!(two_byte_prefix(first, second));
    (low_four(first) << 6) | low_six(second)
}

/// Converts the non-tag bits of a two-byte or a three-byte UTF-8 sequence to
/// a `char`.
///
/// `high_ten` represents either:
///  * the 5 least-significant bits of the lead byte of a valid two-byte UTF-8 sequence
///  * the ten non-tag bits gathered from the first and second bytes of a valid three-byte
///    UTF-8 sequence.
///
/// `low_six` representes the 6 least-significant bytes of the last byte of
/// either a three-byte or two-byte UTF-8 sequence.
///
/// # Safety
///
/// The bits other than the 10 least-significant bits of `high_then` must
/// be zeros. The bits other than the 6 least-significant bits of `low_six`
/// must be zeros. `high_ten` must not a value that would cause the combination
/// of `high_ten` and `low_six` to form a surrogate code point.
///
/// # Panics
///
/// With debug assertions enabled panics if the safety invariant is violated.
#[inline(always)]
pub unsafe fn bits_to_char(high_ten: u32, low_six: u32) -> char {
    debug_assert_eq!(high_ten & !0b11111_11111, 0);
    debug_assert_eq!(low_six & !0b111_111, 0);
    let scalar = (high_ten << 6) | low_six;
    debug_assert!(char::from_u32(scalar).is_some());
    // SAFETY: We are relying on the caller conforming
    // to the documented safety invariant.
    unsafe { char::from_u32_unchecked(scalar) }
}

#[inline(always)]
fn in_inclusive_range8(i: u8, start: u8, end: u8) -> bool {
    i.wrapping_sub(start) <= (end - start)
}

#[cfg(test)]
mod tests {

    fn two_byte_prefix_reference(first: u8, second: u8) -> bool {
        if !super::multi_byte_lead(first) {
            return false;
        }
        let (lower_bound, upper_bound) = match first {
            0xE0 => (0xA0, 0xBF),
            0xED => (0x80, 0x9F),
            0xF0 => (0x90, 0xBF),
            0xF4 => (0x80, 0x8F),
            _ => (0x80, 0xBF),
        };
        super::in_inclusive_range8(second, lower_bound, upper_bound)
    }

    #[test]
    fn test_two_byte_prefix() {
        for first in 0x80..=0xFF {
            for second in 0..0xFF {
                assert_eq!(
                    super::two_byte_prefix(first, second),
                    two_byte_prefix_reference(first, second)
                );
            }
        }
    }
}

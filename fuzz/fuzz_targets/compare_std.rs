// Any copyright is dedicated to the Public Domain.
// https://creativecommons.org/publicdomain/zero/1.0/

#![no_main]
use libfuzzer_sys::fuzz_target;
use utf8_iter::Utf8Chars;

fuzz_target!(|data: &[u8]| {
    let from_std = String::from_utf8_lossy(data);
    assert!(Utf8Chars::new(data).eq(from_std.chars()));
    assert!(Utf8Chars::new(data).rev().eq(from_std.chars().rev()));
});

// Any copyright is dedicated to the Public Domain.
// https://creativecommons.org/publicdomain/zero/1.0/

#![no_main]
use libfuzzer_sys::fuzz_target;
use utf8_iter::Utf8Chars;

fuzz_target!(|data: &[u8]| {
    let from_iter: String = Utf8Chars::new(data).collect();
    let from_std = String::from_utf8_lossy(data);
    assert_eq!(from_iter, from_std);
});

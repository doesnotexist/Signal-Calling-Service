//
// Copyright 2021 Signal Messenger, LLC
// SPDX-License-Identifier: AGPL-3.0-only
//

//! Common functionality for ice, rtp, rtcp, dtls, or googcc.

mod bits;
mod bytes_reader;
mod collections;
mod counters;
mod data_rate;
mod integers;
mod math;
mod serialize;
mod time;

use std::{cmp::PartialEq, convert::TryInto, fmt::Write};

use anyhow::Result;
pub use bits::*;
pub use bytes_reader::*;
pub use collections::*;
pub use counters::*;
pub use data_rate::*;
use hex::FromHex;
pub use integers::*;
pub use math::*;
use rand::{thread_rng, Rng};
pub use serialize::*;
pub use time::*;

// It's (value, rest)
// TODO: Change to Result
pub type ReadOption<'a, T> = Option<(T, &'a [u8])>;

pub fn read_u16_len_prefixed_u16s(input: &[u8]) -> ReadOption<Vec<u16>> {
    let (bytes, rest) = read_u16_len_prefixed(input)?;
    let (values, _) = read_n(bytes, bytes.len() / 2, read_u16)?;
    Some((values, rest))
}

pub fn read_u24_len_prefixed(input: &[u8]) -> ReadOption<&[u8]> {
    let (len, rest) = read_u24(input)?;
    let (bytes, rest) = read_bytes(rest, len.into())?;
    Some((bytes, rest))
}

pub fn read_u16_len_prefixed(input: &[u8]) -> ReadOption<&[u8]> {
    let (len, rest) = read_u16(input)?;
    let (bytes, rest) = read_bytes(rest, len as usize)?;
    Some((bytes, rest))
}

pub fn read_u8_len_prefixed(input: &[u8]) -> ReadOption<&[u8]> {
    let (len, rest) = read_u8(input)?;
    let (bytes, rest) = read_bytes(rest, len as usize)?;
    Some((bytes, rest))
}

pub fn read_u48(input: &[u8]) -> ReadOption<U48> {
    let (bytes, rest) = read_bytes(input, 6)?;
    Some((parse_u48(bytes), rest))
}

pub fn read_u32(input: &[u8]) -> ReadOption<u32> {
    let (bytes, rest) = read_bytes(input, 4)?;
    Some((parse_u32(bytes), rest))
}

pub fn read_u24(input: &[u8]) -> ReadOption<U24> {
    let (bytes, rest) = read_bytes(input, 3)?;
    Some((parse_u24(bytes), rest))
}

pub fn read_u16(input: &[u8]) -> ReadOption<u16> {
    let (bytes, rest) = read_bytes(input, 2)?;
    Some((parse_u16(bytes), rest))
}

pub fn read_i16(input: &[u8]) -> ReadOption<i16> {
    let (bytes, rest) = read_bytes(input, 2)?;
    Some((parse_i16(bytes), rest))
}

pub fn read_u8(input: &[u8]) -> ReadOption<u8> {
    let (bytes, rest) = read_bytes(input, 1)?;
    Some((bytes[0], rest))
}

pub fn read_n<'a, T>(
    mut input: &'a [u8],
    n: usize,
    read_one: impl Fn(&'a [u8]) -> ReadOption<'a, T>,
) -> ReadOption<'a, Vec<T>> {
    let mut values = Vec::with_capacity(n);
    for _ in 0..n {
        let (val, rest) = read_one(input)?;
        values.push(val);
        input = rest;
    }
    Some((values, input))
}

pub fn read_as_many_as_possible<'a, T>(
    mut input: &'a [u8],
    read_one: impl Fn(&'a [u8]) -> ReadOption<'a, T>,
) -> ReadOption<'a, Vec<T>> {
    let mut values = vec![];
    while !input.is_empty() {
        let (val, rest) = read_one(input)?;
        values.push(val);
        input = rest;
    }
    Some((values, input))
}

// Returns (read, rest)
pub fn read_bytes(input: &[u8], len: usize) -> ReadOption<&[u8]> {
    let bytes = input.get(0..len)?;
    let rest = &input[len..];
    Some((bytes, rest))
}

pub fn read_from_end(input: &[u8], len: usize) -> ReadOption<&[u8]> {
    if input.len() < len {
        return None;
    }
    let (before, after) = read_bytes(input, input.len() - len)?;
    Some((after, before))
}

pub fn parse_u16(bytes: &[u8]) -> u16 {
    u16::from_be_bytes(bytes[0..2].try_into().unwrap())
}

pub fn parse_u16_le(bytes: &[u8]) -> u16 {
    u16::from_le_bytes(bytes[0..2].try_into().unwrap())
}

pub fn parse_i16(bytes: &[u8]) -> i16 {
    i16::from_be_bytes(bytes[0..2].try_into().unwrap())
}

pub fn parse_u24(bytes: &[u8]) -> U24 {
    U24::from_be_bytes(bytes[0..U24::SIZE].try_into().unwrap())
}

pub fn parse_u32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes(bytes[0..4].try_into().unwrap())
}

pub fn parse_u48(bytes: &[u8]) -> U48 {
    U48::from_be_bytes(bytes[0..U48::SIZE].try_into().unwrap())
}

#[cfg(test)]
mod parse_tests {
    use std::convert::TryFrom;

    use super::*;

    #[test]
    fn parse_48() {
        assert_eq!(
            U48::try_from(0x010203040506u64).unwrap(),
            parse_u48(vec![1, 2, 3, 4, 5, 6].as_slice())
        )
    }

    #[test]
    fn parse_24() {
        assert_eq!(
            U24::try_from(0x010203u32).unwrap(),
            parse_u24(vec![1, 2, 3].as_slice())
        )
    }
}

pub fn random_hex_string(n: usize) -> String {
    const HEXCHARSET: &[u8] = b"abcdef0123456789";

    let mut rng = thread_rng();
    let string: String = (0..n)
        .map(|_| {
            let index = rng.gen_range(0..HEXCHARSET.len());
            HEXCHARSET[index] as char
        })
        .collect();
    string
}

/// Const generic expressions may replace this in future, but for now we must have a macro
macro_rules! random_base64_string_of_length {
    ($string_length:expr) => {{
        base64::encode(thread_rng().gen::<[u8; $string_length * 6 / 8]>())
    }};
}

/// Create a random Base64 string of length 32.
/// ```
/// # use calling_server::common::random_base64_string_of_length_32;
///
/// let string = random_base64_string_of_length_32();
/// assert_eq!(32, string.len());
///
/// # let string2 = random_base64_string_of_length_32();
/// # assert_ne!(string, string2);
/// ```
pub fn random_base64_string_of_length_32() -> String {
    random_base64_string_of_length!(32)
}

/// Create a random Base64 string of length 4.
/// ```
/// # use calling_server::common::random_base64_string_of_length_4;
///
/// let string = random_base64_string_of_length_4();
/// assert_eq!(4, string.len());
///
/// # let string2 = random_base64_string_of_length_4();
/// # assert_ne!(string, string2);
/// ```
pub fn random_base64_string_of_length_4() -> String {
    random_base64_string_of_length!(4)
}

/// Encodes a slice of bytes as a hexadecimal fingerprint string.
///
/// ```
/// use calling_server::common::bytes_to_colon_separated_hexstring;
///
/// assert_eq!(bytes_to_colon_separated_hexstring(&[]), "");
/// assert_eq!(bytes_to_colon_separated_hexstring(&[0x01, 0xAB, 0xCD]), "01:AB:CD");
/// ```
pub fn bytes_to_colon_separated_hexstring(bytes: &[u8]) -> String {
    let mut result = String::with_capacity(bytes.len() * 3);
    for byte in bytes {
        write!(&mut result, "{:02X}:", byte).expect("Should be safe to write to String");
    }
    if !result.is_empty() {
        // Remove the extra colon.
        result.pop();
    }
    result
}

/// Decodes hexadecimal fingerprint string to a u8 array of 32 bytes.
///
/// ```
/// use calling_server::common::colon_separated_hexstring_to_array;
///
/// assert_eq!(colon_separated_hexstring_to_array(
///     "00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00").unwrap(),
///     [0u8; 32]);
/// assert_eq!(colon_separated_hexstring_to_array(
///     "00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:").unwrap(),
///     [0u8; 32]);
/// assert!(colon_separated_hexstring_to_array("").is_err());
/// assert!(colon_separated_hexstring_to_array(":").is_err());
/// assert!(colon_separated_hexstring_to_array("01").is_err());
/// assert!(colon_separated_hexstring_to_array("01:AB:CD").is_err());
/// assert!(colon_separated_hexstring_to_array("01:AB:CD:").is_err());
/// assert!(colon_separated_hexstring_to_array("1:A:B").is_err());
/// assert!(colon_separated_hexstring_to_array("01:AB:B").is_err());
/// assert!(colon_separated_hexstring_to_array("01:AB:B:").is_err());
/// ```
pub fn colon_separated_hexstring_to_array(string: &str) -> Result<[u8; 32]> {
    let string = string.replace(":", "");
    let result = <[u8; 32]>::from_hex(string)?;
    Ok(result)
}

/// Allows using `?` syntax in a scope and collecting failures in a `Result`.
pub fn try_scoped<T>(call: impl FnOnce() -> anyhow::Result<T>) -> anyhow::Result<T> {
    call()
}

// Can be used for video resolution
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct PixelSize {
    pub width: u16,
    pub height: u16,
}

/// Number of pixels
#[derive(Clone, Debug, Eq, PartialEq, Copy, Ord, PartialOrd, Default)]
pub struct VideoHeight(u16);

impl From<u16> for VideoHeight {
    fn from(height: u16) -> Self {
        VideoHeight(height)
    }
}

impl VideoHeight {
    pub fn as_u16(self) -> u16 {
        self.0
    }
}

// Values beyond a multiple of chunk_size are ignored.
// Panics with a chunk_size of 0.
// Just like https://doc.rust-lang.org/beta/std/primitive.slice.html#method.chunks_exact
pub fn count_in_chunks_exact(
    inputs: impl Iterator<Item = bool>,
    chunk_size: usize,
) -> impl Iterator<Item = usize> {
    fold_in_chunks_exact(
        inputs,
        chunk_size,
        || 0usize,
        |count, bit| count + (bit as usize),
    )
}

// Values beyond a multiple of chunk_size are ignored.
// Panics with a chunk_size of 0.
// Just like https://doc.rust-lang.org/beta/std/primitive.slice.html#method.chunks_exact
pub fn fold_in_chunks_exact<Input, Output, Init, Acc>(
    mut inputs: impl Iterator<Item = Input>,
    chunk_size: usize,
    init: Init,
    acc: Acc,
) -> impl Iterator<Item = Output>
where
    Init: Fn() -> Output,
    Acc: Fn(Output, Input) -> Output,
{
    assert!(chunk_size != 0);
    std::iter::from_fn(move || {
        let mut output = init();
        for _ in 0..chunk_size {
            let input = inputs.next()?;
            output = acc(output, input);
        }
        Some(output)
    })
}

pub trait CheckedSplitAt {
    fn checked_split_at(&self, mid: usize) -> Option<(&[u8], &[u8])>;
}

impl CheckedSplitAt for [u8] {
    fn checked_split_at(&self, mid: usize) -> Option<(&[u8], &[u8])> {
        if self.len() < mid {
            None
        } else {
            Some(self.split_at(mid))
        }
    }
}

pub trait CheckedSplitAtMut {
    fn checked_split_at_mut(&mut self, mid: usize) -> Option<(&mut [u8], &mut [u8])>;
}

impl CheckedSplitAtMut for [u8] {
    fn checked_split_at_mut(&mut self, mid: usize) -> Option<(&mut [u8], &mut [u8])> {
        if self.len() < mid {
            None
        } else {
            Some(self.split_at_mut(mid))
        }
    }
}

#[cfg(test)]
lazy_static::lazy_static! {
    pub(crate) static ref RANDOM_SEED_FOR_TESTS: u64 = {
        let seed = match std::env::var("RANDOM_SEED") {
            Ok(v) => v.parse().unwrap(),
            Err(_) => thread_rng().gen(),
        };

        println!("\n*** Using RANDOM_SEED={}", seed);
        seed
    };
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_count_in_groups_exact() {
        let vals: Vec<bool> = vec![];
        assert_eq!(
            vec![0usize; 0],
            count_in_chunks_exact(vals.iter().copied(), 1).collect::<Vec<_>>()
        );

        let vals = [true, false, false, true, true, false, false];
        assert_eq!(
            vec![1, 0, 0, 1, 1, 0, 0],
            count_in_chunks_exact(vals.iter().copied(), 1).collect::<Vec<_>>()
        );
        assert_eq!(
            vec![1, 1, 1],
            count_in_chunks_exact(vals.iter().copied(), 2).collect::<Vec<_>>()
        );
        assert_eq!(
            vec![1, 2],
            count_in_chunks_exact(vals.iter().copied(), 3).collect::<Vec<_>>()
        );
        assert_eq!(
            vec![3],
            count_in_chunks_exact(vals.iter().copied(), 5).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_checked_split_at() {
        assert_eq!(Some((&b""[..], &b"ab"[..])), b"ab".checked_split_at(0));
        assert_eq!(Some((&b"a"[..], &b"b"[..])), b"ab".checked_split_at(1));
        assert_eq!(Some((&b"ab"[..], &b""[..])), b"ab".checked_split_at(2));
        assert_eq!(None, b"ab".checked_split_at(3));
        assert_eq!(None, b"ab".checked_split_at(30));
    }

    #[test]
    #[allow(clippy::clone_on_copy)]
    fn test_checked_split_at_mut() {
        let mut empty = [];
        let mut zero = [0];
        let mut one = [1];
        let mut zero_one = [0, 1];
        assert_eq!(
            Some((&mut empty[..], &mut zero_one.clone()[..])),
            zero_one.checked_split_at_mut(0)
        );
        assert_eq!(
            Some((&mut zero[..], &mut one[..])),
            zero_one.checked_split_at_mut(1)
        );
        assert_eq!(
            Some((&mut zero_one.clone()[..], &mut empty[..])),
            zero_one.checked_split_at_mut(2)
        );
        assert_eq!(None, zero_one.checked_split_at_mut(3));
        assert_eq!(None, zero_one.checked_split_at_mut(30));
    }
}

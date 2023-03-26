use std::{
	marker::PhantomData,
	mem::{self, MaybeUninit},
};

use num_bigint::{BigInt, BigUint, Sign};

use super::ptrs::VecOffsets;
use crate::{Serialize, Serializer};

const PTR_SIZE: usize = mem::size_of::<usize>();

// `BigUint` is just a wrapper around a `Vec<usize>`.
// This does rely on knowledge of `BigUint`'s internal implementation,
// and would break if it changed. But `num-bigint` is a mature crate,
// so this seems unlikely.
impl<S> Serialize<S> for BigUint
where S: Serializer
{
	// Inline because cast produces no machine instructions,
	// so this is exactly equivalent to `Vec<usize>::serialize_data`
	#[inline]
	fn serialize_data(&self, serializer: &mut S) {
		// Compile-time check `BigUint` and `Vec<usize>` have same size and alignment
		let _ = SameSizeAndAlignment::<BigUint, Vec<usize>>::ASSERT_SAME_SIZE_AND_ALIGNMENT;

		let ptr = self as *const BigUint as *const Vec<usize>;
		let vec: &Vec<usize> = unsafe { &*ptr.cast() };
		vec.serialize_data(serializer);
	}
}

// `BigInt` is defined as `BigInt { sign: Sign, data: BigUint }`.
// `Sign` is a fieldless enum, so requires no further serialization.
// But the `BigUint` does. Serialization gets a reference to the `BigUint`
// and serializes it.
impl<S> Serialize<S> for BigInt
where S: Serializer
{
	// Inline because `data_offset` should always be 0 (see below)
	// and cast produces no machine instructions, so this should be exactly
	// equivalent to `BigUint::serialize_data`
	#[inline]
	fn serialize_data(&self, serializer: &mut S) {
		// Compile-time check `BigInt` and `(u8, BigUint)` have same size and alignment
		let _ = SameSizeAndAlignment::<BigInt, (u8, BigUint)>::ASSERT_SAME_SIZE_AND_ALIGNMENT;

		let data_offset = bigint_data_offset();
		let ptr = self as *const BigInt as *const u8;
		let biguint: &BigUint = unsafe { &*ptr.add(data_offset).cast() };
		biguint.serialize_data(serializer);
	}
}

/// `BigInt` is defined as `BigInt { sign: Sign, data: BigUint }`.
/// Get the offset of the `data` field.
///
/// We deduce it by finding offset of the `sign` field.
///
/// Unfortunately it's not possible to do this as a const, because
/// `BigInt::from_biguint` is not a const function. But still everything in this
/// function can be statically evaluated, so it gets compiled down to a static
/// integer. See ASM output:
/// https://play.rust-lang.org/?version=stable&mode=release&edition=2021&gist=16964e78dfb89715902c42d75d05a40f
#[inline]
fn bigint_data_offset() -> usize {
	// Create positive and negative `BigInt`s.
	// Need to use this hack of creating a `BigUint` with len 1, as
	// `BigInt::from_biguint` calls `BigUint::is_zero()` (which calls
	// `Vec::is_empty()`), and if it returns false, it sets `sign` to
	// `Sign::NoSign`, regardless of the sign you pass in.
	// `MaybeUninit<u8>` because it contains padding bytes.
	let positive_bytes = create_bigint_bytes(Sign::Plus);
	let negative_bytes = create_bigint_bytes(Sign::Minus);

	// Sign is either byte 0 or byte 24 (byte 12 on 32-bit system).
	// We check which byte is different between positive + negative `BigInt`s.
	// If the sign byte byte 0, then the `BigUint` must occupy bytes 8-31.
	// Otherwise, the `BigUint` must occupy bytes 0-23.
	// The latter should always be the case, unless layout randomization is used,
	// because `BigUint` has alignment 8 (4 on 32-bit systems), and `Sign` has
	// alignment 1. `BigInt` is `repr(rust)`, so Rust will put `sign` field last.
	// But don't want to rely on that assumption, so calculate it here.
	unsafe {
		let start_match = positive_bytes[0].assume_init() == negative_bytes[0].assume_init();
		let end_match =
			positive_bytes[PTR_SIZE * 3].assume_init() == negative_bytes[PTR_SIZE * 3].assume_init();
		if start_match {
			assert!(!end_match);
			0
		} else {
			assert!(!start_match);
			PTR_SIZE
		}
	}
}

#[inline(always)]
fn create_bigint_bytes(sign: Sign) -> [MaybeUninit<u8>; PTR_SIZE * 4] {
	// Create an illegal `BigUint` with len 1.
	// This `BigUint` must NOT be dropped, as it contains an illegal `Vec<usize>`
	// with len 1 and capacity 0. Dropping it could cause UB.
	let mut biguint = BigUint::default();
	let ptr = &mut biguint as *mut BigUint as *mut usize;
	let len_offset = VecOffsets::<usize>::OFFSETS_VEC.len() / PTR_SIZE;
	unsafe { ptr.add(len_offset).write(1) };

	// Create `BigInt` wrapping the illegal `BigUint`
	let bigint = BigInt::from_biguint(sign, biguint);

	// Transmute to array of `MaybeUninit<u8>`s. It's now safe to drop.
	unsafe { mem::transmute(bigint) }
}

/// Type for static assertion that 2 types have same size and alignment
struct SameSizeAndAlignment<T1, T2> {
	_marker1: PhantomData<T1>,
	_marker2: PhantomData<T2>,
}

impl<T1, T2> SameSizeAndAlignment<T1, T2> {
	const ASSERT_SAME_SIZE_AND_ALIGNMENT: () = {
		assert!(mem::size_of::<T1>() == mem::size_of::<T2>());
		assert!(mem::align_of::<T1>() == mem::align_of::<T2>());
	};
}

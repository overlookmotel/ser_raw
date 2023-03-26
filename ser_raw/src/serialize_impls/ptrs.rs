use std::{marker::PhantomData, mem};

use crate::{pos::Addr, Serialize, Serializer};

const PTR_SIZE: usize = mem::size_of::<usize>();

impl<T, S> Serialize<S> for Box<T>
where
	S: Serializer,
	T: Serialize<S> + Sized,
{
	fn serialize_data(&self, serializer: &mut S) {
		// Sanity check that `Box<T>` is just a pointer (evaluated at compile time).
		// Unsized types are not supported.
		let _ = SizeCheck::<Box<T>, PTR_SIZE>::ASSERT_SIZE_IS;

		// No need to do anything if box contains ZST
		// TODO: Should we call `serialize_data()` in case user defines some behavior?
		if mem::size_of::<T>() == 0 {
			return;
		}

		// Write boxed value
		let ptr_addr = S::Addr::from_ref(self);
		serializer.push_and_process(&**self, ptr_addr, |serializer| {
			// Serialize boxed value
			(**self).serialize_data(serializer);
		});
	}
}

impl<T, S> Serialize<S> for Vec<T>
where
	S: Serializer,
	T: Serialize<S>,
{
	fn serialize_data(&self, serializer: &mut S) {
		// No need to do anything if vec contains ZSTs
		// TODO: Should we call `serialize_data()` in case user defines some behavior?
		if mem::size_of::<T>() == 0 {
			return;
		}

		// No need to write contents if vec is empty
		if self.len() == 0 {
			// Overwrite `capacity = 0` and `ptr = <dangling>` if it's not already
			serializer.write_correction(|serializer| {
				if self.capacity() != 0 {
					unsafe { write_capacity_and_ptr_for_empty_vec(self, serializer) };
				}
			});

			return;
		}

		// Overwrite `capacity = len`, if it's not already
		serializer.write_correction(|serializer| {
			if self.capacity() != self.len() {
				let cap_offset = VecOffsets::<T>::OFFSETS_VEC.capacity();
				let cap_addr = S::Addr::from_ref_offset(self, cap_offset).addr();
				unsafe { serializer.write(&self.len(), cap_addr) };
			}
		});

		// Write vec's contents
		let ptr_addr = S::Addr::from_ref_offset(self, VecOffsets::<T>::PTR_OFFSET);
		serializer.push_and_process_slice(self.as_slice(), ptr_addr, |serializer| {
			// Serialize vec's contents
			for value in &**self {
				value.serialize_data(serializer);
			}
		});
	}
}

impl<S> Serialize<S> for String
where S: Serializer
{
	fn serialize_data(&self, serializer: &mut S) {
		// No need to write contents if string is empty
		if self.len() == 0 {
			// Overwrite `capacity = 0` and `ptr = <dangling>` if it's not already
			serializer.write_correction(|serializer| {
				if self.capacity() != 0 {
					unsafe { write_capacity_and_ptr_for_empty_string(self, serializer) };
				}
			});

			return;
		}

		// Overwrite `capacity = len`, if it's not already
		serializer.write_correction(|serializer| {
			if self.capacity() != self.len() {
				let cap_offset = OFFSETS_STRING.capacity();
				let cap_addr = S::Addr::from_ref_offset(self, cap_offset).addr();
				unsafe { serializer.write(&self.len(), cap_addr) };
			}
		});

		// Write string's content
		let ptr_addr = S::Addr::from_ref_offset(self, STRING_PTR_OFFSET);
		serializer.push_slice(self.as_bytes(), ptr_addr);
	}
}

/// Type for static assertion of size of type
struct SizeCheck<T, const SIZE: usize> {
	_marker: PhantomData<T>,
}

impl<T, const SIZE: usize> SizeCheck<T, SIZE> {
	const ASSERT_SIZE_IS: () = assert!(mem::size_of::<T>() == SIZE);
}

/// Type for calculating offset of fields in `Vec<T>` at compile time.
///
/// * Offset of `ptr` field: `VecOffsets::<T>::PTR_OFFSET`
/// * Offset of `len` field: `VecOffsets::<T>::OFFSETS_VEC.len()`.
/// * Offset of `capacity` field: `VecOffsets::<T>::OFFSETS_VEC.capacity()`.
///
/// Godbolt shows all of these are compiled down to static integers:
/// https://godbolt.org/z/78MzTKo6f
pub(crate) struct VecOffsets<T> {
	_marker: PhantomData<T>,
}

impl<T> VecOffsets<T> {
	const PTR_INDEX: usize = {
		// Empty vec does not allocate
		let vec = Vec::<T>::new();
		// Will fail to compile if `Vec<T>` is not implemented as 3 x `usize`
		let bytes: [usize; 3] = unsafe { mem::transmute(vec) };
		let dangle = mem::align_of::<T>();
		if bytes[0] == dangle {
			assert!(bytes[1] == 0 && bytes[2] == 0);
			0
		} else if bytes[1] == dangle {
			assert!(bytes[0] == 0 && bytes[2] == 0);
			1
		} else if bytes[2] == dangle {
			assert!(bytes[0] == 0 && bytes[1] == 0);
			2
		} else {
			panic!("Could not determine offset of Vec's ptr field");
		}
	};

	pub(crate) const PTR_OFFSET: usize = Self::PTR_INDEX * PTR_SIZE;

	// `OFFSETS_VEC` is not a valid `Vec<T>` as it violates `Vec`'s invariants.
	// Either `len` > `capacity`, or `capacity` > 0 and ptr dangling.
	// However:
	// 1. We at least ensure ptr is non-null.
	// 2. We never read or write to the vec, or access its pointer.
	// 3. `ManuallyDrop` prevents it ever being dropped (which would be UB).
	// So this hack is *probably* sound.
	pub(crate) const OFFSETS_VEC: mem::ManuallyDrop<Vec<T>> = {
		let dangle = mem::align_of::<T>();
		let bytes = match Self::PTR_INDEX {
			0 => [dangle, PTR_SIZE, PTR_SIZE * 2],
			1 => [0, dangle, PTR_SIZE * 2],
			2 => [0, PTR_SIZE, dangle],
			_ => unreachable!(),
		};
		unsafe { mem::transmute(bytes) }
	};
}

// Constants for offset of fields in `String`, calculated at compile time.
// Uses same hack as `VecOffsets` above.
//
// * Offset of `ptr` field: `STRING_PTR_OFFSET`
// * Offset of `len` field: `OFFSETS_STRING.len()`.
// * Offset of `capacity` field: `OFFSETS_STRING.capacity()`.
const STRING_PTR_INDEX: usize = {
	// Empty string does not allocate
	let s = String::new();
	// Will fail to compile if `String` is not implemented as 3 x `usize`
	let bytes: [usize; 3] = unsafe { mem::transmute(s) };
	let dangle = 1;
	if bytes[0] == dangle {
		assert!(bytes[1] == 0 && bytes[2] == 0);
		0
	} else if bytes[1] == dangle {
		assert!(bytes[0] == 0 && bytes[2] == 0);
		1
	} else if bytes[2] == dangle {
		assert!(bytes[0] == 0 && bytes[1] == 0);
		2
	} else {
		panic!("Could not determine offset of String's ptr field");
	}
};
const STRING_PTR_OFFSET: usize = STRING_PTR_INDEX * PTR_SIZE;

const OFFSETS_STRING: mem::ManuallyDrop<String> = {
	let dangle = 1;
	let bytes = match STRING_PTR_INDEX {
		0 => [dangle, PTR_SIZE, PTR_SIZE * 2],
		1 => [0, dangle, PTR_SIZE * 2],
		2 => [0, PTR_SIZE, dangle],
		_ => unreachable!(),
	};
	unsafe { mem::transmute(bytes) }
};

/// Overwrite `capacity` and `ptr` for empty `Vec<T>`.
///
/// Will write both in a single write if the two fields are next to each other,
/// or fall back to writing each individually. They should be next to each other
/// as they're both within `RawVec` in Rust's current `Vec` implementation.
///
/// `VecOffsets::<T>::OFFSETS_VEC.capacity()`, `VecOffsets::<T>::PTR_OFFSET`,
/// and `mem::align_of::<T>()` can all be statically evaluated.
/// So compiler should remove all but one branch and reduce this whole function
/// down to e.g. `serializer.write(&[0, 8], v as *const Vec<T> as usize)`.
/// Godbolt seems to confirm this: https://godbolt.org/z/nr5b5jn3x
#[inline]
unsafe fn write_capacity_and_ptr_for_empty_vec<T, Ser: Serializer>(
	v: &Vec<T>,
	serializer: &mut Ser,
) {
	// We know `mem::align_of::<T>()` is correct value for a dangling ptr or
	// calculating `VecOffsets::<T>::PTR_INDEX` would have errored
	let dangle = mem::align_of::<T>();
	let cap_offset = VecOffsets::<T>::OFFSETS_VEC.capacity();
	let ptr_offset = VecOffsets::<T>::PTR_OFFSET;

	if cap_offset == 0 && ptr_offset == PTR_SIZE {
		serializer.write(&[0, dangle], Ser::Addr::from_ref(v).addr());
	} else if cap_offset == PTR_SIZE && ptr_offset == 0 {
		serializer.write(&[dangle, 0], Ser::Addr::from_ref(v).addr());
	} else if cap_offset == PTR_SIZE && ptr_offset == PTR_SIZE * 2 {
		serializer.write(&[0, dangle], Ser::Addr::from_ref_offset(v, PTR_SIZE).addr());
	} else if cap_offset == PTR_SIZE * 2 && ptr_offset == PTR_SIZE {
		serializer.write(&[dangle, 0], Ser::Addr::from_ref_offset(v, PTR_SIZE).addr());
	} else {
		serializer.write(&0usize, Ser::Addr::from_ref_offset(v, cap_offset).addr());
		serializer.write(&dangle, Ser::Addr::from_ref_offset(v, ptr_offset).addr());
	}
}

/// Overwrite `capacity` and `ptr` for empty `String`.
///
/// Will write both in a single write if the two fields are next to each other,
/// or fall back to writing each individually. They should be next to each other
/// as they're both within `RawVec` in Rust's current `String` implementation.
///
/// `OFFSETS_STRING.capacity()` and `STRING_PTR_OFFSET` can both be statically
/// evaluated.
/// So compiler should remove all but one branch and reduce this whole function
/// down to e.g. `serializer.write(&[0, 8], s as *const String as usize)`.
#[inline]
unsafe fn write_capacity_and_ptr_for_empty_string<Ser: Serializer>(
	s: &String,
	serializer: &mut Ser,
) {
	// We know 1 is correct value for a dangling ptr or calculating
	// `STRING_PTR_INDEX` would have errored
	let dangle = 1usize;
	let cap_offset = OFFSETS_STRING.capacity();

	if cap_offset == 0 && STRING_PTR_OFFSET == PTR_SIZE {
		serializer.write(&[0, dangle], Ser::Addr::from_ref(s).addr());
	} else if cap_offset == PTR_SIZE && STRING_PTR_OFFSET == 0 {
		serializer.write(&[dangle, 0], Ser::Addr::from_ref(s).addr());
	} else if cap_offset == PTR_SIZE && STRING_PTR_OFFSET == PTR_SIZE * 2 {
		serializer.write(&[0, dangle], Ser::Addr::from_ref_offset(s, PTR_SIZE).addr());
	} else if cap_offset == PTR_SIZE * 2 && STRING_PTR_OFFSET == PTR_SIZE {
		serializer.write(&[dangle, 0], Ser::Addr::from_ref_offset(s, PTR_SIZE).addr());
	} else {
		serializer.write(&0usize, Ser::Addr::from_ref_offset(s, cap_offset).addr());
		serializer.write(
			&dangle,
			Ser::Addr::from_ref_offset(s, STRING_PTR_OFFSET).addr(),
		);
	}
}

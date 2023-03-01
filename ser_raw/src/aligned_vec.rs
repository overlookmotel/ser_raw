use std::{
	alloc,
	borrow::{Borrow, BorrowMut},
	fmt,
	io::{self, ErrorKind, Read},
	ops::{Deref, DerefMut, Index, IndexMut},
	ptr::NonNull,
	slice,
};

/// A vector of bytes that aligns its memory to specified alignment.
///
/// Implementation is a direct copy from
/// [rkyv::AlignedVec](https://docs.rs/rkyv/latest/rkyv/util/struct.AlignedVec.html)
/// but including the changes from [PR #353](https://github.com/rkyv/rkyv/pull/353)
/// for custom alignment.
///
/// ```
/// use rkyv::{AlignedByteVec};
///
/// let bytes = AlignedByteVec::<4096>::with_capacity(1);
/// assert_eq!(bytes.as_ptr() as usize % 4096, 0);
/// ```
pub struct AlignedByteVec<const ALIGNMENT: usize = 16> {
	ptr: NonNull<u8>,
	cap: usize,
	len: usize,
}

impl<const A: usize> Drop for AlignedByteVec<A> {
	#[inline]
	fn drop(&mut self) {
		if self.cap != 0 {
			unsafe {
				alloc::dealloc(self.ptr.as_ptr(), self.layout());
			}
		}
	}
}

impl<const ALIGNMENT: usize> AlignedByteVec<ALIGNMENT> {
	/// The alignment of the vector
	pub const ALIGNMENT: usize = ALIGNMENT;
	const ASSERT_ALIGNMENT_VALID: () = {
		assert!(ALIGNMENT > 0, "ALIGNMENT must be 1 or more");
		assert!(
			ALIGNMENT == ALIGNMENT.next_power_of_two(),
			"ALIGNMENT must be a power of 2"
		);
		// As `ALIGNMENT` has to be a power of 2, this caps `ALIGNMENT`
		// at max of `(isize::MAX + 1) / 2` (1 GiB on 32-bit systems)
		assert!(
			ALIGNMENT < isize::MAX as usize,
			"ALIGNMENT must be less than isize::MAX"
		);
	};
	/// Maximum capacity of the vector.
	/// Dictated by the requirements of
	/// [`alloc::Layout`](https://doc.rust-lang.org/alloc/alloc/struct.Layout.html).
	/// "`size`, when rounded up to the nearest multiple of `align`, must not
	/// overflow `isize` (i.e. the rounded value must be less than or equal to
	/// `isize::MAX`)".
	pub const MAX_CAPACITY: usize = isize::MAX as usize - (Self::ALIGNMENT - 1);

	/// Constructs a new, empty `AlignedVec`.
	///
	/// The vector will not allocate until elements are pushed into it.
	///
	/// # Examples
	/// ```
	/// use rkyv::AlignedVec;
	///
	/// let mut vec = AlignedVec::new();
	/// ```
	#[inline]
	pub fn new() -> Self {
		let _ = Self::ASSERT_ALIGNMENT_VALID;

		Self {
			ptr: NonNull::dangling(),
			cap: 0,
			len: 0,
		}
	}

	/// Constructs a new, empty `AlignedVec` with the specified capacity.
	///
	/// The vector will be able to hold exactly `capacity` bytes without
	/// reallocating. If `capacity` is 0, the vector will not allocate.
	///
	/// # Examples
	/// ```
	/// use rkyv::AlignedVec;
	///
	/// let mut vec = AlignedVec::with_capacity(10);
	///
	/// // The vector contains no items, even though it has capacity for more
	/// assert_eq!(vec.len(), 0);
	/// assert_eq!(vec.capacity(), 10);
	///
	/// // These are all done without reallocating...
	/// for i in 0..10 {
	///     vec.push(i);
	/// }
	/// assert_eq!(vec.len(), 10);
	/// assert_eq!(vec.capacity(), 10);
	///
	/// // ...but this may make the vector reallocate
	/// vec.push(11);
	/// assert_eq!(vec.len(), 11);
	/// assert!(vec.capacity() >= 11);
	/// ```
	#[inline]
	pub fn with_capacity(capacity: usize) -> Self {
		let _ = Self::ASSERT_ALIGNMENT_VALID;

		if capacity == 0 {
			Self::new()
		} else {
			assert!(
				capacity <= Self::MAX_CAPACITY,
				"`capacity` cannot exceed isize::MAX - 15"
			);
			let ptr = unsafe {
				let layout = alloc::Layout::from_size_align_unchecked(capacity, Self::ALIGNMENT);
				let ptr = alloc::alloc(layout);
				if ptr.is_null() {
					alloc::handle_alloc_error(layout);
				}
				NonNull::new_unchecked(ptr)
			};
			Self {
				ptr,
				cap: capacity,
				len: 0,
			}
		}
	}

	#[inline]
	fn layout(&self) -> alloc::Layout {
		unsafe { alloc::Layout::from_size_align_unchecked(self.cap, Self::ALIGNMENT) }
	}

	/// Clears the vector, removing all values.
	///
	/// Note that this method has no effect on the allocated capacity of the
	/// vector.
	///
	/// # Examples
	/// ```
	/// use rkyv::AlignedVec;
	///
	/// let mut v = AlignedVec::new();
	/// v.extend_from_slice(&[1, 2, 3, 4]);
	///
	/// v.clear();
	///
	/// assert!(v.is_empty());
	/// ```
	#[inline]
	pub fn clear(&mut self) {
		self.len = 0;
	}

	/// Change capacity of vector.
	///
	/// # Safety
	///
	/// - `new_cap` must be less than or equal to
	///   [`MAX_CAPACITY`](AlignedVec::MAX_CAPACITY)
	/// - `new_cap` must be greater than or equal to [`len()`](AlignedVec::len)
	#[inline]
	unsafe fn change_capacity(&mut self, new_cap: usize) {
		let new_ptr = if self.cap != 0 {
			let new_ptr = alloc::realloc(self.ptr.as_ptr(), self.layout(), new_cap);
			if new_ptr.is_null() {
				alloc::handle_alloc_error(alloc::Layout::from_size_align_unchecked(
					new_cap,
					Self::ALIGNMENT,
				));
			}
			new_ptr
		} else {
			let layout = alloc::Layout::from_size_align_unchecked(new_cap, Self::ALIGNMENT);
			let new_ptr = alloc::alloc(layout);
			if new_ptr.is_null() {
				alloc::handle_alloc_error(layout);
			}
			new_ptr
		};
		self.ptr = NonNull::new_unchecked(new_ptr);
		self.cap = new_cap;
	}

	/// Shrinks the capacity of the vector as much as possible.
	///
	/// It will drop down as close as possible to the length but the allocator may
	/// still inform the vector that there is space for a few more elements.
	///
	/// # Examples
	/// ```
	/// use rkyv::AlignedVec;
	///
	/// let mut vec = AlignedVec::with_capacity(10);
	/// vec.extend_from_slice(&[1, 2, 3]);
	/// assert_eq!(vec.capacity(), 10);
	/// vec.shrink_to_fit();
	/// assert!(vec.capacity() >= 3);
	///
	/// vec.clear();
	/// vec.shrink_to_fit();
	/// assert!(vec.capacity() == 0);
	/// ```
	#[inline]
	pub fn shrink_to_fit(&mut self) {
		if self.cap != self.len {
			// New capacity cannot exceed max as it's shrinking
			unsafe { self.change_capacity(self.len) };
		}
	}

	/// Returns an unsafe mutable pointer to the vector's buffer.
	///
	/// The caller must ensure that the vector outlives the pointer this function
	/// returns, or else it will end up pointing to garbage. Modifying the vector
	/// may cause its buffer to be reallocated, which would also make any pointers
	/// to it invalid.
	///
	/// # Examples
	/// ```
	/// use rkyv::AlignedVec;
	///
	/// // Allocate vecotr big enough for 4 bytes.
	/// let size = 4;
	/// let mut x = AlignedVec::with_capacity(size);
	/// let x_ptr = x.as_mut_ptr();
	///
	/// // Initialize elements via raw pointer writes, then set length.
	/// unsafe {
	///     for i in 0..size {
	///         *x_ptr.add(i) = i as u8;
	///     }
	///     x.set_len(size);
	/// }
	/// assert_eq!(&*x, &[0, 1, 2, 3]);
	/// ```
	#[inline]
	pub fn as_mut_ptr(&mut self) -> *mut u8 {
		self.ptr.as_ptr()
	}

	/// Extracts a mutable slice of the entire vector.
	///
	/// Equivalent to `&mut s[..]`.
	///
	/// # Examples
	/// ```
	/// use rkyv::AlignedVec;
	///
	/// let mut vec = AlignedVec::new();
	/// vec.extend_from_slice(&[1, 2, 3, 4, 5]);
	/// assert_eq!(vec.as_mut_slice().len(), 5);
	/// for i in 0..5 {
	///     assert_eq!(vec.as_mut_slice()[i], i as u8 + 1);
	///     vec.as_mut_slice()[i] = i as u8;
	///     assert_eq!(vec.as_mut_slice()[i], i as u8);
	/// }
	/// ```
	#[inline]
	pub fn as_mut_slice(&mut self) -> &mut [u8] {
		unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
	}

	/// Returns a raw pointer to the vector's buffer.
	///
	/// The caller must ensure that the vector outlives the pointer this function
	/// returns, or else it will end up pointing to garbage. Modifying the vector
	/// may cause its buffer to be reallocated, which would also make any pointers
	/// to it invalid.
	///
	/// The caller must also ensure that the memory the pointer (non-transitively)
	/// points to is never written to (except inside an `UnsafeCell`) using this
	/// pointer or any pointer derived from it. If you need to mutate the contents
	/// of the slice, use [`as_mut_ptr`](AlignedVec::as_mut_ptr).
	///
	/// # Examples
	/// ```
	/// use rkyv::AlignedVec;
	///
	/// let mut x = AlignedVec::new();
	/// x.extend_from_slice(&[1, 2, 4]);
	/// let x_ptr = x.as_ptr();
	///
	/// unsafe {
	///     for i in 0..x.len() {
	///         assert_eq!(*x_ptr.add(i), 1 << i);
	///     }
	/// }
	/// ```
	#[inline]
	pub fn as_ptr(&self) -> *const u8 {
		self.ptr.as_ptr()
	}

	/// Extracts a slice containing the entire vector.
	///
	/// Equivalent to `&s[..]`.
	///
	/// # Examples
	/// ```
	/// use rkyv::AlignedVec;
	///
	/// let mut vec = AlignedVec::new();
	/// vec.extend_from_slice(&[1, 2, 3, 4, 5]);
	/// assert_eq!(vec.as_slice().len(), 5);
	/// for i in 0..5 {
	///     assert_eq!(vec.as_slice()[i], i as u8 + 1);
	/// }
	/// ```
	#[inline]
	pub fn as_slice(&self) -> &[u8] {
		unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
	}

	/// Returns the number of elements the vector can hold without reallocating.
	///
	/// # Examples
	/// ```
	/// use rkyv::AlignedVec;
	///
	/// let vec = AlignedVec::with_capacity(10);
	/// assert_eq!(vec.capacity(), 10);
	/// ```
	#[inline]
	pub fn capacity(&self) -> usize {
		self.cap
	}

	/// Reserves capacity for at least `additional` more bytes to be inserted into
	/// the given `AlignedVec`. The collection may reserve more space to avoid
	/// frequent reallocations. After calling `reserve`, capacity will be greater
	/// than or equal to `self.len() + additional`. Does nothing if capacity is
	/// already sufficient.
	///
	/// # Panics
	///
	/// Panics if the new capacity exceeds `isize::MAX - 15` bytes.
	///
	/// # Examples
	/// ```
	/// use rkyv::AlignedVec;
	///
	/// let mut vec = AlignedVec::new();
	/// vec.push(1);
	/// vec.reserve(10);
	/// assert!(vec.capacity() >= 11);
	/// ```
	#[inline]
	pub fn reserve(&mut self, additional: usize) {
		// Cannot wrap because capacity always exceeds len,
		// but avoids having to handle potential overflow here
		let remaining = self.cap.wrapping_sub(self.len);
		if additional > remaining {
			self.do_reserve(additional);
		}
	}

	/// Extend capacity after `reserve` has found it's necessary.
	///
	/// Actually performing the extension is in this separate function marked
	/// `#[cold]` to hint to compiler that this branch is not often taken.
	/// This keeps the path for common case where capacity is already sufficient
	/// as fast as possible, and makes `reserve` more likely to be inlined.
	/// This is the same trick that Rust's `Vec::reserve` uses.
	#[cold]
	fn do_reserve(&mut self, additional: usize) {
		let new_cap = self
			.len
			.checked_add(additional)
			.expect("cannot reserve a larger AlignedVec");
		unsafe { self.grow_capacity_to(new_cap) };
	}

	/// Increase total capacity of vector to `new_cap` or more.
	///
	/// Actual capacity will be `new_cap` rounded up to next power of 2,
	/// unless that would exceed maximum capacity, in which case capacity
	/// is capped at the maximum.
	///
	/// This is same growth strategy used by `reserve`, and therefore also
	/// by `push` and `extend_from_slice`.
	///
	/// If you want to reserve an exact amount of additional space,
	/// use `reserve_exact` instead.
	///
	/// Maximum capacity is `isize::MAX - 15` bytes.
	///
	/// # Panics
	///
	/// Panics if the `new_cap` exceeds `isize::MAX - 15` bytes.
	///
	/// # Safety
	///
	/// - `new_cap` must be greater than current
	///   [`capacity()`](AlignedVec::capacity)
	///
	/// # Examples
	/// ```
	/// use rkyv::AlignedVec;
	///
	/// let mut vec = AlignedVec::new();
	/// vec.push(1);
	/// unsafe { vec.increase_capacity(50) };
	/// assert_eq!(vec.len(), 1);
	/// assert_eq!(vec.capacity(), 64);
	/// ```
	#[inline]
	pub unsafe fn grow_capacity_to(&mut self, new_cap: usize) {
		let new_cap = if new_cap > (isize::MAX as usize + 1) >> 1 {
			// Rounding up to next power of 2 would result in `isize::MAX + 1` or higher,
			// which exceeds max capacity. So cap at max instead.
			assert!(
				new_cap <= Self::MAX_CAPACITY,
				"cannot reserve a larger AlignedVec"
			);
			Self::MAX_CAPACITY
		} else {
			// Cannot overflow due to check above
			new_cap.next_power_of_two()
		};
		self.change_capacity(new_cap);
	}

	/// Resizes the Vec in-place so that len is equal to new_len.
	///
	/// If new_len is greater than len, the Vec is extended by the difference,
	/// with each additional slot filled with value. If new_len is less than len,
	/// the Vec is simply truncated.
	///
	/// # Panics
	///
	/// Panics if the new length exceeds `isize::MAX - 15` bytes.
	///
	/// # Examples
	/// ```
	/// use rkyv::AlignedVec;
	///
	/// let mut vec = AlignedVec::new();
	/// vec.push(3);
	/// vec.resize(3, 2);
	/// assert_eq!(vec.as_slice(), &[3, 2, 2]);
	///
	/// let mut vec = AlignedVec::new();
	/// vec.extend_from_slice(&[1, 2, 3, 4]);
	/// vec.resize(2, 0);
	/// assert_eq!(vec.as_slice(), &[1, 2]);
	/// ```
	#[inline]
	pub fn resize(&mut self, new_len: usize, value: u8) {
		if new_len > self.len {
			let additional = new_len - self.len;
			self.reserve(additional);
			unsafe {
				core::ptr::write_bytes(self.ptr.as_ptr().add(self.len), value, additional);
			}
		}
		unsafe {
			self.set_len(new_len);
		}
	}

	/// Returns `true` if the vector contains no elements.
	///
	/// # Examples
	/// ```
	/// use rkyv::AlignedVec;
	///
	/// let mut v = Vec::new();
	/// assert!(v.is_empty());
	///
	/// v.push(1);
	/// assert!(!v.is_empty());
	/// ```
	#[inline]
	pub fn is_empty(&self) -> bool {
		self.len == 0
	}

	/// Returns the number of elements in the vector, also referred to as its
	/// 'length'.
	///
	/// # Examples
	/// ```
	/// use rkyv::AlignedVec;
	///
	/// let mut a = AlignedVec::new();
	/// a.extend_from_slice(&[1, 2, 3]);
	/// assert_eq!(a.len(), 3);
	/// ```
	#[inline]
	pub fn len(&self) -> usize {
		self.len
	}

	/// Copies and appends all bytes in a slice to the `AlignedVec`.
	///
	/// The elements of the slice are appended in-order.
	///
	/// # Examples
	/// ```
	/// use rkyv::AlignedVec;
	///
	/// let mut vec = AlignedVec::new();
	/// vec.push(1);
	/// vec.extend_from_slice(&[2, 3, 4]);
	/// assert_eq!(vec.as_slice(), &[1, 2, 3, 4]);
	/// ```
	#[inline]
	pub fn extend_from_slice(&mut self, other: &[u8]) {
		if !other.is_empty() {
			self.reserve(other.len());
			unsafe {
				core::ptr::copy_nonoverlapping(
					other.as_ptr(),
					self.as_mut_ptr().add(self.len()),
					other.len(),
				);
			}
			self.len += other.len();
		}
	}

	/// Removes the last element from a vector and returns it, or `None` if it is
	/// empty.
	///
	/// # Examples
	/// ```
	/// use rkyv::AlignedVec;
	///
	/// let mut vec = AlignedVec::new();
	/// vec.extend_from_slice(&[1, 2, 3]);
	/// assert_eq!(vec.pop(), Some(3));
	/// assert_eq!(vec.as_slice(), &[1, 2]);
	/// ```
	#[inline]
	pub fn pop(&mut self) -> Option<u8> {
		if self.len == 0 {
			None
		} else {
			let result = self[self.len - 1];
			self.len -= 1;
			Some(result)
		}
	}

	/// Appends an element to the back of a collection.
	///
	/// # Panics
	///
	/// Panics if the new capacity exceeds `isize::MAX - 15` bytes.
	///
	/// # Examples
	/// ```
	/// use rkyv::AlignedVec;
	///
	/// let mut vec = AlignedVec::new();
	/// vec.extend_from_slice(&[1, 2]);
	/// vec.push(3);
	/// assert_eq!(vec.as_slice(), &[1, 2, 3]);
	/// ```
	#[inline]
	pub fn push(&mut self, value: u8) {
		if self.len == self.cap {
			self.reserve_for_push();
		}

		unsafe {
			self.as_mut_ptr().add(self.len).write(value);
			self.len += 1;
		}
	}

	/// Extend capacity by at least 1 byte after `push` has found it's necessary.
	///
	/// Actually performing the extension is in this separate function marked
	/// `#[cold]` to hint to compiler that this branch is not often taken.
	/// This keeps the path for common case where capacity is already sufficient
	/// as fast as possible, and makes `push` more likely to be inlined.
	/// This is the same trick that Rust's `Vec::push` uses.
	#[cold]
	fn reserve_for_push(&mut self) {
		// `len` is always less than `isize::MAX`, so no possibility of overflow here
		let new_cap = self.len + 1;
		unsafe { self.grow_capacity_to(new_cap) };
	}

	/// Reserves the minimum capacity for exactly `additional` more elements to be
	/// inserted in the given `AlignedVec`. After calling `reserve_exact`,
	/// capacity will be greater than or equal to `self.len() + additional`. Does
	/// nothing if the capacity is already sufficient.
	///
	/// Note that the allocator may give the collection more space than it
	/// requests. Therefore, capacity can not be relied upon to be precisely
	/// minimal. Prefer reserve if future insertions are expected.
	///
	/// # Panics
	///
	/// Panics if the new capacity overflows `isize::MAX - 15`.
	///
	/// # Examples
	/// ```
	/// use rkyv::AlignedVec;
	///
	/// let mut vec = AlignedVec::new();
	/// vec.push(1);
	/// vec.reserve_exact(10);
	/// assert!(vec.capacity() >= 11);
	/// ```
	#[inline]
	pub fn reserve_exact(&mut self, additional: usize) {
		// This function does not use the hot/cold paths trick that `reserve`
		// and `push` do, on assumption that user probably knows this will require
		// an increase in capacity. Otherwise, they'd likely use `reserve`.
		let new_cap = self
			.len
			.checked_add(additional)
			.expect("cannot reserve a larger AlignedVec");
		if new_cap > self.cap {
			assert!(
				new_cap <= Self::MAX_CAPACITY,
				"cannot reserve a larger AlignedVec"
			);
			unsafe { self.change_capacity(new_cap) };
		}
	}

	/// Forces the length of the vector to `new_len`.
	///
	/// This is a low-level operation that maintains none of the normal invariants
	/// of the type.
	///
	/// # Safety
	///
	/// - `new_len` must be less than or equal to
	///   [`capacity()`](AlignedVec::capacity)
	/// - The elements at `old_len..new_len` must be initialized
	///
	/// # Examples
	/// ```
	/// use rkyv::AlignedVec;
	///
	/// let mut vec = AlignedVec::with_capacity(3);
	/// vec.extend_from_slice(&[1, 2, 3]);
	///
	/// // SAFETY:
	/// // 1. `old_len..0` is empty to no elements need to be initialized.
	/// // 2. `0 <= capacity` always holds whatever capacity is.
	/// unsafe {
	///     vec.set_len(0);
	/// }
	/// ```
	#[inline]
	pub unsafe fn set_len(&mut self, new_len: usize) {
		debug_assert!(new_len <= self.capacity());

		self.len = new_len;
	}

	/// Converts the vector into `Box<[u8]>`.
	///
	/// This method reallocates and copies the underlying bytes. Any excess
	/// capacity is dropped.
	///
	/// # Examples
	/// ```
	/// use rkyv::AlignedVec;
	///
	/// let mut v = AlignedVec::new();
	/// v.extend_from_slice(&[1, 2, 3]);
	///
	/// let slice = v.into_boxed_slice();
	/// ```
	///
	/// Any excess capacity is removed:
	///
	/// ```
	/// use rkyv::AlignedVec;
	///
	/// let mut vec = AlignedVec::with_capacity(10);
	/// vec.extend_from_slice(&[1, 2, 3]);
	///
	/// assert_eq!(vec.capacity(), 10);
	/// let slice = vec.into_boxed_slice();
	/// assert_eq!(slice.len(), 3);
	/// ```
	#[inline]
	pub fn into_boxed_slice(self) -> Box<[u8]> {
		self.into_vec().into_boxed_slice()
	}

	/// Converts the vector into `Vec<u8>`.
	///
	/// This method reallocates and copies the underlying bytes. Any excess
	/// capacity is dropped.
	///
	/// # Examples
	/// ```
	/// use rkyv::AlignedVec;
	///
	/// let mut v = AlignedVec::new();
	/// v.extend_from_slice(&[1, 2, 3]);
	///
	/// let vec = v.into_vec();
	/// assert_eq!(vec.len(), 3);
	/// assert_eq!(vec.as_slice(), &[1, 2, 3]);
	/// ```
	#[inline]
	pub fn into_vec(self) -> Vec<u8> {
		Vec::from(self.as_ref())
	}

	/// Reads all bytes until EOF from `r` and appends them to this
	/// `AlignedVec`.
	///
	/// If successful, this function will return the total number of bytes read.
	///
	/// # Examples
	/// ```
	/// use rkyv::AlignedVec;
	///
	/// let source = (0..4096).map(|x| (x % 256) as u8).collect::<Vec<_>>();
	/// let mut bytes = AlignedVec::new();
	/// bytes.extend_from_reader(&mut source.as_slice()).unwrap();
	///
	/// assert_eq!(bytes.len(), 4096);
	/// assert_eq!(bytes[0], 0);
	/// assert_eq!(bytes[100], 100);
	/// assert_eq!(bytes[2945], 129);
	/// ```
	pub fn extend_from_reader<R: Read + ?Sized>(&mut self, r: &mut R) -> std::io::Result<usize> {
		let start_len = self.len();
		let start_cap = self.capacity();

		// Extra initialized bytes from previous loop iteration.
		let mut initialized = 0;
		loop {
			if self.len() == self.capacity() {
				// No available capacity, reserve some space.
				self.reserve(32);
			}

			let read_buf_start = unsafe { self.as_mut_ptr().add(self.len) };
			let read_buf_len = self.capacity() - self.len();

			// Initialize the uninitialized portion of the available space.
			unsafe {
				// The first `initialized` bytes don't need to be zeroed.
				// This leaves us `read_buf_len - initialized` bytes to zero
				// starting at `initialized`.
				core::ptr::write_bytes(
					read_buf_start.add(initialized),
					0,
					read_buf_len - initialized,
				);
			}

			// The entire read buffer is now initialized, so we can create a
			// mutable slice of it.
			let read_buf = unsafe { core::slice::from_raw_parts_mut(read_buf_start, read_buf_len) };

			match r.read(read_buf) {
				Ok(read) => {
					// We filled `read` additional bytes.
					unsafe {
						self.set_len(self.len() + read);
					}
					initialized = read_buf_len - read;

					if read == 0 {
						return Ok(self.len() - start_len);
					}
				}
				Err(e) if e.kind() == ErrorKind::Interrupted => continue,
				Err(e) => return Err(e),
			}

			if self.len() == self.capacity() && self.capacity() == start_cap {
				// The buffer might be an exact fit. Let's read into a probe buffer
				// and see if it returns `Ok(0)`. If so, we've avoided an
				// unnecessary doubling of the capacity. But if not, append the
				// probe buffer to the primary buffer and let its capacity grow.
				let mut probe = [0u8; 32];

				loop {
					match r.read(&mut probe) {
						Ok(0) => return Ok(self.len() - start_len),
						Ok(n) => {
							self.extend_from_slice(&probe[..n]);
							break;
						}
						Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
						Err(e) => return Err(e),
					}
				}
			}
		}
	}
}

impl<const A: usize> From<AlignedByteVec<A>> for Vec<u8> {
	#[inline]
	fn from(aligned: AlignedByteVec<A>) -> Self {
		aligned.to_vec()
	}
}

impl<const A: usize> AsMut<[u8]> for AlignedByteVec<A> {
	#[inline]
	fn as_mut(&mut self) -> &mut [u8] {
		self.as_mut_slice()
	}
}

impl<const A: usize> AsRef<[u8]> for AlignedByteVec<A> {
	#[inline]
	fn as_ref(&self) -> &[u8] {
		self.as_slice()
	}
}

impl<const A: usize> Borrow<[u8]> for AlignedByteVec<A> {
	#[inline]
	fn borrow(&self) -> &[u8] {
		self.as_slice()
	}
}

impl<const A: usize> BorrowMut<[u8]> for AlignedByteVec<A> {
	#[inline]
	fn borrow_mut(&mut self) -> &mut [u8] {
		self.as_mut_slice()
	}
}

impl<const A: usize> Clone for AlignedByteVec<A> {
	#[inline]
	fn clone(&self) -> Self {
		unsafe {
			let mut result = Self::with_capacity(self.len);
			result.len = self.len;
			core::ptr::copy_nonoverlapping(self.as_ptr(), result.as_mut_ptr(), self.len);
			result
		}
	}
}

impl<const A: usize> fmt::Debug for AlignedByteVec<A> {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.as_slice().fmt(f)
	}
}

impl<const A: usize> Default for AlignedByteVec<A> {
	#[inline]
	fn default() -> Self {
		Self::new()
	}
}

impl<const A: usize> Deref for AlignedByteVec<A> {
	type Target = [u8];

	#[inline]
	fn deref(&self) -> &Self::Target {
		self.as_slice()
	}
}

impl<const A: usize> DerefMut for AlignedByteVec<A> {
	#[inline]
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.as_mut_slice()
	}
}

impl<const A: usize, I: slice::SliceIndex<[u8]>> Index<I> for AlignedByteVec<A> {
	type Output = <I as slice::SliceIndex<[u8]>>::Output;

	#[inline]
	fn index(&self, index: I) -> &Self::Output {
		&self.as_slice()[index]
	}
}

impl<const A: usize, I: slice::SliceIndex<[u8]>> IndexMut<I> for AlignedByteVec<A> {
	#[inline]
	fn index_mut(&mut self, index: I) -> &mut Self::Output {
		&mut self.as_mut_slice()[index]
	}
}

impl<const A: usize> io::Write for AlignedByteVec<A> {
	#[inline]
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		self.extend_from_slice(buf);
		Ok(buf.len())
	}

	#[inline]
	fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
		let len = bufs.iter().map(|b| b.len()).sum();
		self.reserve(len);
		for buf in bufs {
			self.extend_from_slice(buf);
		}
		Ok(len)
	}

	#[inline]
	fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
		self.extend_from_slice(buf);
		Ok(())
	}

	fn flush(&mut self) -> io::Result<()> {
		Ok(())
	}
}

// SAFETY: AlignedVec is safe to send to another thread
unsafe impl<const A: usize> Send for AlignedByteVec<A> {}

// SAFETY: AlignedVec is safe to share between threads
unsafe impl<const A: usize> Sync for AlignedByteVec<A> {}

impl<const A: usize> Unpin for AlignedByteVec<A> {}

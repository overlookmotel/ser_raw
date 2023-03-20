/// Mapping from input address (i.e. memory address of value being serialized)
/// and output position (i.e. position of that value's representation in
/// serializer's output).
#[derive(Copy, Clone, Debug)]
pub struct PosMapping {
	input_addr: usize,
	output_pos: usize,
}

// TODO: Rename `new` method so `dummy` can be called `new`?
impl PosMapping {
	/// Create new position mapping.
	#[inline]
	pub fn new(input_addr: usize, output_pos: usize) -> Self {
		Self {
			input_addr,
			output_pos,
		}
	}

	/// Create dummy position mapping.
	#[inline]
	pub fn dummy() -> Self {
		Self {
			input_addr: 0,
			output_pos: 0,
		}
	}

	/// Get position in output for a value which has been serialized.
	/// That value must have been serialized in an allocation which this
	/// `PosMapping` represents the start of.
	#[inline]
	pub fn pos_for_addr(&self, addr: usize) -> usize {
		addr - self.input_addr + self.output_pos
	}

	/// Get position in output for a value which has been serialized.
	/// That value must have been serialized in an allocation which this
	/// `PosMapping` represents the start of.
	#[inline]
	pub fn pos_for<T>(&self, value: &T) -> usize {
		self.pos_for_addr(value as *const T as usize)
	}
}

/// Trait for types which record memory addresses of values being serialized.
///
/// Reason why this is a trait, rather than a single type, is that not all
/// serializers need to know output positions.
///
/// Such serializers can use `NoopPos` which is a zero-size type with all
/// methods defined as no-ops. This should cause compiler to optimize out all
/// code related to calculating position. Therefore `Serialize` implementations
/// can include such code, and it has zero cost if it's not actually used.
///
/// The compiler doesn't seem to recognize it can make that optimization without
/// this abstraction.
pub trait Addr: Copy {
	/// Create `Addr` from a value reference.
	fn from_ref<T>(value: &T) -> Self;

	/// Create `Addr` from a value reference and offset.
	fn from_ref_offset<T>(value: &T, offset: usize) -> Self;

	/// Get address of `Addr` as `usize`.
	fn addr(&self) -> usize;
}

/// An `Addr` which does record addresses.
#[derive(Copy, Clone)]
pub struct TrackingAddr {
	addr: usize,
}

impl Addr for TrackingAddr {
	/// Create `TrackingAddr` from a value reference.
	#[inline]
	fn from_ref<T>(value: &T) -> Self {
		Self {
			addr: value as *const T as usize,
		}
	}

	/// Create `TrackingAddr` from a value reference and offset.
	#[inline]
	fn from_ref_offset<T>(value: &T, offset: usize) -> Self {
		Self {
			addr: value as *const T as usize + offset,
		}
	}

	/// Get address of `TrackingAddr` as `usize`.
	#[inline]
	fn addr(&self) -> usize {
		self.addr
	}
}

/// A dummy no-op `Addr` which stores no information
/// and for which all methods are no-ops.
///
/// See `Addr` trait for explanation of why this is useful.
#[derive(Copy, Clone)]
pub struct NoopAddr;

impl Addr for NoopAddr {
	/// Create `NoopAddr` from a value reference.
	#[inline(always)]
	fn from_ref<T>(_value: &T) -> Self {
		Self
	}

	/// Create `NoopAddr` from a value reference and offset.
	#[inline(always)]
	fn from_ref_offset<T>(_value: &T, _offset: usize) -> Self {
		Self
	}

	/// Get address of `NoopAddr` as `usize`.
	#[inline(always)]
	fn addr(&self) -> usize {
		// TODO: Replace this with a compile-time error.
		unreachable!();
	}
}

/// A record of pointers written to storage which may require correction if
/// storage grows during serialization and its memory address changes.
///
/// `current` is the group of of pointers currently in use.
/// `past` is previous groups.
/// Each time a change in memory address for the storage buffer is detected,
/// `current` is added to `past` and a fresh `current` is created.
pub struct Ptrs {
	pub current: PtrGroup,
	pub past: Vec<PtrGroup>,
}

impl Ptrs {
	pub fn new() -> Ptrs {
		Ptrs {
			current: PtrGroup::dummy(),
			past: Vec::new(),
		}
	}
}

/// A group of pointers which were written to storage when the memory address of
/// the storage was `storage_addr`.
/// Used for correcting pointers if the storage grows during serialization and
/// its memory address changes.
// TODO: Use `u32` for ptr positions if `MAX_CAPACITY` is less than `u32::MAX`
pub struct PtrGroup {
	/// Memory address of the storage at time pointers in this group were created
	storage_addr: usize,
	/// Positions of pointers in storage (relative to start of storage)
	ptr_positions: Vec<usize>,
}

impl PtrGroup {
	#[inline]
	pub fn new(storage_addr: usize) -> Self {
		Self {
			storage_addr,
			// TODO: Maybe replace with `with_capacity(32)` or similar to avoid repeated growing?
			ptr_positions: Vec::new(),
		}
	}

	#[inline]
	pub fn dummy() -> Self {
		Self::new(0)
	}

	#[inline]
	pub fn is_empty(&self) -> bool {
		self.ptr_positions.len() == 0
	}

	#[inline]
	pub fn addr(&self) -> usize {
		self.storage_addr
	}

	#[inline]
	pub fn set_addr(&mut self, storage_addr: usize) {
		self.storage_addr = storage_addr;
	}

	#[inline]
	pub fn push_pos(&mut self, pos: usize) {
		self.ptr_positions.push(pos);
	}

	/// Correct pointers in storage.
	///
	/// # Safety
	///
	/// All `ptr_positions` must be within the bounds of the `Storage` pointed to
	/// by `storage_ptr`.
	pub unsafe fn correct_ptrs(&self, storage_ptr: *mut u8) {
		// These pointers were written when start of storage was at
		// `ptr_group.storage_addr`. Now it's at `storage_addr`.
		// Shift pointers' target addresses forward or backwards as required so they
		// point to targets' current memory addresses.
		// Using `wrapping_*` for correct maths for all possible old + new addresses,
		// regardless of whether new addr is less than or greater than old addr.
		// No need to cast to `isize` to handle negative shift.
		// e.g. `old = 4`, `new = 10` -> `shift_by = 6` -> each ptr has 6 added.
		let shift_by = (storage_ptr as usize).wrapping_sub(self.storage_addr);
		for ptr_pos in &self.ptr_positions {
			// TODO: Use `storage.read()` and `storage.write()` instead of this
			let ptr = storage_ptr.add(*ptr_pos) as *mut usize;
			*ptr = (*ptr).wrapping_add(shift_by);
		}
	}
}

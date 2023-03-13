#![allow(dead_code)] // TODO: Remove this

/// Mapping from input address (i.e. memory address of value being serialized)
/// and output position (i.e. position of that value's representation in
/// serializer's output).
#[derive(Copy, Clone, Debug)]
pub struct PosMapping {
	input_addr: usize,
	output_pos: usize,
}

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
		unreachable!();
	}
}

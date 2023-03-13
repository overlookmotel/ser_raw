#![allow(dead_code)]

use crate::Serializer;

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
	pub fn pos_for<T>(&self, value: &T) -> usize {
		(value as *const T as usize) - self.input_addr + self.output_pos
	}
}

/// Trait for serializers which track position in output.
///
/// Implement the trait on a serializer, and then use macro
/// `impl_pos_tracking_serializer!()` to implement `Serialize`.
///
/// `serialize_data` functions can then get the position of any value's
/// serialized representation in output with `serializer.pos_for(&value)`.
///
/// # Example
///
/// ```
/// use ser_raw::{impl_pos_tracking_serializer, PosTrackingSerializer, SerializerStorage};
///
/// struct MySerializer {}
///
/// impl PosTrackingSerializer for MySerializer {}
/// impl_pos_tracking_serializer!(MySerializer);
///
/// impl SerializerStorage for MySerializer {
/// 	// ...
/// }
/// ```
pub trait PosTrackingSerializer: Serializer {
	/// Get current position mapping
	fn pos_mapping(&self) -> &PosMapping;

	/// Set current position mapping
	fn set_pos_mapping(&mut self, pos: PosMapping) -> ();

	/// Get position for a value
	#[inline]
	fn pos_for<T>(&self, value: &T) -> usize {
		self.pos_mapping().pos_for(value)
	}
}

/// Macro to create `Serializer` implementation for serializers implementing
/// `PosTrackingSerializer`.
///
/// See `impl_serializer` for syntax rules.
#[macro_export]
macro_rules! impl_pos_tracking_serializer {
	($($type_def:tt)*) => {
		$crate::impl_serializer!(
			PosTrackingSerializer,
			{
				fn serialize_value<T: Serialize<Self>>(&mut self, value: &T) {
					// Align storage, ready to write value
					self.storage_mut().align_for::<T>();

					// Record position mapping for this value
					self.set_pos_mapping(PosMapping::new(value as *const T as usize, self.pos()));

					// Push value to storage.
					// `push_slice_unaligned`'s requirements are satisfied by `align_for::<T>()` and
					// `align_after::<T>()`.
					let slice = ::std::slice::from_ref(value);
					unsafe { self.storage_mut().push_slice_unaligned(slice) };
					self.storage_mut().align_after::<T>();

					// Serialize value (which may use the pos mapping we set)
					value.serialize_data(self);
				}

				// Skip recording position when no further processing for a slice
				#[inline]
				fn push_slice<T>(&mut self, slice: &[T]) {
					self.push_raw_slice(slice);
				}

				#[inline]
				fn push_and_process_slice<T, P: FnOnce(&mut Self)>(&mut self, slice: &[T], process: P) {
					// Get position mapping before processing this
					let pos_mapping_before = *self.pos_mapping();

					// Align storage, ready to write slice
					self.storage_mut().align_for::<T>();

					// Record position mapping for this slice
					self.set_pos_mapping(PosMapping::new(slice.as_ptr() as usize, self.pos()));

					// Push slice to storage.
					// `push_slice_unaligned`'s requirements are satisfied by `align_for::<T>()` and
					// `align_after::<T>()`.
					unsafe { self.storage_mut().push_slice_unaligned(slice) };
					self.storage_mut().align_after::<T>();

					// Call `process` function (which may use the pos mapping we set)
					process(self);

					// Reset position mapping back to as it was
					self.set_pos_mapping(pos_mapping_before);
				}
			},
			$($type_def)*
		);
	};
}

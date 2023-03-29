use crate::Serializer;

/// Trait for types which can be serialized.
///
/// Usually implemented with the derive macro.
///
/// # With derive macro
///
/// ```
/// use ser_raw::Serialize;
///
/// #[derive(Serialize)]
/// struct Foo {
/// 	smalls: Vec<u8>,
/// 	bigs: Vec<u32>,
/// }
/// ```
///
/// # Manual implementation
///
/// [`Serialize`] has only one method: [`serialize_data`].
///
/// [`Serialize`] implementations should be written to work with all the
/// different types of [`Serializer`].
///
/// `ser_raw` is structured so that [`serialize_data`] can use features which
/// support all possible serializers, but if the serializer is a simpler type
/// which doesn't require those features, the compiler will be able to optimize
/// out that code. So e.g. [`overwrite_with`] can be called, and if the
/// serializer isn't interested in corrections, it's a no-op. So it's zero cost
/// unless it's used, and the speed of the fastest serializers e.g.
/// [`PureCopySerializer`] is unaffected by the more complex code required to
/// support the more complex serializers.
///
/// [`serialize_data`] does **not** serialize the value itself, but only any
/// data which the value owns *outside of it's own memory allocation*. e.g.:
///
/// ```
/// struct Foo {
/// 	// Contained within the type itself.
/// 	// No need for `serialize_data()` to do anything for these fields.
/// 	num: u32,
/// 	yes_or_no: bool,
/// 	ip: [u8; 4],
/// 	opt: Option<i8>,
///
/// 	// Contain pointers to data outside of this type's memory allocation.
/// 	// `serialize_data()` needs to handle serializing the external data for these fields.
/// 	name: String,
/// 	nums: Vec<u64>,
/// 	parent: Option<Box<Foo>>,
/// }
/// ```
///
/// If your type contains external data, you need to carefully consider which
/// [`Serializer`] methods are the right ones to call, in order to ensure your
/// [`Serialize`] implementation supports all the different types of
/// [`Serializer`]. Any of the following methods may be appropriate:
///
/// * [`push`](Serializer::push),
/// * [`push_slice`](Serializer::push_slice)
/// * [`push_and_process`](Serializer::push_and_process)
/// * [`push_and_process_slice`](Serializer::push_and_process_slice)
/// * [`push_raw`](Serializer::push_raw)
/// * [`push_raw_slice`](Serializer::push_raw_slice)
///
/// You may also need to wrap those calls in [`overwrite_with`].
///
/// Look at the [`Serialize` implementation for `Box` and `Vec`] for a better
/// understanding of what these methods do.
///
/// # Example
///
/// ```
/// use ser_raw::{Serialize, Serializer};
///
/// struct Foo {
/// 	smalls: Vec<u8>,
/// 	bigs: Vec<u32>,
/// }
///
/// impl<S> Serialize<S> for Foo
/// where S: Serializer
/// {
/// 	fn serialize_data(&self, serializer: &mut S) {
/// 		self.smalls.serialize_data(serializer);
/// 		self.bigs.serialize_data(serializer);
/// 	}
/// }
/// ```
///
/// [`serialize_data`]: Serialize::serialize_data
/// [`overwrite_with`]: Serializer::overwrite_with
/// [`Serialize` implementation for `Box` and `Vec`]:
/// https://docs.rs/ser_raw/latest/src/ser_raw/serialize_impls/ptrs.rs.html
/// [`PureCopySerializer`]: crate::PureCopySerializer
pub trait Serialize<Ser: Serializer> {
	/// Serialize data owned by this value, outside value's own memory allocation.
	///
	/// See [`Serialize`] trait for more details.
	fn serialize_data(&self, serializer: &mut Ser) -> ();
}

/// Trait for implementing an equivalent of [`Serialize`] on foreign types for
/// which it's not possible to implement [`Serialize`] directly due to orphan
/// rules. Use with `#[ser_with]`.
///
/// # Example
///
/// ```
/// use ser_raw::{Serialize, Serializer, SerializeWith, pos::Addr};
///
/// // The foreign type we want to be able to serialize
/// use num_bigint::BigUint;
///
/// // Our own type which contains the foreign type
/// #[derive(Serialize)]
/// struct Foo {
/// 	#[ser_with(BigUintProxy)]
/// 	big: BigUint,
/// }
///
/// struct BigUintProxy;
/// impl<S> SerializeWith<BigUint, S> for BigUintProxy
/// where S: Serializer
/// {
/// 	fn serialize_data_with(biguint: &BigUint, serializer: &mut S) {
/// 		let bytes = biguint.to_bytes_le();
/// 		// Actually next line isn't quite right.
/// 		// We need to offset `Addr` to the position of pointer within `BigUint`.
/// 		let ptr_addr = S::Addr::from_ref(biguint);
/// 		serializer.push(&bytes.len(), ptr_addr);
/// 		serializer.push_raw_bytes(bytes.as_slice());
/// 	}
/// }
/// ```
pub trait SerializeWith<T, Ser: Serializer> {
	/// Serialize data owned by this value, outside value's own memory allocation.
	///
	/// See [`SerializeWith`] trait for more details.
	fn serialize_data_with(value: &T, serializer: &mut Ser) -> ();
}

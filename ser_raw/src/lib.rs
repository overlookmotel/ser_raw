//! # ser_raw
//!
//! `ser_raw` is a simple and fast serializer.
//!
//! It uses Rust's native memory layouts as the serialization format, so
//! serializing is largely as simple as just copying raw bytes. This offers two
//! main advantages:
//!
//! 1. The simplicity means it's very fast.
//! 2. Deserialization can be zero-copy and instantaneous - just cast a pointer
//! to the serialized data into a `&T`.
//!
//! The primary target for this library is sharing data between different
//! processes on the same machine (including between different languages), but
//! it can also be used for other purposes.
//!
//! # How fast is it?
//!
//! Fast! Benchmark of serializing a large AST structure produced by [SWC]:
//!
//! ```txt
//! serde_json: 226.99 µs
//! rkyv:        44.98 µs
//! ser_raw:     14.35 µs
//! ```
//!
//! # Serializers
//!
//! This crate provides 3 different serializers for different use cases. They
//! offer a range of options, between doing work during serialization, or during
//! deserialization. They mostly differ in how they deal with pointers.
//!
//! [`PureCopySerializer`] is the fastest and simplest serializer. Does not
//! correct pointers, so the data can only be deserialized by traversing the
//! tree of values in order.
//!
//! [`PtrOffsetSerializer`] replaces pointers in the input (e.g. in `Box`, `Vec`
//! or `String`) with offsets. i.e. what byte index the pointee is located in
//! the output.
//!
//! This allows lazy deserialization, and for a deserializer to traverse the
//! tree of values in any order/direction.
//!
//! [`CompleteSerializer`] replaces pointers in the input with valid pointers
//! into the output, and makes other corrections to ensure output is a
//! completely valid representation of the input. Input can be "rehydrated" just
//! by casting a pointer to the start of the output buffer as a `&T`.
//!
//! # Custom serializers
//!
//! This crate provides an easy-to-use [derive
//! macro](ser_raw_derive_serializer::Serializer) to create custom
//! [`Serializer`]s, based on any of the above.
//!
//! Serializers can also choose between different backing storage options.
//! This crate provides only one at present - [`AlignedVec`] - but it's
//! possible to create your own [`Storage`] implementation.
//!
//! # Serializable types
//!
//! Only owned, sized types are supported at present.
//!
//! Support for serializing common Rust types (e.g. `u8`, `isize`, `NonZeroU32`,
//! `Box`, `Vec`, `String`, `Option`) is included out of the box.
//!
//! For your own types, implement the [`Serialize`] trait. Usually, you can use
//! the [derive macro](ser_raw_derive::Serialize).
//!
//! ```
//! use ser_raw::Serialize;
//!
//! #[derive(Serialize)]
//! struct Foo {
//! 	small: u8,
//! 	vec: Vec<u32>
//! }
//! ```
//!
//! For foreign types (i.e. from external crates), use [`SerializeWith`].
//!
//! # Deserializing
//!
//! No deserializers are provided at present.
//!
//! [`CompleteSerializer`] doesn't require a deserializer anyway, as you can
//! just cast a pointer to the output buffer to a `&T`.
//!
//! # Warning
//!
//! As serializers just copy Rust's memory verbatim, a serializer's output
//! will depend on the system it's run on (processor architecture, big endian or
//! little endian, 64 bit or 32 bit).
//!
//! Rust also offers no guarantee that even the same code compiled twice on the
//! same system will result in the same memory layouts (in practice it does, but
//! you can always tag your types `#[repr(C)]` to make sure).
//!
//! Therefore, great care should be taken to ensure deserialization occurs on
//! same type of machine as serialization occured on, and ideally using the same
//! binary. A mismatch will be very likely to cause memory unsafety and the
//! dreaded *undefined behavior*.
//!
//!	For the primary use case for `ser_raw` - transfer of data within a single
//! system - these constraints are not a problem.
//!
//! # Features
//!
//! `derive` feature enables the [`Serialize`] derive macro. Enabled by default.
//!
//! `num_bigint` feature enables serialization of [`num-bigint`]'s [`BigInt`]
//! and [`BigUint`] types.
//!
//! # Future direction and motivation
//!
//! The primary motivator for creating this library is to enable fast sharing of
//! data between Rust and JavaScript (via [napi-rs]). The classic approach of
//! using [serde JSON] is [much too slow] for some use cases, and [rkyv] turned
//! out also to be slower than expected.
//!
//! The idea is to use [layout_inspect] to produce a schema of Rust's type
//! layouts, and write a codegen which uses that schema to generate a JavaScript
//! serializer / deserializer which can deserialize `ser_raw`'s output.
//!
//! This is the main reason why there aren't deserializers implemented in Rust
//! yet! I'm planning to be doing the deserialization in JavaScript.
//!
//! # Credits
//!
//! `ser_raw` follows the same approach to serialization as [abomonation]. It
//! matches abomonation's extremely fast speed, while aiming to avoid its safety
//! issues (and halloween theme!)
//!
//! [rkyv] is also an inspiration, and the backing storage used by most of
//! `ser_raw`'s serializers, [`AlignedVec`], is based on [rkyv]'s type of
//! the same name.
//!
//! [`AlignedVec`]: storage::AlignedVec
//! [`Storage`]: storage::Storage
//! [SWC]: https://swc.rs/
//! [napi-rs]: https://napi.rs/
//! [serde JSON]: https://serde.rs/
//! [much too slow]: https://github.com/swc-project/swc/issues/2175
//! [layout_inspect]: https://github.com/overlookmotel/layout_inspect
//! [abomonation]: https://github.com/TimelyDataflow/abomonation
//! [rkyv]: https://rkyv.org/
//! [`num-bigint`]: https://crates.io/crates/num-bigint
//! [`BigInt`]: https://docs.rs/num-bigint/latest/num_bigint/struct.BigInt.html
//! [`BigUint`]: https://docs.rs/num-bigint/latest/num_bigint/struct.BigUint.html

// Derive macros
#[cfg(feature = "derive")]
pub use ser_raw_derive::Serialize;
pub use ser_raw_derive_serializer::Serializer;

// Export Serializers, Storage, traits, and utils
mod serializer;
pub use serializer::Serializer;

mod serializers;
pub use serializers::{CompleteSerializer, PtrOffsetSerializer, PureCopySerializer};

mod serializer_traits;
pub mod ser_traits {
	//! Traits which are composed to create Serializers. Used internally by
	//! [`Serializer`](crate::Serializer) derive macro.
	pub use super::serializer_traits::*;
}

mod serialize;
pub use serialize::{Serialize, SerializeWith};

pub mod pos;
pub mod storage;
pub mod util;

// `Serialize` implementations for Rust internal types
mod serialize_impls;

// Derive macro for `Serialize`
#[cfg(feature = "derive")]
pub use ser_raw_derive::Serialize;

// Export Serializers, Storage, traits, and utils
mod serializer;
pub use serializer::{BorrowingSerializer, InstantiableSerializer, Serializer, SerializerStorage};

mod serializers;
pub use serializers::PureCopySerializer;

mod serializer_impls;
pub use serializer_impls::{AlignedSerializer, UnalignedSerializer};

mod serialize;
pub use serialize::{Serialize, SerializeWith};

pub mod storage;
pub mod util;

// `Serialize` implementations for Rust internal types
mod serialize_impls;

// Macros for use within crate
mod macros;

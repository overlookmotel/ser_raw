// Derive macro for `Serialize`
#[cfg(feature = "derive")]
pub use ser_raw_derive::Serialize;

// Export Serializers, Storage, traits, and utils
mod serializer;
pub use serializer::{BorrowingSerializer, InstantiableSerializer, Serializer};

mod serializers;
pub use serializers::{AlignedSerializer, UnalignedSerializer};

mod serialize;
pub use serialize::{Serialize, SerializeWith};

pub mod storage;
pub mod util;

// `Serialize` implementations for Rust internal types
mod impls;

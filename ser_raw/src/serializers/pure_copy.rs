use crate::Serializer;

/// Trait for the simplest serializers which purely copy types.
///
/// # Example
///
/// This is a simplified version of the `AlignedSerializer` type provided by
/// this crate:
///
/// ```
/// use ser_raw::{
/// 	pos::NoopAddr,
/// 	storage::{aligned_max_capacity, AlignedVec},
/// 	PureCopySerializer, Serializer,
/// };
///
/// const MAX_CAPACITY: usize = aligned_max_capacity(16);
/// type Store = AlignedVec<16, 8, 16, MAX_CAPACITY>;
///
/// struct MySerializer {
/// 	storage: Store,
/// }
///
/// impl Serializer for MySerializer {
/// 	type Storage = Store;
/// 	type BorrowedStorage = Store;
/// 	type Addr = NoopAddr;
///
/// 	fn storage(&self) -> &Store { &self.storage }
/// 	fn storage_mut(&mut self) -> &mut Store { &mut self.storage }
/// 	fn into_storage(self) -> Store { self.storage }
/// }
///
/// impl PureCopySerializer for MySerializer {}
/// ```
pub trait PureCopySerializer: Serializer {
	// NB: Pure copy serializers can use `NoopAddr` as `Addr` associated type.
}

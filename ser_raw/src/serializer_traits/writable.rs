use crate::{pos::ActiveAddr, ser_traits::PosTracking, storage::RandomAccessStorage};

/// Trait for serializers which can write at arbitrary positions in output.
pub trait Writable: PosTracking
where
	Self::Storage: RandomAccessStorage,
	Self::Addr: ActiveAddr,
{
	#[inline]
	unsafe fn do_overwrite<T>(&mut self, addr: Self::Addr, value: &T) {
		let pos = self.pos_mapping().pos_for_addr(addr);
		self.storage_mut().write(pos, value);
	}
}

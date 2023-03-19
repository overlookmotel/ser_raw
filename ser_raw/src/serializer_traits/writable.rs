use crate::{ser_traits::PosTracking, storage::ContiguousStorage};

/// Trait for serializers which can write at arbitrary positions in output.
pub trait Writable: PosTracking
where Self::Storage: ContiguousStorage
{
	#[inline]
	unsafe fn do_write<T>(&mut self, value: &T, addr: usize) {
		let pos = self.pos_mapping().pos_for_addr(addr);
		self.storage_mut().write(value, pos);
	}
}

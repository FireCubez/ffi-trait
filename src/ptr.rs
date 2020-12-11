use core::mem;

use core::ptr;
use core::ptr::NonNull;

use crate::{FFITrait, FFIDynRef, FFIDynMut};

/// An FFI-safe equivalent of `*mut dyn T`
///
/// # Note
/// Unlike references or boxes, there is no `from_std` function
/// for this type. You may create a reference and convert it back into
/// this type.
#[derive(Debug)]
#[repr(C)]
pub struct FFIDynPtr<T: FFITrait + ?Sized> {
	pub data: *mut (),
	pub vtable: NonNull<T::Vtable>
}

impl<T: FFITrait + ?Sized> Copy for FFIDynPtr<T> {}
impl<T: FFITrait + ?Sized> Clone for FFIDynPtr<T> {
	fn clone(&self) -> Self { *self }
}

/// An FFI-safe equivalent of `NonNull<dyn T>`
///
/// # Note
/// Unlike references or boxes, there is no `from_std` function
/// for this type. You may create a reference and convert it back into
/// this type.
#[derive(Debug)]
#[repr(C)]
pub struct FFIDynNonNull<T: FFITrait + ?Sized> {
	pub data: NonNull<()>,
	pub vtable: NonNull<T::Vtable>
}

impl<T: FFITrait + ?Sized> Copy for FFIDynNonNull<T> {}
impl<T: FFITrait + ?Sized> Clone for FFIDynNonNull<T> {
	fn clone(&self) -> Self { *self }
}

impl<T: FFITrait + ?Sized> FFIDynPtr<T> {
	pub fn from_raw_parts(data: *mut (), vtable: NonNull<T::Vtable>) -> Self {
		Self { data, vtable }
	}

	pub fn null() -> Self {
		Self { data: ptr::null_mut(), vtable: NonNull::dangling() }
	}

	pub fn is_null(self) -> bool { self.data.is_null() }

	pub unsafe fn as_ref<'a>(&self) -> &FFIDynRef<'a, T> { mem::transmute(self) }
	pub unsafe fn as_mut<'a>(&mut self) -> &mut FFIDynMut<'a, T> { mem::transmute(self) }

	pub unsafe fn to_ref<'a>(self) -> FFIDynRef<'a, T> { mem::transmute(self) }
	pub unsafe fn to_ref_mut<'a>(self) -> FFIDynMut<'a, T> { mem::transmute(self) }
}

impl<T: FFITrait + ?Sized> FFIDynNonNull<T> {
	pub fn from_raw_parts(data: NonNull<()>, vtable: NonNull<T::Vtable>) -> Self {
		Self { data, vtable }
	}

	pub fn new(x: FFIDynPtr<T>) -> Option<Self> {
		Some(Self::from_raw_parts(NonNull::new(x.data)?, x.vtable))
	}

	pub unsafe fn new_unchecked(x: FFIDynPtr<T>) -> Self { mem::transmute(x) }

	pub fn as_ptr(&mut self) -> &mut FFIDynPtr<T> { unsafe { mem::transmute(self) } }
	pub unsafe fn as_ref<'a>(&self) -> &FFIDynRef<'a, T> { mem::transmute(self) }
	pub unsafe fn as_mut<'a>(&mut self) -> &mut FFIDynMut<'a, T> { mem::transmute(self) }

	pub fn to_ptr(self) -> FFIDynPtr<T> { unsafe { mem::transmute(self) } }
	pub unsafe fn to_ref<'a>(self) -> FFIDynRef<'a, T> { mem::transmute(self) }
	pub unsafe fn to_ref_mut<'a>(self) -> FFIDynMut<'a, T> { mem::transmute(self) }
}
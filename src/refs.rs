use core::mem;
use core::ptr::NonNull;
use core::marker::PhantomData;

use crate::{FFITrait, FFIDynPtr, FFIDynNonNull, IntoTraitObjectRuntime};

/// An FFI-safe equivalent of `&dyn T`
#[repr(transparent)]
pub struct FFIDynRef<'a, T: FFITrait + ?Sized>(FFIDynNonNull<T>, PhantomData<&'a T>);

impl<T: FFITrait + ?Sized> Copy for FFIDynRef<'_, T> {}
impl<T: FFITrait + ?Sized> Clone for FFIDynRef<'_, T> {
	fn clone(&self) -> Self { *self }
}

/// An FFI-safe equivalent of `&mut dyn T`
#[repr(transparent)]
pub struct FFIDynMut<'a, T: FFITrait + ?Sized>(FFIDynNonNull<T>, PhantomData<&'a mut T>);

impl<'a, T: FFITrait + IntoTraitObjectRuntime<T> + ?Sized> From<&'a T> for FFIDynRef<'a, T> {
	fn from(x: &'a T) -> Self {
		FFIDynRef::from_std(x)
	}
}

impl<'a, T: FFITrait + ?Sized> FFIDynRef<'a, T> {

	pub fn from_std(x: &'a T) -> Self where T: IntoTraitObjectRuntime<T> {
		x.dyn_ref()
	}

	/// Creates an `FFIDynRef` from an `FFIDynPtr`.
	///
	/// # Standard Equivalent
	/// This function is equivalent to `*mut dyn T -> &dyn T`
	pub unsafe fn from_ptr(x: FFIDynPtr<T>) -> Self {
		Self::from_nonnull(FFIDynNonNull::new_unchecked(x))
	}

	pub unsafe fn from_nonnull(x: FFIDynNonNull<T>) -> Self {
		Self(x, PhantomData)
	}

	pub unsafe fn from_raw_parts(data: NonNull<()>, vtable: NonNull<T::Vtable>) -> Self {
		Self::from_nonnull(FFIDynNonNull::from_raw_parts(data, vtable))
	}

	pub fn as_ptr(&mut self) -> &mut FFIDynPtr<T> { unsafe { mem::transmute(self) } }

	pub fn to_ptr(self) -> FFIDynPtr<T> { unsafe { mem::transmute(self) } }
}

impl<'a, T: FFITrait + ?Sized> FFIDynMut<'a, T> {

	pub fn from_std(x: &'a mut T) -> Self where T: IntoTraitObjectRuntime<T> {
		x.dyn_mut()
	}

	/// Creates an `FFIDynMut` from an `FFIDynPtr`.
	///
	/// # Standard Equivalent
	/// This function is equivalent to `*mut dyn T -> &mut dyn T`
	pub unsafe fn from_ptr(x: FFIDynPtr<T>) -> Self {
		Self::from_nonnull(FFIDynNonNull::new_unchecked(x))
	}

	pub unsafe fn from_nonnull(x: FFIDynNonNull<T>) -> Self {
		Self(x, PhantomData)
	}

	pub unsafe fn from_raw_parts(data: NonNull<()>, vtable: NonNull<T::Vtable>) -> Self {
		Self::from_nonnull(FFIDynNonNull::from_raw_parts(data, vtable))
	}

	pub fn as_ptr(&mut self) -> &mut FFIDynPtr<T> { unsafe { mem::transmute(self) } }

	pub fn to_ptr(self) -> FFIDynPtr<T> { unsafe { mem::transmute(self) } }
}
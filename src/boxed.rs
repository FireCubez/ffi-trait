extern crate alloc;

use alloc::boxed::Box;

use core::ops::{Deref, DerefMut};
use core::mem::ManuallyDrop;
use core::marker::PhantomData;

use crate::{FFITrait, FFIDynNonNull, FFIDynPtr, FFIDynRef, FFIDynMut, IntoTraitObjectRuntime};

/// An FFI-safe equivalent of `Box<dyn T>`
#[repr(transparent)]
pub struct FFIDynBox<T: FFITrait + ?Sized>(FFIDynNonNull<T>, PhantomData<T>);

impl<T: FFITrait + IntoTraitObjectRuntime<T> + ?Sized> From<Box<T>> for FFIDynBox<T> {
	fn from(x: Box<T>) -> Self {
		FFIDynBox::from_std(x)
	}
}

impl<'a, T: FFITrait + ?Sized> Deref for FFIDynBox<T> {
	type Target = FFIDynRef<'a, T>;
	fn deref(&self) -> &FFIDynRef<'a, T> {
		
	}

}

impl<T: FFITrait + ?Sized> FFIDynBox<T> {
	pub fn from_std(x: Box<T>) -> Self where T: IntoTraitObjectRuntime<T> {
		unsafe {
			Self::from_raw_std(Box::into_raw(x))
		}
	}

	pub unsafe fn from_raw_std(x: *mut T) -> Self where T: IntoTraitObjectRuntime<T> {
		Self::from_raw(FFIDynMut::from_std(&mut *x).to_ptr())
	}

	pub unsafe fn from_raw(x: FFIDynPtr<T>) -> Self {
		Self::from_nonnull(FFIDynNonNull::new_unchecked(x))
	}

	pub unsafe fn from_nonnull(x: FFIDynNonNull<T>) -> Self {
		Self(x, PhantomData)
	}

	pub fn leak<'a>(b: Self) -> FFIDynMut<'a, T> where T: 'a {
		unsafe {
			Self::into_raw(b).to_ref_mut()
		}
	}

	pub fn into_raw(b: Self) -> FFIDynPtr<T> {
		ManuallyDrop::new(b).0.to_ptr()
	}

	pub fn into_nonnull(b: Self) -> FFIDynNonNull<T> {
		ManuallyDrop::new(b).0
	}

}
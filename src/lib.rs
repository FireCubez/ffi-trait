#![feature(raw)]
#![cfg_attr(not(test), no_std)]

//! Alternative to the `#[stable_vtable]` attribute, which is far from being
//! implemented into the language.
//!
//! The `#[ffi_trait]` allows you to make traits which are FFI-safe with a
//! defined vtable and `&[mut] dyn Trait` and `Box<dyn Trait>` equivalents.

use core::mem;
use core::ptr::NonNull;
use core::raw::TraitObject;

pub use ffi_trait_macro::*;

pub mod refs;
pub mod ptr;
//#[cfg(feature = "boxed")] pub mod boxed;

pub use refs::*;
pub use ptr::*;
//#[cfg(feature = "boxed")] pub use boxed::*;

/// The layout of a generic vtable. All other vtables begin with
/// this layout. This property is guaranteed by the `GenericVtableLayout`
/// trait. This layout is equivalent to that of RFC 2955, making this
/// a drop-in replacement until said RFC is implemented.
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct GenericVtable {
	/// The size of the data
	pub size: usize,
	/// The alignment of the data
	pub align: usize,
	/// An optional function which drops the data
	pub drop_in_place: Option<unsafe extern "C" fn(*mut ())>,
	/// An optional function which deallocates the data, for use in e.x. `Box`.
	/// This allows other languages to deallocate the data.
	pub dealloc: Option<unsafe extern "C" fn(*mut ())>
}

impl<T: GenericVtableLayout> From<T> for GenericVtable {
	fn from(x: T) -> Self {
		unsafe { mem::transmute_copy(&x) }
	}
}

impl<T: GenericVtableLayout> AsRef<T> for GenericVtable {
	fn as_ref(&self) -> &T {
		unsafe { &*(self as *const Self as *const T) }
	}
}

impl<T: GenericVtableLayout> AsMut<T> for GenericVtable {
	fn as_mut(&mut self) -> &mut T {
		unsafe { &mut *(self as *mut Self as *mut T) }
	}
}

/// This marker specifies that it is safe to transmute this type,
/// truncating it if necessary, to a `GenericVtable`
pub unsafe trait GenericVtableLayout {}

pub fn generic<T: GenericVtableLayout>(v: &T) -> &GenericVtable {
	// SAFETY: T implements GenericVtableLayout which explicity allows this
	unsafe { mem::transmute(v) }
}

/// A trait for any trait which is an FFI-compatible trait.
///
/// Since making this trait a supertrait of another trait is
/// problematic, we instead use `dyn` to turn the trait into
/// an impl-able object.
pub trait FFITrait {
	type Vtable: GenericVtableLayout + Copy + Clone;
}

/// A trait for any type which can be turned into an FFI-compatible trait object
/// at **runtime**, unlike its subtrait `IntoTraitObject` which uses a constant
/// and isn't object safe.
pub trait IntoTraitObjectRuntime<T: FFITrait + ?Sized> {
	fn get_vt<'a>(&'a self) -> &'a T::Vtable;

	fn dyn_ref<'a>(&'a self) -> FFIDynRef<'a, T> {
		unsafe {
			FFIDynRef::from_raw_parts(
				NonNull::new_unchecked(self as *const Self as *const () as *mut ()),
				NonNull::new_unchecked(self.get_vt() as *const _ as *mut _)
			)
		}
	}

	fn dyn_mut<'a>(&'a mut self) -> FFIDynMut<'a, T> {
		unsafe {
			FFIDynMut::from_raw_parts(
				NonNull::new_unchecked(self as *mut Self as *mut ()),
				NonNull::new_unchecked(self.get_vt() as *const _ as *mut _)
			)
		}
	}
}

/// A trait for any type which can be turned into an FFI-compatible trait object
/// **using a constant** (i.e. at compile time), unlike its supertrait `IntoTraitObjectRuntime`
/// The type argument should be a `dyn` trait as explained in `FFITrait`
pub trait IntoTraitObject<T: FFITrait + ?Sized> : IntoTraitObjectRuntime<T> where T::Vtable: 'static {
	const VTABLE: &'static T::Vtable;
	fn get_vt(&self) -> &'static T::Vtable { &Self::VTABLE }
}

macro_rules! vtable_fn {
	($field:ident ($($tt:tt)*); $arg:ident => $e:expr => $ret:ty => $ptr:ident, $nn:ident, $ref:ident, $mut:ident) => {
		pub unsafe fn $ptr<T: FFITrait + ?Sized>($arg: FFIDynPtr<T>) -> $ret {
			$e
		}

		pub unsafe fn $nn<T: FFITrait + ?Sized>(x: FFIDynNonNull<T>) -> $ret {
			$ptr(x.to_ptr())
		}

		pub $($tt)* fn $ref<T: FFITrait + ?Sized>(x: FFIDynRef<'_, T>) -> $ret {
			#[allow(unused_unsafe)] unsafe { $ptr(x.to_ptr()) }
		}

		pub $($tt)* fn $mut<T: FFITrait + ?Sized>(x: FFIDynMut<'_, T>) -> $ret {
			#[allow(unused_unsafe)] unsafe { $ptr(x.to_ptr()) }
		}
	};
}

vtable_fn!(size (); x => generic(x.vtable.as_ref()).size => usize => size_of_val_ptr, size_of_val_nonnull, size_of_val_ref, size_of_val_mut);
vtable_fn!(align (); x => generic(x.vtable.as_ref()).align => usize => align_of_val_ptr, align_of_val_nonnull, align_of_val_ref, align_of_val_mut);
vtable_fn!(drop_in_place (unsafe); x => {
	let f = generic(x.vtable.as_ref()).drop_in_place;
	if let Some(f) = f { f(x.data); }
} => () => drop_in_place_ptr, drop_in_place_nonnull, drop_in_place_ref, drop_in_place_mut);
vtable_fn!(dealloc (unsafe); x => {
	let f = generic(x.vtable.as_ref()).dealloc;
	if let Some(f) = f { f(x.data); }
} => () => dealloc_ptr, dealloc_nonnull, dealloc_ref, dealloc_mut);

// used by the proc macro
#[doc(hidden)]
#[allow(non_snake_case)]
pub unsafe extern "C" fn __ffi_trait__raw_drop_in_place<T>(ptr: *mut ()) {
	core::ptr::drop_in_place(ptr as *mut T);
}

// oh lord please bring mercy

/// Please never ever call this function EVER. Please. Don't. EVER. Call this function.
/// This function is used by the autogenerated code from `#[ffi_trait]`. It takes
/// a data and vtable pointer and drops the trait object represented by them.
/// Using this function is worse than transmute and worse than transmute_copy.
///
/// For God's sake please never approach this function in your life.
/// Unless you want UB. Here's how to cause UB:
/// - Pass a null data pointer
/// - Pass a null vtable pointer
/// - Pass a vtable pointer which doesn't point at a valid vtable made by Rust
/// - Pass a data pointer which doesn't correspond with the vtable
/// - Pass a type argument which implements `Sized` (yes, seriously)
/// - Pass a type argument which isn't `dyn SomeTrait`
/// - Pass a `dyn Trait` which isn't the actual trait represented by the data/vtable
/// - Call this function twice with the same arguments
#[doc(hidden)]
#[allow(non_snake_case)]
pub unsafe extern "C" fn __ffi_trait__raw_dyn_drop_in_place<T: ?Sized>(data: *mut (), vtable: *mut ()) {
	// we can't use normal transmute here since we don't know that
	// T isn't sized (since we don't have negative bounds).
	//
	// When Rust literally prevents you from using TRANSMUTE, you know
	// something is WRONG.

	// use *mut T instead of &mut T so we dont have mutable aliasing
	let t: *mut T = mem::transmute_copy(&TraitObject { data, vtable });
	core::ptr::drop_in_place(t);
}

#[cfg(test)]
mod tests {
	#[test]
	fn dyn_drop_in_place() {
		unsafe {
			let p = Box::into_raw(Box::new(Box::new(8))); // something which has a destructor inside the box
			let mut x = Box::from_raw(p);
			let d: *mut dyn std::fmt::Display = &mut x;
			let std::raw::TraitObject { data, vtable } = std::mem::transmute(d);
			assert_eq!(data as *const _, &x as *const _ as *const _);
			std::mem::forget(x);
			crate::__ffi_trait__raw_dyn_drop_in_place::<dyn std::fmt::Display>(data, vtable);
			// from now on we can't access x at all, it's been dropped in place.
			//std::mem::forget(x); forgetting something after it's been dropped - can't do, so we do it above
		}
	}
}
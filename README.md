# ffi-trait

FFI-safe traits.

```rs

#[ffi_trait]
pub trait MyTrait {
    fn a(&self);
}

...

let x: FFIDynRef<dyn MyTrait> = ...;

```

# Notes
- This is an extreme PoC
- It probably works though
- The layout of the vtable is as discussed in [RFC 2955](https://github.com/rust-lang/rfcs/pull/2955)
- Boxes dont exist yet. Please PR
- Yes, I went insane making this
- All ffi_traits must be object safe
- `Self` in methods support is limited. Use erased raw pointers for now. Please PR

#![feature(proc_macro_def_site)]

#[macro_use] extern crate quote;

use syn::*; // note: std result gets shadowed

use proc_macro::Span;
use proc_macro::TokenStream;

use std::cell::RefCell;

fn abi_from_lit(x: LitStr) -> Abi {
	Abi {
		extern_token: token::Extern { span: Span::call_site().into() },
		name: Some(x)
	}
}

#[proc_macro_attribute]
pub fn ffi_trait(attr: TokenStream, item: TokenStream) -> TokenStream {
	let mut default_abi = Some(abi_from_lit(LitStr::new("C", Span::call_site().into())));
	// outer none = not specified
	// inner none = explicitly specified none
	let mut set_default_abi: Option<Option<Abi>> = None;

	let args = parse_macro_input!(attr as AttributeArgs);
	for arg in args {
		match arg {
			NestedMeta::Meta(m) => match m {
				Meta::NameValue(nv) if nv.path.is_ident("default_abi") => {
					if set_default_abi.is_some() {
						panic!("conflicting options for default abi");
					}
					set_default_abi = Some(Some(match nv.lit {
						Lit::Str(x) => abi_from_lit(x),
						_ => panic!("default abi must be a string literal")
					}));
				},
				Meta::Path(p) if p.is_ident("no_default_abi") => {
					if set_default_abi.is_some() {
						panic!("conflicting options for default abi");
					}
					set_default_abi = Some(None);
				},
				_ => panic!("invalid options to `ffi_trait`")
			},
			_ => panic!("invalid options to `ffi_trait`")
		}
	}

	if let Some(x) = set_default_abi {
		default_abi = x;
	}

	let def: ItemTrait = syn::parse(item).unwrap();

	let items = def.items.iter().map(|item| {
		match item {
			TraitItem::Method(x) => {
				if x.sig.constness.is_some() {
					panic!("`const` fns aren't supported in `ffi_trait`s");
				}
				if x.sig.asyncness.is_some() {
					panic!("`async` fns aren't supported in `ffi_trait`s");
				}
				let name = &x.sig.ident;
				let abi2 = &x.sig.abi;
				let unsafety = &x.sig.unsafety;
				let generics = &x.sig.generics;
				let inputs2 = &x.sig.inputs;
				let output = &x.sig.output;
				let block = &x.default;
				if generics.where_clause.is_some() {
					panic!("`where` clauses aren't supported in `ffi_trait`s");
				}

				let lts2 = generics.params.iter().inspect(|param| {
					match param {
						GenericParam::Lifetime(_) => {},
						_ => panic!("generics other than lifetimes aren't supported in `ffi_trait`s")
					}
				});

				let abi = abi2.as_ref().or(default_abi.as_ref());
				let receiver2: RefCell<Option<&Receiver>> = RefCell::new(None);
				let (inputs, t): (Vec<_>, Vec<_>) = inputs2.iter().enumerate().filter_map(|(index, arg)| match arg {
					FnArg::Receiver(x) => {
						*receiver2.borrow_mut() = Some(x);
						None
					},
					FnArg::Typed(x) => Some((x, (
						PatType {
							attrs: Vec::new(),
							pat: Box::new(Pat::Ident(PatIdent {
								attrs: Vec::new(),
								by_ref: None,
								mutability: None,
								ident: Ident::new(&format!("arg{}", index), Span::call_site().into()),
								subpat: None
							})),
							colon_token: token::Colon { spans: [Span::call_site().into()] },
							ty: {
								fn check_self(ty: &Type) {
									match ty {
										Type::Path(x) => {
											let s = &x.path.segments;
											if s.len() == 1 && s[0].ident == "Self" {
												panic!("`Self` arguments aren't supported in `ffi_trait`s. consider using erased pointers.");
											}
										},
										_ => {}
									}
								}
								check_self(&*x.ty);
								x.ty.clone()
							}
						},
						Ident::new(&format!("arg{}", index), Span::call_site().into())
					)))
				}).unzip();
				let (rawinputs, rawnames): (Vec<_>, Vec<_>) = t.into_iter().unzip();
				let receiver = match *receiver2.borrow() {
					Some(x) => x,
					_ => panic!("`ffi_trait` methods must be object safe (all functions must take a self param, but `{}` doesn't)", name)
				};

				let lts = lts2.collect::<Vec<_>>();
				let semicolon = if block.is_none() { quote!(;) } else { quote!() };

				let tname = &def.ident;
				let rawname = Ident::new(&format!("__ffi_trait__{}__raw_{}", tname, name), Span::def_site().into());
				let rmut = receiver.mutability;
				let rawmethod = if receiver.reference.is_some() {
					quote!(unsafe #abi fn #rawname< Impl: #tname, #(#lts),* >(this: ::core::ptr::NonNull<()>, #(#rawinputs),*) #output {
						<Impl as #tname>::#name(&#rmut *(this.as_ptr() as *mut Impl), #(#rawnames)*)
					})
				} else {
					panic!("`ffi_trait` methods cannot take `self` by value.");
				};

				let methodimpl = quote!(fn #name <#(#lts),*> (#receiver, #(#rawinputs),*) #output {
					unsafe {
						#rawname(::core::mem::transmute(#receiver.data), #(#rawnames),*)
					}
				});

				let methodimpli = if rmut.is_none() {
					Some(quote!(fn #name <#(#lts),*> (#receiver, #(#rawinputs),*) #output {
						unsafe {
							#rawname(::core::mem::transmute(#receiver.data), #(#rawnames),*)
						}
					}))
				} else { None };
				(
					// vtable field
					quote!(pub #name: for < #(#lts),* > unsafe #abi fn(::core::ptr::NonNull<()>, #(#rawinputs),*) #output),
					(
						// method
						quote!(#unsafety fn #name< #(#lts),* >(#receiver, #(#inputs),*) #output #block #semicolon),
						(rawmethod, (name, (rawname, (methodimpl, methodimpli))))
					)
				)
			},
			_ => panic!("Only methods are supported in `ffi_trait`s")
		}
	});
	let (fields, t): (Vec<_>, Vec<_>) = items.unzip();
	let (methods, t): (Vec<_>, Vec<_>) = t.into_iter().unzip();
	let (rawmethods, t): (Vec<_>, Vec<_>) = t.into_iter().unzip();
	let (methodnames, t): (Vec<_>, Vec<_>) = t.into_iter().unzip();
	let (rawnames, t): (Vec<_>, Vec<_>) = t.into_iter().unzip();
	let (methodimpls, methodimplsi): (Vec<_>, Vec<_>) = t.into_iter().unzip();

	let name   = def.ident;
	let vis    = def.vis;
	let vtable = Ident::new(&format!("__ffi_trait__{}__vtable", name), Span::call_site().into());
	let dyn_vt = Ident::new(&format!("__ffi_trait__{}__dyn_vtable", name), Span::call_site().into());
	let x = (quote! {
		#(#rawmethods)*

		#vis trait #name {
			#(#methods)*
		}

		#[repr(C)]
		#[derive(Debug, Copy, Clone, Eq, PartialEq)]
		#vis struct #vtable {
			pub __ffi_trait__size: usize,
			pub __ffi_trait__align: usize,
			pub __ffi_trait__drop_in_place: ::core::option::Option<unsafe extern "C" fn(*mut ())>,
			pub __ffi_trait__dealloc: ::core::option::Option<unsafe extern "C" fn(*mut ())>,
			#(#fields),*
		}

		unsafe impl ffi_trait::GenericVtableLayout for #vtable {}

		impl ffi_trait::FFITrait for dyn #name {
			type Vtable = #vtable;
		}

		impl<T: #name> ffi_trait::IntoTraitObject<dyn #name> for T {
			const VTABLE: &'static #vtable = &#vtable {
				__ffi_trait__size: ::core::mem::size_of::<Self>(),
				__ffi_trait__align: ::core::mem::align_of::<Self>(),
				__ffi_trait__drop_in_place: if ::core::mem::needs_drop::<Self>() {
					Some(ffi_trait::__ffi_trait__raw_drop_in_place::<Self>)
				} else { None },
				__ffi_trait__dealloc: None,
				#(#methodnames: #rawnames::<Self>),*
			};
		}

		static #dyn_vt: #vtable = #vtable {
			__ffi_trait__size: ::core::mem::size_of_val(self),
			__ffi_trait__align: ::core::mem::align_of_val(self),
			__ffi_trait__drop_in_place: ffi_trait::__ffi_trait__raw_dyn_drop_in_place::<Self>,
			__ffi_trait__dealloc: None,
			#(#methodnames: #rawnames::<Self>),*
		};

		impl ffi_trait::IntoTraitObjectRuntime<dyn #name> dyn #name {
			fn get_vt<'a>(&'a self) -> &'a #vtable {
				&#dyn_vt
			}
		}

		impl<'a> #name for ffi_trait::FFIDynRef<'a, dyn #name> {
			#(#methodimplsi)*
		}

		impl<'a> #name for ffi_trait::FFIDynMut<'a, dyn #name> {
			#(#methodimpls)*
		}
	}).into();
	x
}
use proc_macro2::TokenStream;
use quote::quote;

use crate::vtable::{Attrs, VTableDefinition};

pub fn expand(attrs: Attrs, vtable: VTableDefinition) -> TokenStream {
    let name = &vtable.module.ident;
    let vis = &vtable.module.vis;
    let ref_struct = make_ref_struct(&vtable);
    let vtables = make_vtables(&attrs, &vtable);
    let mod_items = make_mod_items(&vtable);
    let ti_structs = make_typeinfo_structs(&attrs, &vtable);
    let utils = make_utils(&vtable);

    quote::quote! {
        #vis mod #name {
            #ref_struct
            #vtables
            #mod_items
            #ti_structs
            #utils
        }
    }
}

fn make_vtables(_attrs: &Attrs, vtable: &VTableDefinition) -> TokenStream {
    let krate = crate_name("cpp-class");
    let child = quote::format_ident!("Ref{}", vtable.child.ident);
    // let base_ti_vtable = attrs.known_tables[1];
    let mut items = Vec::new();

    for (offset, base) in vtable.bases.iter().enumerate() {
        let abi = &base.abi;
        let offset = offset as isize;
        let mod_ident = quote::format_ident!("__{}", &base.ident);
        let base_ident = &base.ident;

        let mut table_fields = Vec::new();
        let mut funcs = Vec::new();

        for func in &base.funcs {
            let fn_name = &func.ident;
            let args = func.inputs.iter().skip(1);
            let retval = &func.output;

            let with_offset = if offset > 0 {
                quote! {
                    this.offset(- ( #offset * ::std::mem::size_of::<usize>() as isize ))
                }
            } else {
                quote! {
                    this
                }
            };

            let call_input = func
                .inputs
                .iter()
                .map(|arg| match arg {
                    syn::FnArg::Typed(pat) => Some(&pat.pat),
                    _ => None,
                })
                .flatten();

            let f = quote! {
                pub unsafe extern #abi fn #fn_name(this: *mut u8, #(#args),*) #retval {
                    let this = #with_offset as *mut super::#child;

                    (*this).body.#fn_name(#(#call_input),*)
                }
            };

            let args = func.inputs.iter().skip(1);

            let table_field = quote! {
                pub #fn_name: unsafe extern #abi fn(this: *mut u8, #(#args),*) #retval,
            };

            table_fields.push(table_field);
            funcs.push(f);
        }

        let type_name = byte_str(base.type_name.value(), proc_macro2::Span::call_site());

        let item = quote! {
            #[allow(non_snake_case)]
            mod #mod_ident {
                use super::#base_ident;

                #[allow(non_camel_case_types)]
                pub struct vtable {
                    #(#table_fields)*
                }

                pub static TYPE_INFO: #krate::BaseTypeInfo = #krate::BaseTypeInfo {
                    // vtable: #base_ti_vtable,
                    vtable: unsafe { &#krate::class_type_info.vtable },
                    name: #type_name.as_ptr(),
                };

                #(#funcs)*
            }
        };

        items.push(item);
    }

    quote! {
        #(#items)*
    }
}

fn make_ref_struct(vtable: &VTableDefinition) -> TokenStream {
    let child = &vtable.child.ident;
    let ref_child = quote::format_ident!("Ref{}", vtable.child.ident);
    let global_vtable = quote::format_ident!("__{}_VTABLE", vtable.child.ident);

    let tables = vtable.child.parents.iter().enumerate().map(|(idx, _)| {
        let field = quote::format_ident!("vtable_{}", idx);
        quote! { #field: *const (), }
    });

    let table_ptrs = vtable.child.parents.iter().enumerate().map(|(idx, _)| {
        let field = quote::format_ident!("vtable_{}", idx);

        quote! {
            #field: ::std::ptr::addr_of!(#global_vtable.#field.vtable) as *const _,
        }
    });

    quote! {
        #[repr(C)]
        pub struct #ref_child {
            #(#tables)*
            body: #child,
        }

        impl #ref_child {
            pub fn new_boxed(object: #child) -> *mut #ref_child {
                let object = #ref_child {
                    #(#table_ptrs)*
                    body: object,
                };

                ::std::boxed::Box::into_raw(::std::boxed::Box::new(object))
            }
        }
    }
}

fn make_mod_items(vtable: &VTableDefinition) -> TokenStream {
    let mut items = Vec::new();

    for item in &vtable.module.content.as_ref().unwrap().1 {
        items.push(item);
    }

    quote! {
        #(#items)*
    }
}

fn make_typeinfo_structs(_attrs: &Attrs, vtable: &VTableDefinition) -> TokenStream {
    let krate = crate_name("cpp-class");
    let ti = quote::format_ident!("__{}_TYPEINFO", vtable.child.ident);
    let ti_vtable = quote::format_ident!("{}VTable", vtable.child.ident);
    let global_vtable = quote::format_ident!("__{}_VTABLE", vtable.child.ident);
    let bases_count = vtable.child.parents.len();

    let mut bases = Vec::new();
    let mut vtable_fields = Vec::new();
    let mut vtable_impl = Vec::new();

    for (idx, parent) in vtable.child.parents.iter().enumerate() {
        if let Some(base) = vtable.bases.iter().find(|base| base.ident == *parent) {
            let mod_ident = quote::format_ident!("__{}", base.ident);
            let field = quote::format_ident!("vtable_{}", idx);
            let offset = idx as isize;

            let base_struct = quote! {
                #krate::Base {
                    base: &#mod_ident :: TYPE_INFO,
                    offset_flags: ((#idx * ::std::mem::size_of::<usize>()) << 8) + 2,
                }
            };

            let v_field = quote! {
                #field: #krate::GenericTable<#mod_ident::vtable, #krate::MultipleBasesTypeInfo<#bases_count>>,
            };

            let v_fields = base.funcs.iter().map(|sig| {
                let ident = &sig.ident;
                quote! { #ident: #mod_ident::#ident, }
            });

            let v_impl = quote! {
                #field: #krate::GenericTable {
                    offset: - #offset * ::std::mem::size_of::<usize>() as isize,
                    type_info: &#ti,
                    vtable: #mod_ident::vtable {
                        #(#v_fields)*
                    },
                },
            };

            bases.push(base_struct);
            vtable_fields.push(v_field);
            vtable_impl.push(v_impl);
        } else {
            proc_macro_error::abort!(parent.span(), format!("No virtual table for {}", parent));
        }
    }

    let name = vtable.child.ident.to_string();
    let ns = vtable.module.ident.to_string();
    let ti_name = byte_str(
        format!("N{}{}{}{}E", ns.len(), ns, name.len(), name),
        proc_macro2::Span::call_site(),
    );

    // let vtable = attrs.known_tables[0];
    let bases_count_u32 = bases_count as u32;

    quote! {
        #[repr(C)]
        struct #ti_vtable {
            #(#vtable_fields)*
        }

        #[allow(non_upper_case_globals)]
        static #ti: #krate::MultipleBasesTypeInfo<#bases_count> = #krate::MultipleBasesTypeInfo {
            // vtable: #vtable,
            vtable: unsafe { &#krate::vmi_class_type_info.vtable },
            name: #ti_name.as_ptr(),
            flags: 0,
            bases_count: #bases_count_u32,
            bases: [#(#bases),*],
        };

        #[allow(non_upper_case_globals)]
        static #global_vtable: #ti_vtable = #ti_vtable {
            #(#vtable_impl)*
        };
    }
}

fn crate_name(krate: &str) -> syn::Ident {
    use proc_macro_crate::FoundCrate;

    match proc_macro_crate::crate_name(krate) {
        Ok(FoundCrate::Itself) => {
            let name = krate.to_string().replace("-", "_");
            syn::Ident::new(&name, proc_macro2::Span::call_site())
        }

        Ok(FoundCrate::Name(name)) => syn::Ident::new(&name, proc_macro2::Span::call_site()),

        Err(err) => {
            proc_macro_error::abort!(proc_macro2::Span::call_site(), err.to_string());
        }
    }
}

fn byte_str<T: AsRef<str>>(name: T, span: proc_macro2::Span) -> syn::LitByteStr {
    let mut bytes = Vec::from(name.as_ref().as_bytes());
    bytes.push(0);

    syn::LitByteStr::new(&bytes, span)
}

fn make_utils(vtable: &VTableDefinition) -> TokenStream {
    let name = &vtable.child.ident;
    let ref_name = quote::format_ident!("Ref{}", name);

    quote! {
        pub fn make_boxed(object: #name) -> *mut #ref_name {
            #ref_name::new_boxed(object)
        }

        pub fn from_boxed(ptr: *mut #ref_name) -> #name {
            let wrapper = unsafe { ::std::boxed::Box::from_raw(ptr) };

            wrapper.body
        }
    }
}

use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_error::abort;
use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    Attribute, Ident, Item, ItemMod, LitInt, LitStr, Signature, Token,
};

pub fn make_vtable(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as Attrs);
    let module = parse_macro_input!(input as ItemMod);

    match VTableDefinition::try_from(module) {
        Ok(vtable) => crate::expand::expand(attr, vtable).into(),
        Err(err) => err.into_compile_error().into(),
    }
}

mod keyword {
    syn::custom_keyword!(vtable);
    syn::custom_keyword!(derive);
    syn::custom_keyword!(virtual_class);

    syn::custom_keyword!(abi);
    syn::custom_keyword!(type_name);
}

pub struct Attrs {
    pub known_tables: Vec<usize>,
}

impl Parse for Attrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = input.parse::<Ident>()?;

        // TODO: libstdc++
        if ident != "known_tables" {
            abort!(ident, "Expect known_tables");
        }

        let content;
        syn::parenthesized!(content in input);

        let tables: Punctuated<_, Token![,]> = content.parse_terminated(LitInt::parse)?;
        let known_tables = tables
            .iter()
            .filter_map(|val| val.base10_parse::<usize>().ok())
            .collect();

        Ok(Attrs { known_tables })
    }
}

struct Derive {
    parents: Punctuated<Ident, Token![,]>,
}

impl Parse for Derive {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<keyword::derive>()?;

        let content;
        syn::parenthesized!(content in input);

        let parents = content.parse_terminated(Ident::parse)?;

        Ok(Derive { parents })
    }
}

// TODO: ABI, cpp mangler
struct VirtualClass {
    abi: Ident,
    type_name: LitStr,
}

impl Parse for VirtualClass {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<keyword::virtual_class>()?;

        let content;
        syn::parenthesized!(content in input);

        let span = content.span();

        let mut abi = None;
        let mut type_name = None;

        while !content.is_empty() {
            let lookahead = content.lookahead1();

            if lookahead.peek(keyword::abi) {
                content.parse::<keyword::abi>()?;
                content.parse::<Token![=]>()?;

                abi = Some(content.parse::<Ident>()?);
            } else if lookahead.peek(keyword::type_name) {
                content.parse::<keyword::type_name>()?;
                content.parse::<Token![=]>()?;

                type_name = Some(content.parse::<LitStr>()?);
            } else {
                return Err(lookahead.error());
            }

            let _ = content.parse::<Token![,]>();
        }

        let abi = abi.unwrap_or_else(|| Ident::new("fastcall", span));
        let type_name = type_name.ok_or_else(|| syn::Error::new(span, "Expect type_name"))?;

        Ok(VirtualClass { abi, type_name })
    }
}

enum VTableAttr {
    Derive(Span, Derive),
    VirtualClass(Span, VirtualClass),
}

impl Parse for VTableAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token![#]>()?;

        let content;
        syn::bracketed!(content in input);

        content.parse::<keyword::vtable>()?;
        content.parse::<Token![::]>()?;

        let lookahead = content.lookahead1();

        if lookahead.peek(keyword::derive) {
            Ok(VTableAttr::Derive(input.span(), content.parse()?))
        } else if lookahead.peek(keyword::virtual_class) {
            Ok(VTableAttr::VirtualClass(input.span(), content.parse()?))
        } else {
            Err(lookahead.error())
        }
    }
}

impl Spanned for VTableAttr {
    fn span(&self) -> Span {
        match self {
            Self::Derive(span, _) => *span,
            Self::VirtualClass(span, _) => *span,
        }
    }
}

pub struct Child {
    pub ident: Ident,
    pub parents: Punctuated<Ident, Token![,]>,
}

pub struct Base {
    pub ident: Ident,
    pub abi: LitStr,
    pub type_name: LitStr,
    pub funcs: Vec<Signature>,
}

pub struct VTableDefinition {
    pub module: ItemMod,
    pub items: Vec<Item>,
    pub child: Child,
    pub bases: Vec<Base>,
}

impl VTableDefinition {
    fn try_from(mut item: ItemMod) -> syn::Result<Self> {
        let item_span = item.span();

        let content = &mut item
            .content
            .as_mut()
            .ok_or_else(|| syn::Error::new(item_span, "Module should be inlined"))?
            .1;

        let mut items = Vec::new();
        let mut child = None;
        let mut bases = Vec::new();

        for item in content.iter_mut() {
            match item {
                Item::Const(const_item) => {
                    let attrs = take_attrs::<VTableAttr>(&mut const_item.attrs)?;

                    if attrs.len() > 0 {
                        let first = &attrs[0];
                        abort!(
                            first.span(),
                            "vtable attrubites can be only at struct or trait"
                        );
                    }
                }

                Item::Struct(struct_item) => {
                    let attrs = take_attrs::<VTableAttr>(&mut struct_item.attrs)?;

                    for attr in attrs {
                        match attr {
                            VTableAttr::Derive(span, derive) => {
                                if child.is_some() {
                                    abort!(span, "Only one #[vtable::derive] in the module ...");
                                }

                                child = Some(Child {
                                    ident: struct_item.ident.clone(),
                                    parents: derive.parents,
                                });
                            }

                            VTableAttr::VirtualClass(span, _) => {
                                abort!(span, "#[vtable::virtual_class] should be on trait.");
                            }
                        }
                    }
                }

                Item::Trait(trait_item) => {
                    let attrs = take_attrs::<VTableAttr>(&mut trait_item.attrs)?;

                    for attr in attrs {
                        match attr {
                            VTableAttr::Derive(span, _) => {
                                abort!(span, "#[vtable::derive] should be on struct.");
                            }

                            VTableAttr::VirtualClass(_, vc) => {
                                let mut funcs = Vec::new();

                                for item in trait_item.items.iter() {
                                    match item {
                                        syn::TraitItem::Method(method) => {
                                            if method.default.is_some() {
                                                return Err(syn::Error::new(method.span(), "Trait describing virtual class cannot define defaults"));
                                            }

                                            if method.sig.receiver().is_none() {
                                                return Err(syn::Error::new(method.span(), "Trait describing virtual class cannot have static functions"));
                                            }

                                            funcs.push(method.sig.clone());
                                        }

                                        _ => (),
                                    }
                                }

                                bases.push(Base {
                                    ident: trait_item.ident.clone(),
                                    abi: LitStr::new(&vc.abi.to_string(), vc.abi.span()),
                                    type_name: vc.type_name,
                                    funcs,
                                });
                            }
                        }
                    }
                }

                _ => (),
            }

            items.push(item.clone());
        }

        let child = child
            .ok_or_else(|| syn::Error::new(item_span, "Module cannot exist without a child"))?;

        Ok(VTableDefinition {
            module: item,
            items,
            child,
            bases,
        })
    }
}

fn take_first_attr<Attr: Parse>(attrs: &mut Vec<Attribute>) -> syn::Result<Option<Attr>> {
    if let Some(idx) = attrs.iter().position(|attr| {
        attr.path
            .segments
            .first()
            .map_or(false, |seg| seg.ident == "vtable")
    }) {
        let attr = attrs.remove(idx);
        let parsed = syn::parse2(attr.into_token_stream())?;

        Ok(Some(parsed))
    } else {
        Ok(None)
    }
}

fn take_attrs<Attr: Parse>(attrs: &mut Vec<Attribute>) -> syn::Result<Vec<Attr>> {
    let mut vtable_attrs = Vec::new();

    while let Some(attr) = take_first_attr(attrs)? {
        vtable_attrs.push(attr);
    }

    Ok(vtable_attrs)
}

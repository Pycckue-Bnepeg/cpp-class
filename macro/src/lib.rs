use proc_macro::TokenStream;

pub(crate) mod expand;
pub(crate) mod vtable;

#[proc_macro_error::proc_macro_error]
#[proc_macro_attribute]
pub fn vtable(attr: TokenStream, item: TokenStream) -> TokenStream {
    vtable::make_vtable(attr, item)
}

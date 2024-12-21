extern crate proc_macro;
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn syscall_handler(_attr: TokenStream, item: TokenStream) -> proc_macro::TokenStream {
    item
}

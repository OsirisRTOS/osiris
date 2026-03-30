use quote::{ToTokens, format_ident};
use proc_macro2::TokenStream;

pub const MAX_ARGS: usize = 4;

pub fn valid_ret_type_check(item: &syn::ItemFn) -> Result<proc_macro2::TokenStream, syn::Error> {
    let ret_ty = match &item.sig.output {
        syn::ReturnType::Default => {
            // no "-> Type" present
            return Err(syn::Error::new_spanned(
                &item.sig.output,
                "syscall_handler: missing return type; expected a register‐sized type",
            ));
        }
        syn::ReturnType::Type(_, ty) => (*ty).clone(),
    };

    Ok(quote::quote! {
        const _: () = {
            if core::mem::size_of::<#ret_ty>() > core::mem::size_of::<usize>() {
                panic!("syscall_handler: the return type is bigger than usize. return type must fit in a register.");
            }
        };
    })
}

pub fn valid_arg_types_check(item: &syn::ItemFn) -> Result<Vec<syn::Type>, syn::Error> {
    let types: Vec<Result<syn::Type, syn::Error>> = item
        .sig
        .inputs
        .iter()
        .map(|arg| {
            if let syn::FnArg::Typed(pat_type) = arg {
                Ok((*pat_type.ty).clone())
            } else {
                Err(syn::Error::new(
                    item.sig.ident.span(),
                    format!(
                        "argument {} is invalid. expected a typed argument.\n",
                        arg.to_token_stream()
                    ),
                ))
            }
        })
        .collect();

    let concat_errors: Vec<_> = types
        .iter()
        .filter_map(|arg0: &std::result::Result<syn::Type, syn::Error>| Result::err(arg0.clone()))
        .collect();

    if !concat_errors.is_empty() {
        return Err(syn::Error::new(
            item.sig.ident.span(),
            format!(
                "syscall_handler: function {} has invalid arguments: {}",
                item.sig.ident,
                concat_errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
    }

    Ok(types.into_iter().map(Result::unwrap).collect())
}

pub fn syscall_handler_fn(item: &syn::ItemFn) -> TokenStream {
    let name = item.sig.ident.to_string().to_uppercase();
    let num_args = item.sig.inputs.len();

    // Check if the function has a valid signature. So args <= 4 and return type is u32.
    if num_args > MAX_ARGS {
        return syn::Error::new(
            item.sig.ident.span(),
            format!(
                "syscall_handler: function {name} has too many arguments (max is {MAX_ARGS})",
            ),
        )
        .to_compile_error();
    }

    let ret_check = match valid_ret_type_check(item) {
        Ok(check) => check,
        Err(e) => return e.to_compile_error(),
    };

    let types = match valid_arg_types_check(item) {
        Ok(types) => {
            if types.len() > MAX_ARGS {
                return syn::Error::new(
                    item.sig.ident.span(),
                    format!(
                        "syscall_handler: function {name} has too many arguments (max is {MAX_ARGS})",
                    ),
                )
                .to_compile_error();
            }
            types
        }
        Err(e) => return e.to_compile_error(),
    };

    // Check if each argument type is valid and fits in a register.
    let size_checks: Vec<TokenStream> = types.iter().map(|ty| {
        quote::quote! {
            const _: () = {
                if core::mem::size_of::<#ty>() > core::mem::size_of::<usize>() {
                    panic!("syscall_handler: an argument type is bigger than usize. arguments must fit in a register.");
                }
            };
        }
    }).collect();

    let unpack = types.iter().enumerate().map(|(i, ty)| {
        quote::quote! {
            unsafe { *(args.add(#i) as *const #ty) }
        }
    });

    let wrapper_name = format_ident!("entry_{}", item.sig.ident.clone());
    let func_name = item.sig.ident.clone();

    let call = quote::quote! {
        #func_name( #(#unpack),* )
    };

    let wrapper = quote::quote! {
        #[unsafe(no_mangle)]
        pub extern "C" fn  #wrapper_name(svc_args: *const core::ffi::c_uint) -> core::ffi::c_int {
            // This function needs to extract the arguments from the pointer and call the original function by passing the arguments as actual different parameters.
            let args = unsafe { svc_args as *const usize };
            // Call the original function with the extracted arguments.
            #call
        }
    };

    quote::quote! {
        #wrapper
        #item
        #ret_check
        #(#size_checks)*
    }
}

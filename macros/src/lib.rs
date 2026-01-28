use quote::{quote, quote_spanned};
use quote::{ToTokens, format_ident};
use syn::{parse_macro_input, parse_quote, Error, FnArg, GenericArgument, Pat, PatType, PathArguments, ReturnType, Type, TypeReference, TypeSlice};
use syn::ItemFn;

use proc_macro2::TokenStream;
use syn::spanned::Spanned;

#[proc_macro_attribute]
pub fn service(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    // This macro should be used to annotate a service struct.
    let item = syn::parse_macro_input!(item as syn::ItemStruct);

    let service_name = item.ident.clone();

    let mut mem_size: usize = 0;
    let mut stack_size: usize = 0;

    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("mem_size") {
            mem_size = meta.value()?.parse::<syn::LitInt>()?.base10_parse()?;
            Ok(())
        } else if meta.path.is_ident("stack_size") {
            stack_size = meta.value()?.parse::<syn::LitInt>()?.base10_parse()?;
            Ok(())
        } else {
            Err(meta.error("unknown attribute"))
        }
    });

    parse_macro_input!(attr with parser);

    let mem_size_ident = format_ident!("TASK_{}_MEM_SIZE", service_name.to_string().to_uppercase());
    let stack_size_ident = format_ident!(
        "TASK_{}_STACK_SIZE",
        service_name.to_string().to_uppercase()
    );

    let expanded = quote::quote! {
        const #mem_size_ident: usize = #mem_size;
        const #stack_size_ident: usize = #stack_size;
        #item

        impl #service_name {
            pub fn task_desc() -> crate::sched::task::TaskDescriptor {
                crate::sched::task::TaskDescriptor {
                    mem_size: #mem_size_ident,
                }
            }
        }
    };

    expanded.into()
}

const SYSCALL_MAX_ARGS: usize = 4;

fn is_return_type_register_sized_check(
    item: &syn::ItemFn,
) -> Result<proc_macro2::TokenStream, syn::Error> {
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

fn check_and_collect_argument_types(item: &syn::ItemFn) -> Result<Vec<syn::Type>, syn::Error> {
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

#[proc_macro_attribute]
pub fn syscall_handler(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut num = 0;

    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("num") {
            num = meta.value()?.parse::<syn::LitInt>()?.base10_parse()?;
            Ok(())
        } else {
            Err(meta.error("unknown attribute"))
        }
    });

    parse_macro_input!(attr with parser);

    let item = syn::parse_macro_input!(item as syn::ItemFn);
    syscall_handler_fn(&item).into()
}

fn syscall_handler_fn(item: &syn::ItemFn) -> TokenStream {
    let name = item.sig.ident.to_string().to_uppercase();
    let num_args = item.sig.inputs.len();

    // Check if the function has a valid signature. So args <= 4 and return type is u32.
    if num_args > SYSCALL_MAX_ARGS {
        return syn::Error::new(
            item.sig.ident.span(),
            format!(
                "syscall_handler: function {name} has too many arguments (max is {SYSCALL_MAX_ARGS})"
            ),
        )
        .to_compile_error();
    }

    let ret_check = match is_return_type_register_sized_check(item) {
        Ok(check) => check,
        Err(e) => return e.to_compile_error(),
    };

    let types = match check_and_collect_argument_types(item) {
        Ok(types) => {
            if types.len() > SYSCALL_MAX_ARGS {
                return syn::Error::new(
                    item.sig.ident.span(),
                    format!(
                        "syscall_handler: function {name} has too many arguments (max is {SYSCALL_MAX_ARGS})"
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


#[proc_macro_attribute]
pub fn kernelmod_call(_attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    
    match generate_wrapper(input) {
        Ok(tokens) => proc_macro::TokenStream::from(tokens),
        Err(e) => proc_macro::TokenStream::from(e.to_compile_error()),
    }
}

fn generate_wrapper(input: ItemFn) -> Result<TokenStream,Error> {
    let fn_name = &input.sig.ident;
    let fn_vis = &input.vis;
    let wrapper_name = syn::Ident::new(&format!("__{}_wrapper", fn_name), fn_name.span());
    let args_struct_name = syn::Ident::new(&format!("__{}Args", fn_name), fn_name.span());

    validate_return_type(&input.sig.output)?;

    let mut arg_fields = Vec::new();
    let mut arg_names = Vec::new();
    let mut arg_reconstructions = Vec::new();

    for arg in &input.sig.inputs {
        let (name, ty) = match arg {
            FnArg::Typed(PatType { pat, ty, .. }) => {
                let name = match &**pat {
                    Pat::Ident(ident) => &ident.ident,
                    _ => return Err(Error::new_spanned(pat, "Expected simple identifier pattern")),
                };
                (name, ty)
            }
            FnArg::Receiver(_) => {
                return Err(Error::new_spanned(arg, "Methods with 'self' are not supported"));
            }
        };

        arg_names.push(name.clone());

        match validate_and_generate_field(name, ty)? {
            ArgFieldInfo::Direct(field, reconstruction) => {
                arg_fields.push(field);
                arg_reconstructions.push(reconstruction);
            }
            ArgFieldInfo::Slice(ptr_field, len_field, reconstruction) => {
                arg_fields.push(ptr_field);
                arg_fields.push(len_field);
                arg_reconstructions.push(reconstruction);
            }
        }
    }

    let return_handling = generate_return_handling(&input.sig.output)?;

    let original_fn = &input;

    let output = quote! {
        #original_fn
        
        #[repr(C)]
        struct #args_struct_name {
            #(#arg_fields),*
        }
        
        #fn_vis unsafe fn #wrapper_name(args_ptr: *const u8) -> usize {
            let args = &*(args_ptr as *const #args_struct_name);
            
            #(#arg_reconstructions)*
            
            let result = #fn_name(#(#arg_names),*);
            
            #return_handling
        }
    };

    Ok(output)
}

enum ArgFieldInfo {
    Direct(proc_macro2::TokenStream, proc_macro2::TokenStream),
    Slice(proc_macro2::TokenStream, proc_macro2::TokenStream, proc_macro2::TokenStream),
}

fn validate_and_generate_field(
    name: &syn::Ident,
    ty: &Type,
) -> Result<ArgFieldInfo,Error> {
    match ty {
        Type::Path(type_path) => {
            let type_name = type_path.path.segments.last()
                .ok_or_else(|| Error::new_spanned(ty, "Invalid type path"))?
                .ident
                .to_string();

            if is_valid_primitive(&type_name) {
                let field = quote! { #name: #ty };
                let reconstruction = quote! { let #name = args.#name; };
                return Ok(ArgFieldInfo::Direct(field, reconstruction));
            }
            
            let field = quote! { #name: #ty };
            let reconstruction = quote! { 
                let #name = args.#name;
                let _: fn() = || { fn assert_copy<T: Copy>() {} assert_copy::<#ty>(); };
            };

            Ok(ArgFieldInfo::Direct(field, reconstruction))
        }
        Type::Reference(TypeReference { elem, mutability, .. }) => {
            if mutability.is_some() {
                return Err(Error::new_spanned(
                    ty,
                    "Mutable references are not supported. Only immutable references are allowed."
                ));
            }

            match &**elem {
                Type::Slice(TypeSlice { elem: slice_elem, .. }) => {
                    let ptr_name = syn::Ident::new(&format!("{}_ptr", name), name.span());
                    let len_name = syn::Ident::new(&format!("{}_len", name), name.span());

                    let ptr_field = quote! { #ptr_name: *const #slice_elem };
                    let len_field = quote! { #len_name: usize };
                    let reconstruction = quote! {
                        let #name = unsafe { 
                            core::slice::from_raw_parts(args.#ptr_name, args.#len_name) 
                        };
                    };

                    Ok(ArgFieldInfo::Slice(ptr_field, len_field, reconstruction))
                }
                Type::Path(path) => {
                    // Check for &str
                    if path.path.is_ident("str") {
                        let ptr_name = syn::Ident::new(&format!("{}_ptr", name), name.span());
                        let len_name = syn::Ident::new(&format!("{}_len", name), name.span());

                        let ptr_field = quote! { #ptr_name: *const u8 };
                        let len_field = quote! { #len_name: usize };
                        let reconstruction = quote! {
                            let #name = unsafe { 
                                let bytes = core::slice::from_raw_parts(args.#ptr_name, args.#len_name);
                                core::str::from_utf8_unchecked(bytes)
                            };
                        };

                        return Ok(ArgFieldInfo::Slice(ptr_field, len_field, reconstruction));
                    }

                    // Reference to struct - store as thin pointer
                    let field = quote! { #name: *const #path };
                    let reconstruction = quote! { let #name = unsafe { &*args.#name }; };
                    Ok(ArgFieldInfo::Direct(field, reconstruction))
                }
                _ => Err(Error::new_spanned(
                    ty,
                    "Unsupported reference type. Only references to structs, slices (&[T]), and &str are supported."
                ))
            }
        }
        _ => Err(Error::new_spanned(
            ty,
            "Unsupported argument type. Supported types are:\n\
             - Primitive types (at most usize)\n\
             - Structs implementing Copy (at most usize)\n\
             - References to structs (&T)\n\
             - Slices (&[T])\n\
             - String slices (&str)"
        ))
    }
}

fn is_valid_primitive(type_name: &str) -> bool {
    matches!(
        type_name,
        "u8" | "u16" | "u32" | "u64" | "usize" |
        "i8" | "i16" | "i32" | "i64" | "isize" |
        "bool" | "char"
    )
}

fn validate_return_type(return_type: &ReturnType) -> Result<(),Error> {
    match return_type {
        ReturnType::Default => {
            return Err(Error::new_spanned(
                return_type,
                "Function must return Result<T, UnixError> where T is a primitive type at most usize or ()"
            ));
        }
        ReturnType::Type(_, ty) => {
            if let Type::Path(type_path) = &**ty {
                let last_segment = type_path.path.segments.last()
                    .ok_or_else(|| Error::new_spanned(ty, "Invalid return type path"))?;

                if last_segment.ident != "Result" {
                    return Err(Error::new_spanned(
                        ty,
                        "Function must return Result<T, UnixError>"
                    ));
                }

                if let PathArguments::AngleBracketed(args) = &last_segment.arguments {
                    if args.args.len() != 2 {
                        return Err(Error::new_spanned(
                            ty,
                            "Result must have exactly 2 type parameters: Result<T, UnixError>"
                        ));
                    }

                    if let Some(GenericArgument::Type(Type::Path(err_path))) = args.args.iter().nth(1) {
                        if !err_path.path.is_ident("UnixError") {
                            return Err(Error::new_spanned(
                                err_path,
                                "Error type must be UnixError"
                            ));
                        }
                    }

                    return Ok(());
                }

                return Err(Error::new_spanned(
                    ty,
                    "Invalid Result type. Expected Result<T, UnixError>"
                ));
            }

            Err(Error::new_spanned(
                ty,
                "Return type must be Result<T, UnixError>"
            ))
        }
    }
}

fn generate_return_handling(return_type: &ReturnType) -> Result<proc_macro2::TokenStream,Error> {
    match return_type {
        ReturnType::Type(_, ty) => {
            if let Type::Path(type_path) = &**ty {
                if let Some(last_segment) = type_path.path.segments.last() {
                    if let PathArguments::AngleBracketed(args) = &last_segment.arguments {
                        if let Some(GenericArgument::Type(ok_type)) = args.args.first() {
                            if let Type::Tuple(tuple) = ok_type {
                                if tuple.elems.is_empty() {
                                    return Ok(quote! {
                                        match result {
                                            Ok(()) => 0,
                                            Err(e) => e as usize,
                                        }
                                    });
                                }
                            }

                            // Check if ok_type is a primitive that can be cast to usize
                            if let Type::Path(ok_path) = ok_type {
                                if let Some(ident) = ok_path.path.get_ident() {
                                    let type_str = ident.to_string();
                                    if is_valid_primitive(&type_str) {
                                        return Ok(quote! {
                                            match result {
                                                Ok(val) => val as usize,
                                                Err(e) => e as usize,
                                            }
                                        });
                                    }
                                }
                            }

                            return Err(Error::new_spanned(
                                ok_type,
                                "Return type T in Result<T, UnixError> must be a primitive type at most usize or unit ()"
                            ));
                        }
                    }
                }
            }
        }
        _ => {}
    }

    Err(Error::new_spanned(return_type, "Invalid return type"))
}

#[proc_macro_attribute]
pub fn kernel_init(_attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    item
}

#[proc_macro_attribute]
pub fn kernel_deinit(_attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    item
}
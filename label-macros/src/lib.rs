// Let's hope this is stabilized soon. There is some activity on it. https://github.com/rust-lang/rust/issues/54725
#![feature(proc_macro_span)]
///! # Label
///!
///! `label` is a library that can be used to create custom attributes for functions, through which you can list them and perform actions on them.
///!
///! For more documentation, refer to (https://docs.rs/label)[https://docs.rs/label]
///!

extern crate proc_macro;

use proc_macro::{TokenStream, Span};
use quote::{quote, format_ident};
use lazy_static::lazy_static;
use std::sync::atomic::{AtomicUsize, Ordering};
use syn::parse::{Parse, ParseStream, Result};
use syn::spanned::Spanned;

lazy_static! {
    static ref ANNOTATIONID: AtomicUsize = AtomicUsize::new(0);
}


struct ParsableAttribute{
    pub attributes: Vec<syn::Attribute>
}

impl Parse for ParsableAttribute {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(ParsableAttribute { attributes: input.call(syn::Attribute::parse_outer)? })
    }
}

fn simplify_path(path: syn::Path) -> syn::Path {
    let mut res = match path {
        syn::Path { leading_colon: leading_colon@ Some(_), segments } => syn::Path { leading_colon, segments },
        syn::Path { leading_colon: None, mut segments } => match &segments[0] {
            syn::PathSegment { ident, arguments: _ } if ident.to_string() == "crate" => syn::Path { leading_colon: None, segments },
            syn::PathSegment { ident, arguments: _ } if ident.to_string() == "self" => syn::Path { leading_colon: None, segments },
            syn::PathSegment { ident, arguments: _ } => {
                let span = ident.span();
                segments.insert(0, syn::PathSegment {
                    ident: syn::Ident::new("super", span),
                    arguments: syn::PathArguments::None
                });
                syn::Path { leading_colon: None, segments }
            }
        }
    };

    // replace ::annotate with ::add in the path.
    // It would be cleaner to remove ::annotate entirely, but couldn't find
    // a way to do that. .pop() retains the ::.
    res.segments.last_mut().map(|i| {
        assert_eq!(i.ident.to_string(), "label");
        let new_ident = syn::Ident::new("add", i.span());
        i.ident = new_ident;
    });

    res
}

#[proc_macro_attribute]
#[doc(hidden)]
/// DO NOT USE DIRECTLY! USE THROUGH CREATE_ANNOTATION
pub fn __label(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut func = syn::parse_macro_input!(item as syn::ItemFn);

    let function_name = &func.sig.ident;

    // other annotation attributes
    let mut other_annotations = Vec::new();
    // any other attribute present
    let mut other_attrs = Vec::new();
    for i in func.attrs {
        if let Some(ref lst) =  i.path.segments.last() {
            if lst.ident.to_string() == "label" {
                other_annotations.push(simplify_path(i.path));
                continue;
            }
        }
        other_attrs.push(i);
    }

    // remove all label from the function's attributes
    // but keep other attributes
    func.attrs = other_attrs;

    // for the following, the feature
    // #![feature(proc_macro_quote)]
    // could be used together with syn and proc_macro::quote_span.
    // However, this feature does not look like it will stabilize any time soon.
    // Therefore a regex is in my opinion currently better suited.
    let spans = {
        let mut current = Span::call_site();
        let mut possible = vec![current];
        loop {
            if let Some(new) = current.parent() {
                current = new;
                possible.push(new);
            } else {
                break;
            }
        }
        possible
    };


    let mut res = None;
    for span in spans.iter().rev().map(|i| i.source_text()).filter_map(|i| i) {
        if let Ok(i) = syn::parse_str::<ParsableAttribute>(&span) {
            res = Some(i);
            break;
        }
    }

    let path: syn::Path = if let Some(res) = res {
        res.attributes[0].path.clone()
    } else {
        unreachable!()
    };

    let annotation_id = ANNOTATIONID.load(Ordering::SeqCst);
    ANNOTATIONID.store(annotation_id + 1, Ordering::SeqCst);

    let varname = format_ident!("__ANNOTATION_{}_{}", function_name, annotation_id);

    let callpath = simplify_path(path);

    let result = quote! {
        #func

        #[allow(non_snake_case)]
        mod #varname {
            use label::ctor;
            use super::#function_name;

            #[ctor]
            fn create () {
                // register for all label it should be registered for
                #callpath::__add_label(&#function_name);

                #(#other_annotations ::__add_label(&#function_name);)*
            }
        }
    };

    result.into()
}


struct Signature {
    name: syn::Ident,
    params: syn::punctuated::Punctuated<syn::BareFnArg, syn::Token![,]>,
    returntype: syn::ReturnType,
}

impl Parse for Signature {
    fn parse(input: ParseStream) -> Result<Self> {
        let _ = input.parse::<syn::Token![pub]>();
        input.parse::<syn::Token![fn]>()?;

        let name = input.parse()?;


        let content;
        syn::parenthesized!(
           content in input
        );

        let params = content.parse_terminated::<_, syn::Token![,]>(syn::BareFnArg::parse)?;

        let returntype = input.parse::<syn::ReturnType>()?;

        input.parse::<syn::Token![;]>()?;

        Ok(Signature { name, params, returntype })
    }
}


#[proc_macro]
/// Creates a new label.
/// ```
/// create_label!(fn test() -> (););
/// ```
///
/// To use a label, add an attribute to a function in the following style:
///
/// ```
/// #[test::label]
/// fn my_function() {
///
/// }
///
/// ```
///
/// Test is the name of your label (this has to be a full path to it. Labels can be imported).
/// The annotation has to end with `::label`, or otherwise it will not compile.
///
pub fn create_label(signature: TokenStream) -> TokenStream {
    let Signature {
        name, params, returntype
    } = syn::parse_macro_input!(signature);


    let signature = quote! {
        &'static (dyn Fn( #params ) #returntype + 'static)
    };

    let result = quote! {
        #[allow(non_snake_case)]
        pub mod #name {
            static mut FUNCTIONS: Option<Vec<#signature>> = None;

            pub use core::iter;
            pub use label::__label as label;

            pub fn iter() -> impl Iterator<Item = #signature> {
                // Safety: after FUNCTIONS is populated (before main is called),
                // FUNCTIONS remains unchanged for the entire rest of the program.
                unsafe{
                    FUNCTIONS.iter().flat_map(|i| i).cloned()
                }
            }


            pub mod add {
                use super::*;
                // WARNING: DO NOT CALL. THIS HAS TO BE PUBLIC FOR OTHER
                // PARTS OF THE LIBRARY TO WORK BUT SHOULD NEVER BE USED.
                pub fn __add_label(func: #signature) {
                    unsafe {
                        if let Some(f) = &mut FUNCTIONS {
                            f.push(func)
                        } else {
                            FUNCTIONS = Some(vec![func])
                        }
                    }
                }
            }
        }
    };

    result.into()
}
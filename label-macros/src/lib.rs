// Let's hope this is stabilized soon. There is some activity on it. https://github.com/rust-lang/rust/issues/54725
#![feature(proc_macro_span)]
//! # Label
//!
//! `label` is a library that can be used to create custom attributes for functions, through which you can list them and perform actions on them.
//!
//! For more documentation, refer to [https://docs.rs/label](https://docs.rs/label).
//!

extern crate proc_macro;

use lazy_static::lazy_static;
use proc_macro::{Span, TokenStream};
use quote::{format_ident, quote};
use std::sync::atomic::{AtomicUsize, Ordering};
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

lazy_static! {
    static ref ANNOTATIONID: AtomicUsize = AtomicUsize::new(0);
}

struct ParsableAttribute {
    pub attributes: Vec<syn::Attribute>,
}

impl Parse for ParsableAttribute {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(ParsableAttribute {
            attributes: input.call(syn::Attribute::parse_outer)?,
        })
    }
}

fn simplify_path(path: syn::Path) -> syn::Path {
    let mut res = match path {
        syn::Path {
            leading_colon: leading_colon @ Some(_),
            segments,
        } => syn::Path {
            leading_colon,
            segments,
        },
        syn::Path {
            leading_colon: None,
            mut segments,
        } => match &segments[0] {
            syn::PathSegment {
                ident,
                arguments: _,
            } if &*ident.to_string() == "crate" => syn::Path {
                leading_colon: None,
                segments,
            },
            syn::PathSegment {
                ident,
                arguments: _,
            } if &*ident.to_string() == "self" => syn::Path {
                leading_colon: None,
                segments,
            },
            syn::PathSegment {
                ident,
                arguments: _,
            } => {
                let span = ident.span();
                segments.insert(
                    0,
                    syn::PathSegment {
                        ident: syn::Ident::new("super", span),
                        arguments: syn::PathArguments::None,
                    },
                );
                syn::Path {
                    leading_colon: None,
                    segments,
                }
            }
        },
    };

    // replace ::annotate with ::add in the path.
    // It would be cleaner to remove ::annotate entirely, but couldn't find
    // a way to do that. .pop() retains the ::.
    if let Some(i) = res.segments.last_mut() {
        assert_eq!(i.ident.to_string(), "label");
        let new_ident = syn::Ident::new("add", i.span());
        i.ident = new_ident;
    }

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
        if let Some(ref lst) = i.path.segments.last() {
            if &*lst.ident.to_string() == "label" {
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

        while let Some(new) = current.parent() {
            current = new;
            possible.push(new);
        }

        possible
    };

    let mut res = None;
    for span in spans
        .iter()
        .rev()
        .map(|i| i.source_text())
        .filter_map(|i| i)
    {
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
    let function_name_str = format!("{}", function_name);

    let result = quote! {
        #func

        #[allow(non_snake_case)]
        mod #varname {
            use label::ctor;
            use super::*;

            #[ctor]
            fn create () {
                // register for all label it should be registered for
                #callpath::__add_label(#function_name_str, &#function_name);

                #(#other_annotations ::__add_label(#function_name_str, &#function_name);)*
            }
        }
    };

    result.into()
}

struct Signatures {
    signatures: Punctuated<Signature, syn::Token![;]>,
}

impl Parse for Signatures {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            signatures: input.parse_terminated::<_, syn::Token![;]>(Signature::parse)?,
        })
    }
}

struct Signature {
    name: syn::Ident,
    params: syn::punctuated::Punctuated<syn::BareFnArg, syn::Token![,]>,
    generics: syn::Generics,
    returntype: syn::ReturnType,
}

impl Parse for Signature {
    fn parse(input: ParseStream) -> Result<Self> {
        let _ = input.parse::<syn::Visibility>();
        input.parse::<syn::Token![fn]>()?;

        let name = input.parse()?;

        let before = input.fork();

        let generics: syn::Generics = input.parse()?;

        if generics.type_params().next().is_some() {
            return Err(
                before.error("Labels can not have generic type parameters (only lifetimes).")
            );
        }
        if generics.const_params().next().is_some() {
            return Err(before.error("Labels can not have const parameters (only lifetimes)."));
        }

        let content;
        syn::parenthesized!(
           content in input
        );

        let params = content.parse_terminated::<_, syn::Token![,]>(syn::BareFnArg::parse)?;

        let returntype = input.parse::<syn::ReturnType>()?;

        Ok(Signature {
            name,
            params,
            generics,
            returntype,
        })
    }
}

#[proc_macro]
/// Creates a new label.
/// ```
/// create_label!(fn test() -> ());
/// ```
///
/// To use a label, add an attribute to a function in the following style:
///
/// ```
/// #[test::label]
/// fn my_function() {
///     // contents
/// }
///
/// ```
///
/// `test` is the name of your label (this has to be a full path to it. Labels can be imported).
/// The annotation has to end with `::label`, or otherwise it will not compile.
///
///
/// It is possible to create multipe labels in one invocation of the `create_label` macro. The syntax for this is as follows:
/// ```
/// create_label!(
///     fn test() -> ();
///     fn test1(usize) -> (usize);
///     fn test2(usize) -> (isize);
/// );
///
/// ```
///
/// It is not supported to have two labels in scope with the same name, just like two structs in the same scope with the same name won't work either.
///
///1
/// After a label is created, it is possible to iterate over all functions annotated with this label, using the iter function:
///
/// ```
/// for func in test::iter() {
///     // do something with the function
///     func();
/// }
///
/// ```
///
/// The order in which iteration occurs is *not* defined.
///
/// Alternatively, you can iterate over functions and their names using the `iter_named()` function:
///
/// ```
/// for (name, func) in test::iter_named() {
///     println!("name: {}", name);
///
///     // do something with the function
///     func();
/// }
///
/// ```
///
pub fn create_label(signatures: TokenStream) -> TokenStream {
    let labels = syn::parse_macro_input!(signatures as Signatures)
        .signatures
        .iter()
        .map(|signature| {
            let Signature {
                name,
                generics,
                params,
                returntype,
            } = signature;

            let lifetimes = generics.lifetimes();

            let signature = quote! {
                &'static (dyn for<#(#lifetimes),*> Fn ( #params ) #returntype + 'static)
            };

            quote! {
                #[allow(non_snake_case)]
                pub mod #name {
                    use super::*;

                    pub use std::collections::HashMap;
                    pub use label::__label as label;

                    static mut FUNCTIONS: Option<Vec<(&'static str, #signature)>> = None;

                    pub fn iter() -> impl Iterator<Item = #signature> {
                        // Safety: after FUNCTIONS is populated (before main is called),
                        // FUNCTIONS remains unchanged for the entire rest of the program.
                        unsafe{
                            FUNCTIONS.iter().flat_map(|i| i.iter().map(|i| &i.1)).cloned()
                        }
                    }

                    pub fn iter_named() -> impl Iterator<Item = (&'static str, #signature)> {
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
                        pub fn __add_label(name: &'static str, func: #signature) {
                            unsafe {
                                if let Some(f) = &mut FUNCTIONS {
                                    f.push((name, func));
                                } else {
                                    FUNCTIONS = Some(vec![(name, func)])
                                }
                            }
                        }
                    }
                }
            }
        })
        .collect::<Vec<_>>();

    let result = quote! {
        #(#labels)*
    };
    result.into()
}

use log::debug;

use crate::utils::generate_effect_enum::generate_effect_enum;
use crate::utils::generate_error_enum::generate_error_enum;
use crate::utils::read_rust_files::{read_rust_file_content, tokens_2_file_locations};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{File, Result};

pub fn generate_api_impl(file_pathes: TokenStream) -> Result<TokenStream> {
    simple_logger::init_with_level(log::Level::Debug).expect("faild to init logger");

    log::info!("-------- Generating API --------");

    let file_locations = tokens_2_file_locations(file_pathes)?;
    let (base_path, file_content) = read_rust_file_content(&file_locations[0])?;

    // TODO implement for more than one file
    let ast = syn::parse_file(&file_content)?;

    let domain_model_struct_name = get_domain_model_struct_name(&ast)
        .expect("Couldn't extract the domain model's name. One Struct needs to derive CqrsModel.");
    debug!("domain model name: {:#?}", domain_model_struct_name);

    // let cqrs_fns = ast
    //     .items
    //     .iter()
    //     // get all funtions from 'impl domain_model_struct'
    //     .filter_map(|item| match item {
    //         syn::Item::Impl(item_impl)
    //             if item_impl.trait_.is_none() && {
    //                 match *item_impl.self_ty.clone() {
    //                     syn::Type::Path(type_path) => {
    //                         *type_path.path.is_ident(domain_model_struct_name)
    //                     }
    //                     _ => false,
    //                 }
    //             } =>
    //         {
    //             Some(&item_impl.items)
    //         }
    //         _ => None,
    //     })
    //     // filter for -> Result<
    //     // .filter_map(|item_impl| match item_impl.items {
    //     //     syn::ItemImpl::Fn(fn_) => Some(fn_),
    //     //     _ => None,
    //     // })
    //     // .map(|ident| ident.get_ident().unwrap().to_string())
    //     // .collect::<String>();
    //     .collect::<Vec<_>>();

    // // debug!("----------- parsed items: {:#?}\n", cqrs_fns);

    // generate the code

    let generated_effect_enum = generate_effect_enum(&domain_model_struct_name, &ast);
    let generated_error_enum = generate_error_enum(&base_path, &ast);

    let generated_code = quote! {
        #generated_effect_enum
        #generated_error_enum
    };
    debug!(
        "generated code:\n----------------------------------------------------------------------------------------\n{:}\n----------------------------------------------------------------------------------------\n",
        generated_code
    );
    Ok(generated_code)
}

fn get_domain_model_struct_name(ast: &File) -> Result<String> {
    let domain_model_name = ast
        .items
        .iter()
        .filter_map(|item| match item {
            // syn::Item::Impl(item_impl)
            syn::Item::Impl(item_impl)
                if item_impl.trait_.is_some()
                    && item_impl
                        .trait_
                        .clone()
                        .expect("Should have gotten a trait")
                        .1
                        .segments
                        .iter()
                        .any(|segment| segment.ident == "CqrsModel") =>
            {
                match item_impl.self_ty.as_ref() {
                    syn::Type::Path(type_path) => Some(&type_path.path),
                    _ => None,
                }
            }
            _ => None,
        })
        .filter_map(|path| Some(path.get_ident()?.to_string()))
        .collect::<Vec<String>>();
    if domain_model_name.len() != 1 {
        return Err(syn::Error::new(
            Span::call_site(),
            "expected exactly one domain model struct",
        ));
    }
    Ok(domain_model_name[0].clone())
}

#[cfg(test)]
mod tests {
    use crate::{
        utils::generate_error_enum::generate_error_enum,
        utils::read_rust_files::read_rust_file_content,
    };
    use quote::quote;

    // use syn::{
    //     parse::{Parse, Parser},
    //     parse_str, File,
    // };
    // use proc_macro2::TokenStream;
    // pub fn prettyprint(tokens: TokenStream) -> String {
    //     prettyplease::unparse(&syn::File::parse.parse2(tokens).unwrap())
    // }

    thread_local! {
        pub (crate) static AST: syn::File = syn::parse_file(
            &read_rust_file_content("../tests/good_source_file/mod.rs")
            .unwrap()
            .1,
        ).unwrap();
    }

    #[test]
    fn generate_all_test() {
        let expected = quote! {
            use::MyGoodProcessingError;

            #[derive(thiserror::Error, Debug)]
            pub enum ProcessingError {
                #[error("Error during processing: {0}")]
                MyGoodProcessingError(MyGoodProcessingError),
                #[error("Processing was fine, but state could not be persisted: {0}")]
                NotPersisted(#[source] std::io::Error),
            }
        };
        let result = AST.with(|ast| generate_error_enum("", ast));
        assert_eq!(expected.to_string(), result.to_string());
    }
}

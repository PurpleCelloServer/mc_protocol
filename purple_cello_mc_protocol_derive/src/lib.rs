use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput};

#[proc_macro_derive(Packet)]
pub fn packet_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;

    if let Data::Struct(DataStruct { fields, .. }) = &input.data {
        let field_names: Vec<_> = fields
            .iter()
            .map(|field| field.ident.as_ref().unwrap())
            .collect();
        let field_types: Vec<_> = fields
            .iter()
            .map(|field| &field.ty)
            .collect();

        // let get_code = quote! {
        //     Ok(Self {
        //         #( #field_names: #{
        //             let data_fn = #{
        //                 let field_ty_str = stringify!(#field_types).replace(" ", "");
        //                 TYPE_FUNCTION_GET_MAPPING
        //                     .iter()
        //                     .find(|(ty, _)| field_ty_str.contains(*ty))
        //                     .map(|(_, fn_name)| fn_name)
        //                     .unwrap_or_else(|| panic!("Unknown type: {}", field_ty_str))
        //             };
        //             let fn_ident = syn::Ident::new(data_fn, proc_macro2::Span::call_site());
        //             quote! { #fn_ident(data)? }
        //         }, )*
        //     })
        // };

        // let convert_code = quote! {
        //     let mut data: Vec<u8> = vec![];
        //     data.append(&mut mc_types::convert_var_int(Self::packet_id()));
        //     #( data.append(&mut #{
        //         let data_fn = #{
        //             let field_ty_str = stringify!(#field_types).replace(" ", "");
        //             TYPE_FUNCTION_CONVERT_MAPPING
        //                 .iter()
        //                 .find(|(ty, _)| field_ty_str.contains(*ty))
        //                 .map(|(_, fn_name)| fn_name)
        //                 .unwrap_or_else(|| panic!("Unknown type: {}", field_ty_str))
        //         };
        //         let fn_ident = syn::Ident::new(data_fn, proc_macro2::Span::call_site());
        //         quote! { #fn_ident(self.#field_names) }
        //     }); )*
        // };

        let type_function_get_mapping: Vec<(&str, &str)> = vec![
            ("Boolean", "mc_types::get_boolean"),
            ("Byte", "mc_types::get_byte"),
            ("UnsignedByte", "mc_types::get_unsigned_byte"),
            ("Short", "mc_types::get_short"),
            ("UnsignedShort", "mc_types::get_unsigned_short"),
            ("Int", "mc_types::get_int"),
            ("Long", "mc_types::get_long"),
            ("Float", "mc_types::get_float"),
            ("Double", "mc_types::get_double"),
            ("String", "mc_types::get_string"),
            ("Json", "mc_types::get_string"),
            ("Identifier", "mc_types::get_string"),
            ("VarInt", "mc_types::get_var_int"),
            ("VarLong", "mc_types::get_var_long"),
            ("Position", "mc_types::get_position"),
            ("Angle", "mc_types::get_unsigned_short"),
            ("Uuid", "mc_types::get_uuid"),
            ("ByteArray", "mc_types::get_byte_array"),
        ];

        let type_function_convert_mapping: Vec<(&str, &str)> = vec![
            ("Boolean", "mc_types::convert_boolean"),
            ("Byte", "mc_types::convert_byte"),
            ("UnsignedByte", "mc_types::convert_unsigned_byte"),
            ("Short", "mc_types::convert_short"),
            ("UnsignedShort", "mc_types::convert_unsigned_short"),
            ("Int", "mc_types::convert_int"),
            ("Long", "mc_types::convert_long"),
            ("Float", "mc_types::convert_float"),
            ("Double", "mc_types::convert_double"),
            ("String", "mc_types::convert_string"),
            ("Json", "mc_types::convert_string"),
            ("Identifier", "mc_types::convert_string"),
            ("VarInt", "mc_types::convert_var_int"),
            ("VarLong", "mc_types::convert_var_long"),
            ("Position", "mc_types::convert_position"),
            ("Angle", "mc_types::convert_unsigned_short"),
            ("Uuid", "mc_types::convert_uuid"),
            ("ByteArray", "mc_types::convert_byte_array"),
        ];

        let get_code = {
            let field_code: Vec<_> = field_names.iter().zip(field_types.iter()).map(|(name, ty)| {
                let data_fn = {
                    let field_ty_str = stringify!(#ty).replace(" ", "");
                    type_function_get_mapping
                        .iter()
                        .find(|(ty, _)| field_ty_str.contains(*ty))
                        .map(|(_, fn_name)| fn_name)
                        .unwrap_or_else(|| panic!("Unknown type: {}", field_ty_str))
                };
                quote! { #name: #data_fn(data)?, }
            }).collect::<Vec<_>>();

            quote! {
                Ok(Self {
                    #( #field_code )*
                })
            }
        };

        let convert_code = {
            let field_code: Vec<_> = field_names.iter().zip(field_types.iter()).map(|(name, ty)| {
                let data_fn = {
                    let field_ty_str = stringify!(#ty).replace(" ", "");
                    type_function_convert_mapping
                        .iter()
                        .find(|(ty, _)| field_ty_str.contains(*ty))
                        .map(|(_, fn_name)| fn_name)
                        .unwrap_or_else(|| panic!("Unknown type: {}", field_ty_str))
                };
                quote! { mc_types::#data_fn(self.#name), }
            }).collect::<Vec<_>>();

            quote! {
                let mut data: Vec<u8> = vec![];
                data.append(&mut mc_types::convert_var_int(Self::packet_id()));
                #( data.append(&mut #field_code );)*
                data
            }
        };

        let gen = quote! {
            impl Packet for #struct_name {
                fn packet_id() -> i32 { 0 }

                fn get(data: &mut Vec<u8>) -> Result<Self> {
                    #get_code
                }

                fn convert(&self) -> Vec<u8> {
                    #convert_code
                }
            }
        };

        return gen.into();
    }

    TokenStream::from(quote! {
        compile_error!("Packet derive macro only supports named structs");
    })
}

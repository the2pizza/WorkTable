use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

use crate::worktable::generator::Generator;

impl Generator {
    pub fn gen_index_def(&mut self) -> TokenStream {
        let type_def = self.gen_type_def();
        let impl_def = self.gen_impl_def();

        quote! {
            #type_def
            #impl_def
        }
    }

    fn gen_type_def(&mut self) -> TokenStream {
        let name = &self.name;
        let index_rows = self.columns.indexes
            .iter()
            .map(|(i, inx)| (
                &inx.name,
                self.columns.columns_map.get(&i).clone(),
            ))
            .map(|(i, t)| {
                quote! {#i: TreeIndex<#t, Link>}
            })
            .collect::<Vec<_>>();

        let ident = Ident::new(format!("{name}Index").as_str(), Span::mixed_site());
        self.index_name = Some(ident.clone());
        let struct_def = quote! {pub struct #ident};
        quote! {
            #[derive(Debug, Default, Clone)]
            #struct_def {
                #(#index_rows),*
            }
        }
    }

    fn gen_impl_def(&mut self) -> TokenStream {
        let index_rows = self.columns.indexes
            .iter()
            .map(|(i, idx)| {
                let index_field_name = &idx.name;
                quote! {
                    self.#index_field_name.insert(row.#i, link);
                }
            }).collect::<Vec<_>>();

        let row_type_name = &self.row_name.clone().unwrap();
        let index_type_name = &self.index_name.clone().unwrap();

        quote! {
            impl TableIndex<#row_type_name> for #index_type_name {
                fn save_row(&self, row: #row_type_name, link: Link) {
                    #(#index_rows)*
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::{Ident, Span, TokenStream};
    use quote::quote;

    use crate::worktable::generator::Generator;
    use crate::worktable::Parser;

    #[test]
    fn test_type_def() {
        let tokens = TokenStream::from(quote! {
            columns: {
                id: i64 primary_key,
                test: u64,
            },
            indexes: {
                test_idx: test,
            }
        });
        let mut parser = Parser::new(tokens);

        let mut columns = parser.parse_columns().unwrap();
        let idx = parser.parse_indexes().unwrap();
        columns.indexes = idx;

        let ident = Ident::new("Test", Span::call_site());
        let mut generator = Generator::new(ident, columns);

        let res = generator.gen_type_def();

        assert_eq!(generator.index_name.unwrap().to_string(), "TestIndex".to_string());
        assert_eq!(res.to_string(), "# [derive (Debug , Default , Clone)] pub struct TestIndex { test_idx : TreeIndex < u64 , Link > }")
    }

    #[test]
    fn test_impl_def() {
        let tokens = TokenStream::from(quote! {
            columns: {
                id: i64 primary_key,
                test: u64,
            },
            indexes: {
                test_idx: test,
            }
        });
        let mut parser = Parser::new(tokens);

        let mut columns = parser.parse_columns().unwrap();
        let idx = parser.parse_indexes().unwrap();
        columns.indexes = idx;

        let ident = Ident::new("Test", Span::call_site());
        let mut generator = Generator::new(ident, columns);
        generator.gen_type_def();
        generator.gen_pk_def();
        generator.gen_row_def();

        let res = generator.gen_impl_def();

        assert_eq!(res.to_string(), "impl TableIndex < TestRow > for TestIndex { fn save_row (& self , row : TestRow , link : Link) { self . test_idx . insert (row . test , link) ; } }")
    }
}
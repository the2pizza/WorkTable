use std::collections::HashMap;

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

use crate::worktable::generator::Generator;
use crate::worktable::model::{GeneratorType, Index};

impl Generator {
    /// Generates type alias for new [`WorkTable`].
    ///
    /// [`WorkTable`]: worktable::WorkTable
    pub fn gen_table_def(&mut self) -> TokenStream {
        let name = &self.name;
        let ident = Ident::new(format!("{}WorkTable", name).as_str(), Span::mixed_site());
        self.table_name = Some(ident.clone());

        let row_type = self.row_name.as_ref().unwrap();
        let pk_type = &self.pk.as_ref().unwrap().ident;
        let index_type = self.index_name.as_ref().unwrap();


        let get_next = match self.columns.generator_type {
            GeneratorType::Custom |
            GeneratorType::Autoincrement => {
                quote! {
                pub fn get_next_pk(&self) -> #pk_type {
                    self.0.get_next_pk()
                }
            }
            }
            GeneratorType::None => {
                quote! {}
            }
        };

        let iter_with = Self::gen_iter_with();
        let iter_with_async = Self::gen_iter_with_async();

        quote! {
            #[derive(Debug, Default)]
            pub struct #ident(WorkTable<#row_type, #pk_type, #index_type>);

            impl #ident {
                pub fn select(&self, pk: #pk_type) -> Option<#row_type> {
                    self.0.select(pk)
                }

                pub fn insert(&self, row: #row_type) -> core::result::Result<#pk_type, WorkTableError> {
                    self.0.insert::<{ #row_type::ROW_SIZE }>(row)
                }

                pub async fn upsert(&self, row: #row_type) -> core::result::Result<(), WorkTableError> {
                    let pk = row.get_primary_key();
                    let need_to_update = {
                        let guard = Guard::new();
                        if let Some(_) = self.0.pk_map.peek(&pk, &guard) {
                            true
                        } else {
                            false
                        }
                    };
                    if need_to_update {
                        self.update(row).await?;
                    } else {
                        self.insert(row)?;
                    }
                    core::result::Result::Ok(())
                }

                #get_next

                #iter_with

                #iter_with_async
            }
        }
    }

    pub fn gen_table_index_impl(&mut self) -> syn::Result<TokenStream> {
        let fn_defs = self
            .columns
            .indexes
            .iter()
            .map(|(i, idx)| {
                if idx.is_unique {
                    Self::gen_unique_index_fn(
                        i,
                        idx,
                        &self.columns.columns_map,
                        self.row_name.clone().unwrap(),
                    )
                } else {
                    Self::gen_non_unique_index_fn(
                        i,
                        idx,
                        &self.columns.columns_map,
                        self.row_name.clone().unwrap(),
                    )
                }
            })
            .collect::<Result<Vec<_>, syn::Error>>()?;

        let table_ident = self.table_name.clone().unwrap();
        Ok(quote! {
            impl #table_ident {
                #(#fn_defs)*
            }
        })
    }

    fn gen_unique_index_fn(
        i: &Ident,
        idx: &Index,
        columns_map: &HashMap<Ident, Ident>,
        row_ident: Ident,
    ) -> syn::Result<TokenStream> {
        let type_ = columns_map
            .get(&i)
            .ok_or(syn::Error::new(i.span(), "Row not found"))?;
        let fn_name = Ident::new(format!("select_by_{i}").as_str(), Span::mixed_site());
        let field_ident = &idx.name;

        Ok(quote! {
            pub fn #fn_name(&self, by: #type_) -> Option<#row_ident> {
                let guard = Guard::new();
                let link = self.0.indexes.#field_ident.peek(&by, &guard)?;
                self.0.data.select(*link).ok()
            }
        })
    }

    fn gen_non_unique_index_fn(
        i: &Ident,
        idx: &Index,
        columns_map: &HashMap<Ident, Ident>,
        row_ident: Ident,
    ) -> syn::Result<TokenStream> {
        let type_ = columns_map
            .get(&i)
            .ok_or(syn::Error::new(i.span(), "Row not found"))?;
        let fn_name = Ident::new(format!("select_by_{i}").as_str(), Span::mixed_site());
        let field_ident = &idx.name;

        Ok(quote! {
            pub fn #fn_name(&self, by: #type_) -> core::result::Result<Vec<#row_ident>, WorkTableError> {
                {
                    let guard = Guard::new();
                    self.0.indexes.#field_ident
                        .peek(&by, &guard)
                        .ok_or(WorkTableError::NotFound)?
                        .iter()
                        .map(|l| *l.as_ref())
                        .collect::<Vec<_>>()
                }.iter().map(|link| {
                    self.0.data.select(*link).map_err(WorkTableError::PagesError)
                })
                .collect()
            }
        })
    }

    fn gen_iter_with() -> TokenStream {
        quote! {
            pub fn iter_with<F: Fn(TestRow) -> core::result::Result<(), WorkTableError>>(&self, f: F) -> core::result::Result<(), WorkTableError> {
                let first = {
                    let guard = Guard::new();
                    self.0.pk_map.iter(&guard).next().map(|(k, v)| (k.clone(), *v))
                };
                let Some((mut k, link)) = first else {
                    return Ok(())
                };

                let data = self.0.data.select(link).map_err(WorkTableError::PagesError)?;
                f(data)?;

                let mut ind = false;
                while !ind {
                    let next = {
                        let guard = Guard::new();
                        let mut iter = self.0.pk_map.range(k.clone().., &guard);
                        let next = iter.next().map(|(k, v)| (k.clone(), *v)).filter(|(key, _)| key != &k);
                        if next.is_some() {
                            next
                        } else {
                            iter.next().map(|(k, v)| (k.clone(), *v))
                        }
                    };
                    if let Some((key, link)) = next {
                        let data = self.0.data.select(link).map_err(WorkTableError::PagesError)?;
                        f(data)?;
                        k = key
                    } else {
                        ind = true;
                    };
                }

                core::result::Result::Ok(())
            }
        }
    }

    fn gen_iter_with_async() -> TokenStream {
        quote! {
            pub async fn iter_with_async<F: Fn(TestRow) -> Fut , Fut: std::future::Future<Output = core::result::Result<(), WorkTableError>>>(&self, f: F) ->core::result::Result<(), WorkTableError> {
                let first = {
                    let guard = Guard::new();
                    self.0.pk_map.iter(&guard).next().map(|(k, v)| (k.clone(), *v))
                };
                let Some((mut k, link)) = first else {
                    return Ok(())
                };

                let data = self.0.data.select(link).map_err(WorkTableError::PagesError)?;
                f(data).await?;

                let mut ind = false;
                while !ind {
                    let next = {
                        let guard = Guard::new();
                        let mut iter = self.0.pk_map.range(k.clone().., &guard);
                        let next = iter.next().map(|(k, v)| (k.clone(), *v)).filter(|(key, _)| key != &k);
                        if next.is_some() {
                            next
                        } else {
                            iter.next().map(|(k, v)| (k.clone(), *v))
                        }
                    };
                    if let Some((key, link)) = next {
                        let data = self.0.data.select(link).map_err(WorkTableError::PagesError)?;
                        f(data).await?;
                        k = key
                    } else {
                        ind = true;
                    };
                }

                core::result::Result::Ok(())
            }
        }
    }
}

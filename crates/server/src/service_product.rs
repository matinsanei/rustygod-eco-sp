use saleor_rustify_core::product::Product;
use saleor_rustify_db::{catalog, relations};
use saleor_rustify_proto::product::{
    product_service_server::ProductService, CreateProductRequest, CreateProductResponse,
    GetCategoryRequest, GetCategoryResponse, GetCollectionRequest, GetCollectionResponse,
    GetProductAttributesRequest, GetProductAttributesResponse, GetProductRequest,
    GetProductResponse, GetProductTypeRequest, GetProductTypeResponse, ListCategoriesRequest,
    ListCategoriesResponse, ListCollectionsRequest, ListCollectionsResponse,
    ListProductMediaRequest, ListProductMediaResponse, ListProductsRequest, ListProductsResponse,
};
use sea_orm::DatabaseConnection;
use tonic::{Request, Response, Status};

use crate::store::{lock, SharedStore};

/// Catalog service with two backends:
/// - **Postgres (Saleor Django schema)** when `db` is `Some`: same rows Django serves.
/// - **In-memory seed** otherwise (offline demos / unit tests).
pub struct ProductServiceImpl {
    store: SharedStore,
    db: Option<DatabaseConnection>,
}

impl ProductServiceImpl {
    pub fn new(store: SharedStore, db: Option<DatabaseConnection>) -> Self {
        Self { store, db }
    }

    fn err(code: &str, message: String) -> saleor_rustify_proto::common::Error {
        saleor_rustify_proto::common::Error {
            code: code.into(),
            message,
            field: String::new(),
        }
    }
}

#[tonic::async_trait]
impl ProductService for ProductServiceImpl {
    async fn get_product(
        &self,
        request: Request<GetProductRequest>,
    ) -> Result<Response<GetProductResponse>, Status> {
        let id = request.into_inner().id;
        if let Some(db) = &self.db {
            let channel = "default-channel";
            let pid: i32 = id.parse().unwrap_or(-1);
            match catalog::get_product(db, channel, pid).await {
                Ok(Some(p)) => {
                    return Ok(Response::new(GetProductResponse {
                        product: Some(p.to_proto()),
                        errors: vec![],
                    }))
                }
                Ok(None) => {
                    return Ok(Response::new(GetProductResponse {
                        product: None,
                        errors: vec![Self::err("NOT_FOUND", "product not found".into())],
                    }))
                }
                Err(e) => return Err(Status::internal(e.to_string())),
            }
        }
        let store = lock(&self.store)?;
        match store.products.get(&id) {
            Some(p) => Ok(Response::new(GetProductResponse {
                product: Some(p.to_proto()),
                errors: vec![],
            })),
            None => Ok(Response::new(GetProductResponse {
                product: None,
                errors: vec![Self::err("NOT_FOUND", "product not found".into())],
            })),
        }
    }

    async fn list_products(
        &self,
        request: Request<ListProductsRequest>,
    ) -> Result<Response<ListProductsResponse>, Status> {
        let req = request.into_inner();
        if let Some(db) = &self.db {
            let channel = if req.channel.is_empty() {
                "default-channel"
            } else {
                req.channel.as_str()
            };
            let first = if req.first <= 0 { 100 } else { (req.first as u64).min(1000) };
            let cat = if req.category_id.is_empty() {
                None
            } else {
                req.category_id.parse::<i32>().ok()
            };
            match catalog::list_products(db, channel, cat, first).await {
                Ok(products) => {
                    return Ok(Response::new(ListProductsResponse {
                        products: products.iter().map(Product::to_proto).collect(),
                        page_info: Some(saleor_rustify_proto::common::PageInfo {
                            has_next_page: false,
                            end_cursor: String::new(),
                        }),
                    }))
                }
                Err(e) => return Err(Status::internal(e.to_string())),
            }
        }
        let store = lock(&self.store)?;
        let mut products: Vec<_> = store
            .products
            .values()
            .filter(|p| {
                req.category_id.is_empty()
                    || p.category_id.as_deref() == Some(req.category_id.as_str())
            })
            .map(Product::to_proto)
            .collect();
        products.sort_by(|a, b| a.id.cmp(&b.id));
        let first = if req.first <= 0 { 100 } else { (req.first as usize).min(1000) };
        products.truncate(first);
        Ok(Response::new(ListProductsResponse {
            products,
            page_info: Some(saleor_rustify_proto::common::PageInfo {
                has_next_page: false,
                end_cursor: String::new(),
            }),
        }))
    }

    async fn create_product(
        &self,
        request: Request<CreateProductRequest>,
    ) -> Result<Response<CreateProductResponse>, Status> {
        let req = request.into_inner();
        if let Err(e) = Product::validate_new(&req.name, &req.slug) {
            return Ok(Response::new(CreateProductResponse {
                product: None,
                errors: vec![Self::err("VALIDATION", e.to_string())],
            }));
        }
        let price = match req.price {
            Some(m) => match saleor_rustify_core::money::Money::try_from(m) {
                Ok(p) => p,
                Err(e) => {
                    return Ok(Response::new(CreateProductResponse {
                        product: None,
                        errors: vec![Self::err("VALIDATION", e.to_string())],
                    }))
                }
            },
            None => {
                return Ok(Response::new(CreateProductResponse {
                    product: None,
                    errors: vec![Self::err("REQUIRED", "price is required".into())],
                }))
            }
        };
        let product = Product {
            id: format!("prod_{}", uuid::Uuid::new_v4()),
            name: req.name,
            slug: req.slug,
            description: String::new(),
            product_type_id: req.product_type_id,
            category_id: if req.category_id.is_empty() {
                None
            } else {
                Some(req.category_id)
            },
            is_published: false,
            default_price: price,
            variants: vec![],
        };
        let proto = product.to_proto();
        lock(&self.store)?
            .products
            .insert(product.id.clone(), product);
        Ok(Response::new(CreateProductResponse {
            product: Some(proto),
            errors: vec![],
        }))
    }

    async fn get_category(
        &self,
        request: Request<GetCategoryRequest>,
    ) -> Result<Response<GetCategoryResponse>, Status> {
        let Some(db) = &self.db else {
            return err_response("postgres unavailable in offline mode");
        };
        let id: i32 = request.into_inner().id.parse().unwrap_or(-1);
        match relations::get_category(db, id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
        {
            Some(c) => Ok(Response::new(GetCategoryResponse {
                category: Some(crate::mapping::category_to_proto(&c)),
                errors: vec![],
            })),
            None => Ok(Response::new(GetCategoryResponse {
                category: None,
                errors: vec![Self::err("NOT_FOUND", "category not found".into())],
            })),
        }
    }

    async fn list_categories(
        &self,
        _request: Request<ListCategoriesRequest>,
    ) -> Result<Response<ListCategoriesResponse>, Status> {
        let Some(db) = &self.db else {
            return Ok(Response::new(ListCategoriesResponse { categories: vec![] }));
        };
        let cats = relations::list_categories_tree(db)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(ListCategoriesResponse {
            categories: cats.iter().map(crate::mapping::category_to_proto).collect(),
        }))
    }

    async fn get_collection(
        &self,
        request: Request<GetCollectionRequest>,
    ) -> Result<Response<GetCollectionResponse>, Status> {
        let Some(db) = &self.db else {
            return err_response("postgres unavailable in offline mode");
        };
        let req = request.into_inner();
        let channel = if req.channel.is_empty() { "default-channel" } else { req.channel.as_str() };
        let id: i32 = req.id.parse().unwrap_or(-1);
        match relations::get_collection(db, channel, id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
        {
            Some(c) => Ok(Response::new(GetCollectionResponse {
                collection: Some(crate::mapping::collection_to_proto(&c)),
                errors: vec![],
            })),
            None => Ok(Response::new(GetCollectionResponse {
                collection: None,
                errors: vec![Self::err("NOT_FOUND", "collection not found".into())],
            })),
        }
    }

    async fn list_collections(
        &self,
        request: Request<ListCollectionsRequest>,
    ) -> Result<Response<ListCollectionsResponse>, Status> {
        let Some(db) = &self.db else {
            return Ok(Response::new(ListCollectionsResponse { collections: vec![] }));
        };
        let req = request.into_inner();
        let channel = if req.channel.is_empty() { "default-channel" } else { req.channel.as_str() };
        let cols = relations::list_collections(db, channel)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(ListCollectionsResponse {
            collections: cols.iter().map(crate::mapping::collection_to_proto).collect(),
        }))
    }

    async fn get_product_type(
        &self,
        request: Request<GetProductTypeRequest>,
    ) -> Result<Response<GetProductTypeResponse>, Status> {
        let Some(db) = &self.db else {
            return err_response("postgres unavailable in offline mode");
        };
        let id: i32 = request.into_inner().id.parse().unwrap_or(-1);
        match relations::get_product_type(db, id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
        {
            Some(t) => Ok(Response::new(GetProductTypeResponse {
                product_type: Some(crate::mapping::product_type_to_proto(&t)),
                errors: vec![],
            })),
            None => Ok(Response::new(GetProductTypeResponse {
                product_type: None,
                errors: vec![Self::err("NOT_FOUND", "product type not found".into())],
            })),
        }
    }

    async fn get_product_attributes(
        &self,
        request: Request<GetProductAttributesRequest>,
    ) -> Result<Response<GetProductAttributesResponse>, Status> {
        let Some(db) = &self.db else {
            return Ok(Response::new(GetProductAttributesResponse { attributes: vec![] }));
        };
        let pid: i32 = request.into_inner().product_id.parse().unwrap_or(-1);
        let attrs = relations::product_attributes(db, pid)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(GetProductAttributesResponse {
            attributes: crate::mapping::attributes_to_proto(&attrs),
        }))
    }

    async fn list_product_media(
        &self,
        request: Request<ListProductMediaRequest>,
    ) -> Result<Response<ListProductMediaResponse>, Status> {
        let Some(db) = &self.db else {
            return Ok(Response::new(ListProductMediaResponse { media: vec![] }));
        };
        let pid: i32 = request.into_inner().product_id.parse().unwrap_or(-1);
        let media = relations::product_media(db, pid)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(ListProductMediaResponse {
            media: crate::mapping::media_to_proto(&media),
        }))
    }
}

fn err_response<T>(msg: &str) -> Result<Response<T>, Status> {
    Err(Status::unavailable(msg))
}

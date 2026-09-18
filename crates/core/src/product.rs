use serde::{Deserialize, Serialize};

use crate::money::Money;
use crate::{DomainError, Result};

/// Mirrors `saleor/product/models.py`: Product.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub product_type_id: String,
    pub category_id: Option<String>,
    pub is_published: bool,
    pub default_price: Money,
    pub variants: Vec<ProductVariant>,
}

/// Mirrors `saleor/product/models.py`: ProductVariant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductVariant {
    pub id: String,
    pub product_id: String,
    pub name: String,
    pub sku: String,
    pub price: Money,
    pub quantity_available: i32,
}

/// Mirrors `saleor/product/models.py`: Category (MPTT tree flattened for v1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub parent_id: Option<String>,
}

impl Product {
    pub fn validate_new(name: &str, slug: &str) -> Result<()> {
        if name.trim().is_empty() {
            return Err(DomainError::Validation {
                field: "name".into(),
                message: "product name is required".into(),
            });
        }
        if slug.trim().is_empty() {
            return Err(DomainError::Validation {
                field: "slug".into(),
                message: "product slug is required".into(),
            });
        }
        Ok(())
    }

    pub fn to_proto(&self) -> rustygod_proto::product::Product {
        rustygod_proto::product::Product {
            id: self.id.clone(),
            name: self.name.clone(),
            slug: self.slug.clone(),
            description: self.description.clone(),
            product_type_id: self.product_type_id.clone(),
            category_id: self.category_id.clone().unwrap_or_default(),
            is_published: self.is_published,
            price: Some(self.default_price.clone().into()),
            variants: self.variants.iter().map(|v| v.to_proto()).collect(),
        }
    }
}

impl ProductVariant {
    pub fn to_proto(&self) -> rustygod_proto::product::ProductVariant {
        rustygod_proto::product::ProductVariant {
            id: self.id.clone(),
            product_id: self.product_id.clone(),
            name: self.name.clone(),
            sku: self.sku.clone(),
            price: Some(self.price.clone().into()),
            quantity_available: self.quantity_available,
        }
    }
}

impl Category {
    pub fn to_proto(&self) -> rustygod_proto::product::Category {
        rustygod_proto::product::Category {
            id: self.id.clone(),
            name: self.name.clone(),
            slug: self.slug.clone(),
            parent_id: self.parent_id.clone().unwrap_or_default(),
        }
    }
}

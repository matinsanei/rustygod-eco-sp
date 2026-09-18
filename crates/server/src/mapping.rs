//! View → protobuf mapping (transport belongs to the server crate,
//! never to `core` or `db`).

use rustygod_db::relations::{
    AttributeView, CategoryDetail, CollectionView, MediaView, ProductTypeView,
};
use rustygod_proto::product as pb;

pub fn category_to_proto(c: &CategoryDetail) -> pb::CategoryDetail {
    pb::CategoryDetail {
        id: c.id.to_string(),
        name: c.name.clone(),
        slug: c.slug.clone(),
        parent_id: c.parent_id.map(|p| p.to_string()).unwrap_or_default(),
        level: c.level,
        children_ids: c.children_ids.iter().map(|i| i.to_string()).collect(),
        product_count: c.product_count as i64,
    }
}

pub fn collection_to_proto(c: &CollectionView) -> pb::Collection {
    pb::Collection {
        id: c.id.to_string(),
        name: c.name.clone(),
        slug: c.slug.clone(),
        is_published: c.is_published,
        product_ids: c.product_ids.iter().map(|i| i.to_string()).collect(),
    }
}

pub fn product_type_to_proto(t: &ProductTypeView) -> pb::ProductType {
    pb::ProductType {
        id: t.id.to_string(),
        name: t.name.clone(),
        slug: t.slug.clone(),
        has_variants: t.has_variants,
        is_shipping_required: t.is_shipping_required,
        is_digital: t.is_digital,
        kind: t.kind.clone(),
    }
}

pub fn attributes_to_proto(attrs: &[AttributeView]) -> Vec<pb::ProductAttribute> {
    attrs
        .iter()
        .map(|a| pb::ProductAttribute {
            slug: a.slug.clone(),
            name: a.name.clone(),
            input_type: a.input_type.clone(),
            values: a
                .values
                .iter()
                .map(|v| pb::ProductAttributeValue {
                    slug: v.slug.clone(),
                    name: v.name.clone(),
                })
                .collect(),
        })
        .collect()
}

pub fn media_to_proto(media: &[MediaView]) -> Vec<pb::ProductMedia> {
    media
        .iter()
        .map(|m| pb::ProductMedia {
            id: m.id.to_string(),
            r#type: m.media_type.clone(),
            image: m.image.clone(),
            alt: m.alt.clone(),
            sort_order: m.sort_order,
        })
        .collect()
}

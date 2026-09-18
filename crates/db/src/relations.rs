//! Product relations over Django's tables: category tree (MPTT order),
//! collections (channel-aware), product types, assigned attributes, media.
//!
//! Mirrors `saleor/product/models.py` relations and the channel-visibility
//! rules from `saleor/graphql/product/`.

use sea_orm::{ConnectionTrait,
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, SelectorTrait,
};

use crate::{
    entities::{
        attribute_assignedproductattributevalue, attribute_attribute, attribute_attributevalue,
        product_category, product_collection, product_collectionchannellisting,
        product_collectionproduct, product_product, product_productmedia, product_producttype,
    },
    Result,
};

pub struct CategoryDetail {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub parent_id: Option<i32>,
    pub level: i32,
    pub children_ids: Vec<i32>,
    pub product_count: u64,
}

async fn category_detail(
    db: &impl sea_orm::ConnectionTrait,
    c: &product_category::Model,
) -> Result<CategoryDetail> {
    let children: Vec<i32> = product_category::Entity::find()
        .select_only()
        .column(product_category::Column::Id)
        .filter(product_category::Column::ParentId.eq(c.id))
        .into_tuple::<i32>()
        .all(db)
        .await?;
    let product_count = product_product::Entity::find()
        .filter(product_product::Column::CategoryId.eq(c.id))
        .count(db)
        .await?;
    Ok(CategoryDetail {
        id: c.id,
        name: c.name.clone(),
        slug: c.slug.clone(),
        parent_id: c.parent_id,
        level: c.level,
        children_ids: children,
        product_count,
    })
}

pub async fn list_categories_tree(db: &DatabaseConnection) -> Result<Vec<CategoryDetail>> {
    // MPTT tree order, exactly how Django walks the tree.
    let cats = product_category::Entity::find()
        .order_by_asc(product_category::Column::TreeId)
        .order_by_asc(product_category::Column::Lft)
        .all(db)
        .await?;
    let mut out = Vec::with_capacity(cats.len());
    for c in &cats {
        out.push(category_detail(db, c).await?);
    }
    Ok(out)
}

pub async fn get_category(
    db: &impl sea_orm::ConnectionTrait,
    id: i32,
) -> Result<Option<CategoryDetail>> {
    use sea_orm::EntityTrait;
    match product_category::Entity::find_by_id(id).one(db).await? {
        Some(c) => Ok(Some(category_detail(db, &c).await?)),
        None => Ok(None),
    }
}

pub struct CollectionView {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub is_published: bool,
    pub product_ids: Vec<i32>,
}

async fn collection_view(
    db: &impl sea_orm::ConnectionTrait,
    c: product_collection::Model,
    ch_id: Option<i32>,
) -> Result<CollectionView> {
    let is_published = match ch_id {
        Some(ch) => product_collectionchannellisting::Entity::find()
            .filter(product_collectionchannellisting::Column::CollectionId.eq(c.id))
            .filter(product_collectionchannellisting::Column::ChannelId.eq(ch))
            .one(db)
            .await?
            .map(|l| l.is_published)
            .unwrap_or(false),
        None => false,
    };
    let product_ids: Vec<i32> = product_collectionproduct::Entity::find()
        .select_only()
        .column(product_collectionproduct::Column::ProductId)
        .filter(product_collectionproduct::Column::CollectionId.eq(c.id))
        .order_by_asc(product_collectionproduct::Column::SortOrder)
        .into_tuple::<i32>()
        .all(db)
        .await?;
    Ok(CollectionView {
        id: c.id,
        name: c.name,
        slug: c.slug,
        is_published,
        product_ids,
    })
}

pub async fn list_collections(
    db: &impl sea_orm::ConnectionTrait,
    channel_slug: &str,
) -> Result<Vec<CollectionView>> {
    let ch_id = super::catalog::channel_info(db, channel_slug)
        .await
        .map(|(id, _)| id)
        .ok();
    let cols = product_collection::Entity::find().all(db).await?;
    let mut out = Vec::with_capacity(cols.len());
    for c in cols {
        out.push(collection_view(db, c, ch_id).await?);
    }
    Ok(out)
}

pub async fn get_collection(
    db: &impl sea_orm::ConnectionTrait,
    channel_slug: &str,
    id: i32,
) -> Result<Option<CollectionView>> {
    let ch_id = super::catalog::channel_info(db, channel_slug)
        .await
        .map(|(id, _)| id)
        .ok();
    match product_collection::Entity::find_by_id(id).one(db).await? {
        Some(c) => Ok(Some(collection_view(db, c, ch_id).await?)),
        None => Ok(None),
    }
}

pub struct ProductTypeView {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub has_variants: bool,
    pub is_shipping_required: bool,
    pub is_digital: bool,
    pub kind: String,
}

pub async fn get_product_type(
    db: &impl sea_orm::ConnectionTrait,
    id: i32,
) -> Result<Option<ProductTypeView>> {
    Ok(product_producttype::Entity::find_by_id(id)
        .one(db)
        .await?
        .map(|t| ProductTypeView {
            id: t.id,
            name: t.name,
            slug: t.slug,
            has_variants: t.has_variants,
            is_shipping_required: t.is_shipping_required,
            is_digital: t.is_digital,
            kind: t.kind,
        }))
}

pub struct AttributeValueView {
    pub slug: String,
    pub name: String,
}

pub struct AttributeView {
    pub slug: String,
    pub name: String,
    pub input_type: String,
    pub values: Vec<AttributeValueView>,
}

/// Assigned product attributes in value sort order, mirroring
/// `Product.attributes` resolution in saleor's GraphQL layer.
pub async fn product_attributes(
    db: &impl sea_orm::ConnectionTrait,
    product_id: i32,
) -> Result<Vec<AttributeView>> {
    let assignments = attribute_assignedproductattributevalue::Entity::find()
        .filter(attribute_assignedproductattributevalue::Column::ProductId.eq(product_id))
        .order_by_asc(attribute_assignedproductattributevalue::Column::SortOrder)
        .all(db)
        .await?;
    if assignments.is_empty() {
        return Ok(vec![]);
    }
    let value_ids: Vec<i32> = assignments.iter().map(|a| a.value_id).collect();
    let values = attribute_attributevalue::Entity::find()
        .filter(attribute_attributevalue::Column::Id.is_in(value_ids))
        .all(db)
        .await?;
    use std::collections::HashMap;
    let vmap: HashMap<i32, _> = values.into_iter().map(|v| (v.id, v)).collect();
    let attr_ids: Vec<i32> = vmap.values().map(|v| v.attribute_id).collect();
    let attrs = attribute_attribute::Entity::find()
        .filter(attribute_attribute::Column::Id.is_in(attr_ids))
        .all(db)
        .await?;
    let amap: HashMap<i32, _> = attrs.into_iter().map(|a| (a.id, a)).collect();

    let mut grouped: HashMap<i32, Vec<AttributeValueView>> = HashMap::new();
    let mut order: Vec<i32> = vec![];
    for a in &assignments {
        if let Some(v) = vmap.get(&a.value_id) {
            if !grouped.contains_key(&v.attribute_id) {
                order.push(v.attribute_id);
            }
            grouped.entry(v.attribute_id).or_default().push(AttributeValueView {
                slug: v.slug.clone(),
                name: v.name.clone(),
            });
        }
    }
    Ok(order
        .into_iter()
        .filter_map(|aid| {
            amap.get(&aid).map(|a| AttributeView {
                slug: a.slug.clone(),
                name: a.name.clone(),
                input_type: a.input_type.clone(),
                values: grouped.remove(&aid).unwrap_or_default(),
            })
        })
        .collect())
}

pub struct MediaView {
    pub id: i32,
    pub media_type: String,
    pub image: String,
    pub alt: String,
    pub sort_order: i32,
}

pub async fn product_media(
    db: &impl sea_orm::ConnectionTrait,
    product_id: i32,
) -> Result<Vec<MediaView>> {
    Ok(product_productmedia::Entity::find()
        .filter(product_productmedia::Column::ProductId.eq(product_id))
        .order_by_asc(product_productmedia::Column::SortOrder)
        .all(db)
        .await?
        .into_iter()
        .map(|m| MediaView {
            id: m.id,
            media_type: m.r#type,
            image: m.image.or(m.external_url).unwrap_or_default(),
            alt: m.alt,
            sort_order: m.sort_order.unwrap_or(0),
        })
        .collect())
}

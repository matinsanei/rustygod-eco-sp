//! Product-relations contract: tree, collections, types, attributes, media
//! must mirror Django's tables. Mirrors
//! `saleor/product/tests/test_category.py`,
//! `test_collections_availability.py` and attribute-assignment tests.
//!
//! Requires the Saleor database (`RUSTYGOD_DATABASE_URL`).

use rustygod_db::{database_url, relations};
use sea_orm::DatabaseConnection;

async fn db() -> DatabaseConnection {
    rustygod_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

#[tokio::test]
async fn category_tree_matches_django() {
    use rustygod_db::entities::product_category;
    use sea_orm::{EntityTrait, PaginatorTrait};

    let db = db().await;
    let cats = relations::list_categories_tree(&db).await.unwrap();
    let expected = product_category::Entity::find()
        .paginate(&db, 1000)
        .num_items()
        .await
        .unwrap();
    assert_eq!(cats.len() as u64, expected);
    assert!(!cats.is_empty());

    // MPTT order: roots (level 0) come before their descendants in each tree.
    let mut seen: std::collections::HashSet<i32> = Default::default();
    for c in &cats {
        if let Some(pid) = c.parent_id {
            assert!(
                seen.contains(&pid),
                "parent {pid} must precede child {} in tree order",
                c.id
            );
        }
        seen.insert(c.id);
    }
    // Children/parent consistency both ways.
    for c in &cats {
        for ch in &c.children_ids {
            let child = cats.iter().find(|x| &x.id == ch).unwrap();
            assert_eq!(child.parent_id, Some(c.id));
        }
    }
}

#[tokio::test]
async fn category_product_count_matches() {
    use rustygod_db::entities::product_product;
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

    let db = db().await;
    let cats = relations::list_categories_tree(&db).await.unwrap();
    let c = cats.iter().find(|c| c.product_count > 0).unwrap();
    let expected = product_product::Entity::find()
        .filter(product_product::Column::CategoryId.eq(c.id))
        .paginate(&db, 100_000)
        .num_items()
        .await
        .unwrap();
    assert_eq!(c.product_count, expected);

    let detail = relations::get_category(&db, c.id).await.unwrap().unwrap();
    assert_eq!(detail.slug, c.slug);
}

#[tokio::test]
async fn collections_match_django() {
    use rustygod_db::entities::{product_collection, product_collectionchannellisting};
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

    let db = db().await;
    let cols = relations::list_collections(&db, "default-channel").await.unwrap();
    let expected = product_collection::Entity::find()
        .paginate(&db, 1000)
        .num_items()
        .await
        .unwrap();
    assert_eq!(cols.len() as u64, expected);

    for col in &cols {
        let cid: i32 = col.id;
        let listing = product_collectionchannellisting::Entity::find()
            .filter(product_collectionchannellisting::Column::CollectionId.eq(cid))
            .filter(product_collectionchannellisting::Column::ChannelId.eq(1))
            .one(&db)
            .await
            .unwrap();
        assert_eq!(
            col.is_published,
            listing.map(|l| l.is_published).unwrap_or(false),
            "collection {cid} visibility must match Django's listing"
        );
    }
}

#[tokio::test]
async fn product_type_matches_django() {
    use rustygod_db::entities::{product_product, product_producttype};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};

    let db = db().await;
    // Project explicitly: product tables carry tsvector columns codegen mistypes.
    let (pid, ptid): (i32, i32) = product_product::Entity::find()
        .select_only()
        .column(product_product::Column::Id)
        .column(product_product::Column::ProductTypeId)
        .filter(product_product::Column::CategoryId.is_not_null())
        .into_tuple()
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    let _ = pid;
    let t = relations::get_product_type(&db, ptid).await.unwrap().unwrap();
    let (slug, has_variants, ship, kind): (String, bool, bool, String) =
        product_producttype::Entity::find_by_id(ptid)
            .select_only()
            .column(product_producttype::Column::Slug)
            .column(product_producttype::Column::HasVariants)
            .column(product_producttype::Column::IsShippingRequired)
            .column(product_producttype::Column::Kind)
            .into_tuple()
            .one(&db)
            .await
            .unwrap()
            .unwrap();
    assert_eq!(t.slug, slug);
    assert_eq!(t.has_variants, has_variants);
    assert_eq!(t.is_shipping_required, ship);
    assert_eq!(t.kind, kind);
}

#[tokio::test]
async fn attributes_match_assignments() {
    use rustygod_db::entities::attribute_assignedproductattributevalue;
    use sea_orm::EntityTrait;

    let db = db().await;
    // A product Django gave attributes to.
    let assignment = attribute_assignedproductattributevalue::Entity::find()
        .one(&db)
        .await
        .unwrap()
        .expect("populatedb must assign attributes");
    let attrs = relations::product_attributes(&db, assignment.product_id)
        .await
        .unwrap();
    assert!(!attrs.is_empty());
    let total_values: usize = attrs.iter().map(|a| a.values.len()).sum();
    assert!(total_values > 0);
    for a in &attrs {
        assert!(!a.slug.is_empty());
        assert!(!a.values.is_empty());
    }
}

#[tokio::test]
async fn media_matches_django_rows() {
    use rustygod_db::entities::product_productmedia;
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

    let db = db().await;
    let any = product_productmedia::Entity::find()
        .one(&db)
        .await
        .unwrap()
        .expect("populatedb must have media");
    let pid = any.product_id.unwrap();
    let media = relations::product_media(&db, pid).await.unwrap();
    let expected = product_productmedia::Entity::find()
        .filter(product_productmedia::Column::ProductId.eq(pid))
        .paginate(&db, 1000)
        .num_items()
        .await
        .unwrap();
    assert_eq!(media.len() as u64, expected);
}

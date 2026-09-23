//! Attribute writes contract: CRUD + values + assignments + reorders hit
//! real Django tables with Django's validations.

use rustygod_db::{attribute_writes::*, database_url};
use sea_orm::DatabaseConnection;

async fn db() -> DatabaseConnection {
    rustygod_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

fn slug(tag: &str) -> String {
    format!("t-{}-{}", tag, &uuid::Uuid::new_v4().to_string()[..8])
}

async fn product_type_id(db: &DatabaseConnection) -> i32 {
    use rustygod_db::entities::product_producttype;
    use sea_orm::{EntityTrait, QuerySelect};
    product_producttype::Entity::find()
        .select_only()
        .column(product_producttype::Column::Id)
        .into_tuple::<i32>()
        .one(db)
        .await
        .unwrap()
        .expect("seed must contain a product type")
}

async fn mk_attr(db: &DatabaseConnection, tag: &str, input_type: &str) -> i32 {
    create_attribute(
        db,
        &AttributeCreate {
            name: format!("Test {tag}"),
            slug: Some(slug(tag)),
            input_type: input_type.to_string(),
            attr_type: "product-type".to_string(),
            entity_type: None,
            unit: None,
            value_required: false,
            external_reference: None,
            reference_types: vec![],
        },
    )
    .await
    .unwrap()
}

fn vc(name: &str) -> ValueCreate {
    ValueCreate {
        name: name.to_string(),
        value: None,
        plain_text: None,
        rich_text: None,
        file_url: None,
        content_type: None,
        external_reference: None,
    }
}

#[tokio::test]
async fn attribute_crud_and_values() {
    let db = db().await;
    let aid = mk_attr(&db, "crud", "dropdown").await;

    let v1 = create_attribute_value(&db, aid, &vc("Red")).await.unwrap();
    let v2 = create_attribute_value(&db, aid, &vc("Blue")).await.unwrap();
    assert_ne!(v1, v2);

    // Reorder: Blue first.
    reorder_attribute_values(&db, aid, &[(v2, 0), (v1, 1)]).await.unwrap();
    use rustygod_db::entities::attribute_attributevalue;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
    let ordered: Vec<(i32, Option<i32>)> = attribute_attributevalue::Entity::find()
        .select_only()
        .column(attribute_attributevalue::Column::Id)
        .column(attribute_attributevalue::Column::SortOrder)
        .filter(attribute_attributevalue::Column::AttributeId.eq(aid))
        .order_by_asc(attribute_attributevalue::Column::SortOrder)
        .into_tuple::<(i32, Option<i32>)>()
        .all(&db)
        .await
        .unwrap();
    assert_eq!(ordered[0].0, v2);
    assert_eq!(ordered[1].0, v1);

    // Reorder rejects foreign values.
    assert!(reorder_attribute_values(&db, aid, &[(v1, 0), (-7, 1)]).await.is_err());

    // Update: rename + add Green + remove Red.
    update_attribute(
        &db,
        aid,
        &AttributePatch { name: Some("Test crud renamed".into()), ..Default::default() },
        &[vc("Green")],
        &[v1],
    )
    .await
    .unwrap();
    let names: Vec<String> = attribute_attributevalue::Entity::find()
        .select_only()
        .column(attribute_attributevalue::Column::Name)
        .filter(attribute_attributevalue::Column::AttributeId.eq(aid))
        .into_tuple::<String>()
        .all(&db)
        .await
        .unwrap();
    assert!(names.contains(&"Blue".to_string()));
    assert!(names.contains(&"Green".to_string()));
    assert!(!names.contains(&"Red".to_string()));

    // Bulk delete values.
    let left: Vec<i32> = attribute_attributevalue::Entity::find()
        .select_only()
        .column(attribute_attributevalue::Column::Id)
        .filter(attribute_attributevalue::Column::AttributeId.eq(aid))
        .into_tuple::<i32>()
        .all(&db)
        .await
        .unwrap();
    assert_eq!(bulk_delete_attribute_values(&db, &left).await.unwrap(), left.len() as u64);

    delete_attribute(&db, aid).await.unwrap();
    use rustygod_db::entities::attribute_attribute;
    assert!(attribute_attribute::Entity::find_by_id(aid).one(&db).await.unwrap().is_none());
}

#[tokio::test]
async fn values_rejected_on_non_choice_types() {
    let db = db().await;
    let aid = mk_attr(&db, "plain", "plain-text").await;
    assert!(create_attribute_value(&db, aid, &vc("Nope")).await.is_err());
    delete_attribute(&db, aid).await.unwrap();
}

#[tokio::test]
async fn assign_unassign_validations() {
    let db = db().await;
    let pt = product_type_id(&db).await;
    let aid = mk_attr(&db, "assign", "dropdown").await;
    let only = mk_attr(&db, "vonly", "dropdown").await;
    // Mark variant-only via patch.
    update_attribute(
        &db,
        only,
        &AttributePatch { is_variant_only: Some(true), ..Default::default() },
        &[],
        &[],
    )
    .await
    .unwrap();

    // Missing attribute.
    assert!(assign_attributes(
        &db,
        pt,
        &[AssignOp { attr_id: -7, kind: AssignKind::Product, variant_selection: false }]
    )
    .await
    .is_err());
    // Variant-only on PRODUCT scope.
    assert!(assign_attributes(
        &db,
        pt,
        &[AssignOp { attr_id: only, kind: AssignKind::Product, variant_selection: false }]
    )
    .await
    .is_err());
    // variant_selection on PRODUCT scope.
    assert!(assign_attributes(
        &db,
        pt,
        &[AssignOp { attr_id: aid, kind: AssignKind::Product, variant_selection: true }]
    )
    .await
    .is_err());

    // Real assigns: product scope + variant scope with selection.
    assign_attributes(
        &db,
        pt,
        &[
            AssignOp { attr_id: aid, kind: AssignKind::Product, variant_selection: false },
            AssignOp { attr_id: only, kind: AssignKind::Variant, variant_selection: true },
        ],
    )
    .await
    .unwrap();
    // Double assign rejected.
    assert!(assign_attributes(
        &db,
        pt,
        &[AssignOp { attr_id: aid, kind: AssignKind::Product, variant_selection: false }]
    )
    .await
    .is_err());

    // Assignment update flips variant_selection.
    update_attribute_assignment(&db, pt, &[(only, false)]).await.unwrap();
    use rustygod_db::entities::attribute_attributevariant;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
    let sel: Option<bool> = attribute_attributevariant::Entity::find()
        .select_only()
        .column(attribute_attributevariant::Column::VariantSelection)
        .filter(attribute_attributevariant::Column::ProductTypeId.eq(pt))
        .filter(attribute_attributevariant::Column::AttributeId.eq(only))
        .into_tuple::<bool>()
        .one(&db)
        .await
        .unwrap();
    assert_eq!(sel, Some(false));
    // Updating a product-scope assignment fails (no variant row).
    assert!(update_attribute_assignment(&db, pt, &[(aid, true)]).await.is_err());

    // Unassign clears both scopes.
    assert_eq!(unassign_attributes(&db, pt, &[aid, only]).await.unwrap(), 2);
    delete_attribute(&db, aid).await.unwrap();
    delete_attribute(&db, only).await.unwrap();
}

#[tokio::test]
async fn collection_reorder_persists() {
    let db = db().await;
    // Pick a collection that already has >= 2 products in seed.
    use rustygod_db::entities::product_collectionproduct;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
    let cid: i32 = product_collectionproduct::Entity::find()
        .select_only()
        .column(product_collectionproduct::Column::CollectionId)
        .into_tuple::<i32>()
        .one(&db)
        .await
        .unwrap()
        .expect("seed must contain collection memberships");
    let members: Vec<(i32, Option<i32>)> = product_collectionproduct::Entity::find()
        .select_only()
        .column(product_collectionproduct::Column::ProductId)
        .column(product_collectionproduct::Column::SortOrder)
        .filter(product_collectionproduct::Column::CollectionId.eq(cid))
        .into_tuple::<(i32, Option<i32>)>()
        .all(&db)
        .await
        .unwrap();
    assert!(members.len() >= 2, "need 2+ members to reorder");
    let moves: Vec<(i32, i32)> = members
        .iter()
        .enumerate()
        .map(|(i, (pid, _))| (*pid, (members.len() - i) as i32))
        .collect();
    reorder_collection_products(&db, cid, &moves).await.unwrap();
    let after: Vec<(i32, Option<i32>)> = product_collectionproduct::Entity::find()
        .select_only()
        .column(product_collectionproduct::Column::ProductId)
        .column(product_collectionproduct::Column::SortOrder)
        .filter(product_collectionproduct::Column::CollectionId.eq(cid))
        .into_tuple::<(i32, Option<i32>)>()
        .all(&db)
        .await
        .unwrap();
    for (pid, so) in &moves {
        assert!(after.contains(&(*pid, Some(*so))), "product {pid} must carry sort {so}");
    }
    // Foreign product rejected.
    assert!(reorder_collection_products(&db, cid, &[(-7, 0)]).await.is_err());
}

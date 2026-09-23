//! Content writes: menus (+MPTT), pages, page types.

use saleor_rustify_db::{content_writes, database_url};
use sea_orm::DatabaseConnection;
use serde_json::json;

async fn db() -> DatabaseConnection {
    saleor_rustify_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

fn target() -> content_writes::MenuItemTarget {
    content_writes::MenuItemTarget { url: Some("https://example.com".into()), category_id: None, collection_id: None, page_id: None }
}

#[tokio::test]
async fn menu_tree_lifecycle() {
    let db = db().await;
    let mid = content_writes::create_menu(&db, "Smoke Menu", None).await.unwrap();
    assert!(content_writes::create_menu(&db, "Smoke Menu", None).await.is_err());
    let root = content_writes::create_menu_item(&db, mid, "Home", &target(), None).await.unwrap();
    let kid = content_writes::create_menu_item(
        &db, mid, "Sub",
        &content_writes::MenuItemTarget { url: None, category_id: None, collection_id: None, page_id: None },
        Some(root),
    )
    .await
    .unwrap();
    // Two targets refused.
    assert!(content_writes::create_menu_item(
        &db, mid, "Bad",
        &content_writes::MenuItemTarget { url: Some("x".into()), category_id: Some(1), collection_id: None, page_id: None },
        None,
    )
    .await
    .is_err());
    // MPTT consistent: root wraps child.
    use saleor_rustify_db::entities::menu_menuitem;
    use sea_orm::EntityTrait;
    let r = menu_menuitem::Entity::find_by_id(root).one(&db).await.unwrap().unwrap();
    let k = menu_menuitem::Entity::find_by_id(kid).one(&db).await.unwrap().unwrap();
    assert_eq!(r.tree_id, mid);
    assert_eq!(k.level, 1);
    assert!(r.lft < k.lft && k.rght < r.rght);
    // Move kid to root.
    content_writes::move_menu_items(
        &db, mid,
        vec![content_writes::MenuMove { item_id: kid, parent_id: None, sort_order: Some(0) }],
    )
    .await
    .unwrap();
    let k2 = menu_menuitem::Entity::find_by_id(kid).one(&db).await.unwrap().unwrap();
    assert_eq!(k2.level, 0);
    content_writes::delete_menu(&db, mid).await.unwrap();
    assert!(menu_menuitem::Entity::find_by_id(root).one(&db).await.unwrap().is_none());
}

#[tokio::test]
async fn page_type_and_page_lifecycle() {
    let db = db().await;
    let ptid = content_writes::create_page_type(&db, Some("Smoke Type".into()), None, vec![])
        .await
        .unwrap();
    let pid = content_writes::create_page(
        &db, ptid, None, Some("Smoke Page".into()), Some(json!({"blocks": []})),
        true, None, None, None, vec![],
    )
    .await
    .unwrap();
    assert!(content_writes::create_page(
        &db, ptid, Some("smoke-page".into()), Some("Dup".into()), None,
        false, None, None, None, vec![],
    )
    .await
    .is_err());
    content_writes::update_page(&db, pid, None, Some("Smoke Page 2".into()), None, None, None, None, None, None)
        .await
        .unwrap();
    assert_eq!(content_writes::bulk_publish_pages(&db, &[pid], false).await.unwrap(), 1);
    content_writes::delete_page(&db, pid).await.unwrap();
    // Type with pages blocks delete.
    let pid2 = content_writes::create_page(&db, ptid, None, Some("P2".into()), None, false, None, None, None, vec![])
        .await
        .unwrap();
    assert!(content_writes::delete_page_type(&db, ptid).await.is_err());
    content_writes::delete_page(&db, pid2).await.unwrap();
    content_writes::delete_page_type(&db, ptid).await.unwrap();
}

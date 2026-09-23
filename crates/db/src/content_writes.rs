//! Content writes: menus (+MPTT), pages, page types (Django parity).
//!
//! Django references (`saleor/graphql/menu`, `saleor/graphql/page`,
//! `saleor/menu/models.py` MPTT):
//! - menu items are MPTT (`tree_id/lft/rght/level`); writes rebalance by
//!   rebuilding one menu's tree from `parent_id` adjacency (menus are tiny,
//!   always consistent — Django rebalances incrementally, same end state).
//! - exactly one of url/category/page/collection per item (Django
//!   `MenuItemInput` validation);
//! - page attribute values link value→page (`assignedpageattributevalue`);
//!   plain-text values resolve-or-create by slug (Django's
//!   `get_or_create` on the value);
//! - page-type attribute links carry `sort_order` (Django's reorder path).

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde_json::{json, Value};

use crate::{
    attribute_writes::slugify,
    entities::{
        attribute_assignedpageattributevalue, attribute_attribute, attribute_attributepage,
        attribute_attributevalue, menu_menu, menu_menuitem, page_page, page_pagetype,
    },
    DbError, Result,
};

fn fail(msg: impl Into<String>) -> DbError {
    DbError::App(format!("content error: {}", msg.into()))
}

// ------------------------------------------------------------------ menus --

/// Rebuild one menu's MPTT columns from parent adjacency (DFS by
/// sort_order, then id — deterministic, matches Django's ordering).
pub async fn rebuild_menu_mptt(txn: &impl ConnectionTrait, menu_id: i32) -> Result<()> {
    let rows: Vec<(i32, Option<i32>, i32)> = menu_menuitem::Entity::find()
        .select_only()
        .column(menu_menuitem::Column::Id)
        .column(menu_menuitem::Column::ParentId)
        .column(menu_menuitem::Column::SortOrder)
        .filter(menu_menuitem::Column::MenuId.eq(menu_id))
        .into_tuple()
        .all(txn)
        .await?;
    let mut kids: std::collections::HashMap<Option<i32>, Vec<(i32, i32)>> =
        std::collections::HashMap::new();
    for (id, parent, sort) in rows {
        kids.entry(parent).or_default().push((sort, id));
    }
    for v in kids.values_mut() {
        v.sort();
    }
    let mut counter = 1;
    let mut stack: Vec<(i32, i32, bool)> = kids
        .get(&None)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .rev()
        .map(|(_, id)| (id, 0, false))
        .collect();
    // Iterative DFS assigning lft on entry, rght on exit.
    let mut pos: std::collections::HashMap<i32, (i32, i32, i32)> = std::collections::HashMap::new();
    while let Some((id, level, exit)) = stack.pop() {
        if exit {
            let e = pos.get_mut(&id).expect("entry recorded");
            e.1 = counter;
            counter += 1;
            continue;
        }
        pos.insert(id, (counter, 0, level));
        counter += 1;
        stack.push((id, level, true));
        if let Some(children) = kids.get(&Some(id)) {
            for (_, kid) in children.iter().rev() {
                stack.push((*kid, level + 1, false));
            }
        }
    }
    for (id, (lft, rght, level)) in pos {
        if let Some(m) = menu_menuitem::Entity::find_by_id(id).one(txn).await? {
            let mut am: menu_menuitem::ActiveModel = m.into();
            am.lft = Set(lft);
            am.rght = Set(rght);
            am.level = Set(level);
            am.tree_id = Set(menu_id);
            am.update(txn).await?;
        }
    }
    Ok(())
}

async fn next_sort(txn: &impl ConnectionTrait, menu_id: i32, parent: Option<i32>) -> Result<i32> {
    let max: Option<i32> = menu_menuitem::Entity::find()
        .select_only()
        .column(menu_menuitem::Column::SortOrder)
        .filter(menu_menuitem::Column::MenuId.eq(menu_id))
        .filter(
            match parent {
                Some(p) => menu_menuitem::Column::ParentId.eq(p),
                None => menu_menuitem::Column::ParentId.is_null(),
            },
        )
        .order_by_desc(menu_menuitem::Column::SortOrder)
        .into_tuple()
        .one(txn)
        .await?;
    Ok(max.unwrap_or(-1) + 1)
}

pub struct MenuItemTarget {
    pub url: Option<String>,
    pub category_id: Option<i32>,
    pub collection_id: Option<i32>,
    pub page_id: Option<i32>,
}

fn check_target(t: &MenuItemTarget) -> Result<()> {
    let n = t.url.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false) as u8
        + t.category_id.is_some() as u8
        + t.collection_id.is_some() as u8
        + t.page_id.is_some() as u8;
    if n > 1 {
        return Err(fail("only one of url, category, page, collection is allowed per item"));
    }
    Ok(())
}

/// Create a menu (Django `menuCreate`); nested `items` (one level, like the
/// dashboard form) are created as roots.
pub async fn create_menu(db: &sea_orm::DatabaseConnection, name: &str, slug: Option<String>) -> Result<i32> {
    if name.trim().is_empty() {
        return Err(fail("name is required"));
    }
    let slug = slug
        .map(|s| slugify(&s))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| slugify(name));
    if menu_menu::Entity::find()
        .filter(menu_menu::Column::Slug.eq(&slug))
        .one(db)
        .await?
        .is_some()
    {
        return Err(fail("menu with this slug already exists"));
    }
    let row = menu_menu::ActiveModel {
        name: Set(name.trim().to_string()),
        slug: Set(slug),
        metadata: Set(json!({})),
        private_metadata: Set(json!({})),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(row.id)
}

/// Update name/slug (Django `menuUpdate`).
pub async fn update_menu(
    db: &sea_orm::DatabaseConnection,
    id: i32,
    name: Option<String>,
    slug: Option<String>,
) -> Result<()> {
    let m = menu_menu::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| fail(format!("menu {id} not found")))?;
    let mut am: menu_menu::ActiveModel = m.into();
    if let Some(n) = name {
        if n.trim().is_empty() {
            return Err(fail("name cannot be empty"));
        }
        am.name = Set(n.trim().to_string());
    }
    if let Some(s) = slug {
        let s = slugify(&s);
        if s.is_empty() {
            return Err(fail("slug cannot be empty"));
        }
        am.slug = Set(s);
    }
    am.update(db).await?;
    Ok(())
}

/// Delete a menu with its whole tree (Django `menuDelete`).
pub async fn delete_menu(db: &sea_orm::DatabaseConnection, id: i32) -> Result<()> {
    let txn = db.begin().await?;
    if menu_menu::Entity::find_by_id(id).one(&txn).await?.is_none() {
        return Err(fail(format!("menu {id} not found")));
    }
    menu_menuitem::Entity::delete_many()
        .filter(menu_menuitem::Column::MenuId.eq(id))
        .exec(&txn)
        .await?;
    if let Some(m) = menu_menu::Entity::find_by_id(id).one(&txn).await? {
        let am: menu_menu::ActiveModel = m.into();
        am.delete(&txn).await?;
    }
    txn.commit().await?;
    Ok(())
}

/// Create one item (Django `menuItemCreate`).
pub async fn create_menu_item(
    db: &sea_orm::DatabaseConnection,
    menu_id: i32,
    name: &str,
    target: &MenuItemTarget,
    parent_id: Option<i32>,
) -> Result<i32> {
    if name.trim().is_empty() {
        return Err(fail("name is required"));
    }
    check_target(target)?;
    let txn = db.begin().await?;
    if menu_menu::Entity::find_by_id(menu_id).one(&txn).await?.is_none() {
        return Err(fail(format!("menu {menu_id} not found")));
    }
    if let Some(pid) = parent_id {
        let p = menu_menuitem::Entity::find_by_id(pid)
            .one(&txn)
            .await?
            .ok_or_else(|| fail(format!("parent item {pid} not found")))?;
        if p.menu_id != menu_id {
            return Err(fail("parent belongs to a different menu"));
        }
    }
    let sort = next_sort(&txn, menu_id, parent_id).await?;
    let row = menu_menuitem::ActiveModel {
        name: Set(name.trim().to_string()),
        sort_order: Set(Some(sort)),
        url: Set(target.url.clone()),
        lft: Set(0),
        rght: Set(0),
        tree_id: Set(menu_id),
        level: Set(0),
        category_id: Set(target.category_id),
        collection_id: Set(target.collection_id),
        menu_id: Set(menu_id),
        page_id: Set(target.page_id),
        parent_id: Set(parent_id),
        metadata: Set(json!({})),
        private_metadata: Set(json!({})),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    rebuild_menu_mptt(&txn, menu_id).await?;
    txn.commit().await?;
    Ok(row.id)
}

/// Update one item (Django `menuItemUpdate`).
pub async fn update_menu_item(
    db: &sea_orm::DatabaseConnection,
    id: i32,
    name: Option<String>,
    target: &MenuItemTarget,
    clear_target: bool,
) -> Result<i32> {
    check_target(target)?;
    let txn = db.begin().await?;
    let m = menu_menuitem::Entity::find_by_id(id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail(format!("menu item {id} not found")))?;
    let menu_id = m.menu_id;
    let mut am: menu_menuitem::ActiveModel = m.into();
    if let Some(n) = name {
        if n.trim().is_empty() {
            return Err(fail("name cannot be empty"));
        }
        am.name = Set(n.trim().to_string());
    }
    if target.url.is_some() || clear_target {
        am.url = Set(target.url.clone());
        am.category_id = Set(None);
        am.collection_id = Set(None);
        am.page_id = Set(None);
    }
    if target.category_id.is_some() {
        am.category_id = Set(target.category_id);
        am.url = Set(None);
        am.collection_id = Set(None);
        am.page_id = Set(None);
    }
    if target.collection_id.is_some() {
        am.collection_id = Set(target.collection_id);
        am.url = Set(None);
        am.category_id = Set(None);
        am.page_id = Set(None);
    }
    if target.page_id.is_some() {
        am.page_id = Set(target.page_id);
        am.url = Set(None);
        am.category_id = Set(None);
        am.collection_id = Set(None);
    }
    am.update(&txn).await?;
    let _ = menu_id;
    txn.commit().await?;
    Ok(id)
}

/// Move items (Django `menuItemMove`): reparent + resort, then rebalance.
pub struct MenuMove {
    pub item_id: i32,
    pub parent_id: Option<i32>,
    pub sort_order: Option<i32>,
}

pub async fn move_menu_items(
    db: &sea_orm::DatabaseConnection,
    menu_id: i32,
    moves: Vec<MenuMove>,
) -> Result<()> {
    if moves.is_empty() {
        return Err(fail("no moves provided"));
    }
    let txn = db.begin().await?;
    if menu_menu::Entity::find_by_id(menu_id).one(&txn).await?.is_none() {
        return Err(fail(format!("menu {menu_id} not found")));
    }
    for mv in &moves {
        let m = menu_menuitem::Entity::find_by_id(mv.item_id)
            .one(&txn)
            .await?
            .ok_or_else(|| fail(format!("menu item {} not found", mv.item_id)))?;
        if m.menu_id != menu_id {
            return Err(fail("item belongs to a different menu"));
        }
        if let Some(pid) = mv.parent_id {
            if pid == mv.item_id {
                return Err(fail("an item cannot be its own parent"));
            }
            let p = menu_menuitem::Entity::find_by_id(pid)
                .one(&txn)
                .await?
                .ok_or_else(|| fail(format!("parent item {pid} not found")))?;
            if p.menu_id != menu_id {
                return Err(fail("parent belongs to a different menu"));
            }
        }
        let mut am: menu_menuitem::ActiveModel = m.into();
        am.parent_id = Set(mv.parent_id);
        if let Some(s) = mv.sort_order {
            am.sort_order = Set(Some(s));
        }
        am.update(&txn).await?;
    }
    rebuild_menu_mptt(&txn, menu_id).await?;
    txn.commit().await?;
    Ok(())
}

// ------------------------------------------------------------------ pages --

/// Resolve-or-create a plain-text value for an attribute (Django's
/// value get_or_create on page assignment).
async fn value_for(
    txn: &impl ConnectionTrait,
    attribute_id: i32,
    text: &str,
) -> Result<i32> {
    let slug = slugify(text);
    if let Some(v) = attribute_attributevalue::Entity::find()
        .filter(attribute_attributevalue::Column::AttributeId.eq(attribute_id))
        .filter(attribute_attributevalue::Column::Slug.eq(&slug))
        .one(txn)
        .await?
    {
        return Ok(v.id);
    }
    let row = attribute_attributevalue::ActiveModel {
        name: Set(text.to_string()),
        attribute_id: Set(attribute_id),
        slug: Set(slug),
        value: Set(text.to_string()),
        ..Default::default()
    }
    .insert(txn)
    .await?;
    Ok(row.id)
}

pub struct PageAttrInput {
    pub attribute_id: i32,
    pub values: Vec<String>,
}

/// Replace a page's attribute values (Django `PageInput.attributes`).
pub async fn set_page_attributes(
    txn: &impl ConnectionTrait,
    page_id: i32,
    attrs: &[PageAttrInput],
) -> Result<()> {
    for a in attrs {
        if attribute_attribute::Entity::find_by_id(a.attribute_id).one(txn).await?.is_none() {
            return Err(fail(format!("attribute {} not found", a.attribute_id)));
        }
        // Clear this attribute's rows (values carry the attribute link).
        let old: Vec<i32> = attribute_assignedpageattributevalue::Entity::find()
            .select_only()
            .column(attribute_assignedpageattributevalue::Column::Id)
            .filter(attribute_assignedpageattributevalue::Column::PageId.eq(page_id))
            .into_tuple()
            .all(txn)
            .await?;
        for oid in old {
            if let Some(row) = attribute_assignedpageattributevalue::Entity::find_by_id(oid).one(txn).await? {
                if let Some(v) = attribute_attributevalue::Entity::find_by_id(row.value_id).one(txn).await? {
                    if v.attribute_id == a.attribute_id {
                        let dam: attribute_assignedpageattributevalue::ActiveModel = row.into();
                        dam.delete(txn).await?;
                    }
                }
            }
        }
        for (i, text) in a.values.iter().enumerate() {
            let vid = value_for(txn, a.attribute_id, text).await?;
            attribute_assignedpageattributevalue::ActiveModel {
                sort_order: Set(Some(i as i32)),
                value_id: Set(vid),
                page_id: Set(page_id),
                ..Default::default()
            }
            .insert(txn)
            .await?;
        }
    }
    Ok(())
}

/// Create a page (Django `pageCreate`).
#[allow(clippy::too_many_arguments)]
pub async fn create_page(
    db: &sea_orm::DatabaseConnection,
    page_type_id: i32,
    slug: Option<String>,
    title: Option<String>,
    content: Option<Value>,
    is_published: bool,
    published_at: Option<chrono::DateTime<Utc>>,
    seo_title: Option<String>,
    seo_description: Option<String>,
    attrs: Vec<PageAttrInput>,
) -> Result<i32> {
    if page_pagetype::Entity::find_by_id(page_type_id).one(db).await?.is_none() {
        return Err(fail(format!("page type {page_type_id} not found")));
    }
    let slug = slug.map(|s| slugify(&s)).filter(|s| !s.is_empty()).unwrap_or_else(|| slugify(&title.clone().unwrap_or_default()));
    if slug.is_empty() {
        return Err(fail("slug is required"));
    }
    if page_page::Entity::find()
        .filter(page_page::Column::Slug.eq(&slug))
        .one(db)
        .await?
        .is_some()
    {
        return Err(fail("page with this slug already exists"));
    }
    let txn = db.begin().await?;
    let t = Utc::now();
    let row = page_page::ActiveModel {
        slug: Set(slug),
        title: Set(title.unwrap_or_default()),
        content: Set(content),
        created_at: Set(t.into()),
        is_published: Set(is_published),
        published_at: Set(published_at.map(|d| d.into())),
        seo_description: Set(seo_description),
        seo_title: Set(seo_title),
        metadata: Set(json!({})),
        private_metadata: Set(json!({})),
        page_type_id: Set(page_type_id),
        search_index_dirty: Set(true),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    set_page_attributes(&txn, row.id, &attrs).await?;
    txn.commit().await?;
    Ok(row.id)
}

/// Update a page (Django `pageUpdate`).
#[allow(clippy::too_many_arguments)]
pub async fn update_page(
    db: &sea_orm::DatabaseConnection,
    id: i32,
    slug: Option<String>,
    title: Option<String>,
    content: Option<Value>,
    is_published: Option<bool>,
    published_at: Option<chrono::DateTime<Utc>>,
    seo_title: Option<String>,
    seo_description: Option<String>,
    attrs: Option<Vec<PageAttrInput>>,
) -> Result<()> {
    let txn = db.begin().await?;
    let p = page_page::Entity::find_by_id(id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail(format!("page {id} not found")))?;
    let mut am: page_page::ActiveModel = p.into();
    if let Some(s) = slug {
        let s = slugify(&s);
        if s.is_empty() {
            return Err(fail("slug cannot be empty"));
        }
        am.slug = Set(s);
    }
    if let Some(t) = title {
        am.title = Set(t);
    }
    if let Some(c) = content {
        am.content = Set(Some(c));
    }
    if let Some(x) = is_published {
        am.is_published = Set(x);
    }
    if let Some(pa) = published_at {
        am.published_at = Set(Some(pa.into()));
    }
    if seo_title.is_some() {
        am.seo_title = Set(seo_title);
    }
    if seo_description.is_some() {
        am.seo_description = Set(seo_description);
    }
    am.update(&txn).await?;
    if let Some(a) = attrs {
        set_page_attributes(&txn, id, &a).await?;
    }
    txn.commit().await?;
    Ok(())
}

/// Delete a page with its attribute rows (Django `pageDelete`).
pub async fn delete_page(db: &sea_orm::DatabaseConnection, id: i32) -> Result<()> {
    let txn = db.begin().await?;
    if page_page::Entity::find_by_id(id).one(&txn).await?.is_none() {
        return Err(fail(format!("page {id} not found")));
    }
    attribute_assignedpageattributevalue::Entity::delete_many()
        .filter(attribute_assignedpageattributevalue::Column::PageId.eq(id))
        .exec(&txn)
        .await?;
    if let Some(p) = page_page::Entity::find_by_id(id).one(&txn).await? {
        let am: page_page::ActiveModel = p.into();
        am.delete(&txn).await?;
    }
    txn.commit().await?;
    Ok(())
}

/// Bulk publish toggle (Django `pageBulkPublish`).
pub async fn bulk_publish_pages(db: &sea_orm::DatabaseConnection, ids: &[i32], published: bool) -> Result<i32> {
    let mut n = 0;
    for id in ids {
        if let Some(p) = page_page::Entity::find_by_id(*id).one(db).await? {
            let mut am: page_page::ActiveModel = p.into();
            am.is_published = Set(published);
            if published {
                am.published_at = Set(Some(Utc::now().into()));
            }
            am.update(db).await?;
            n += 1;
        }
    }
    Ok(n)
}

// ------------------------------------------------------------- page types --

/// Create a page type with attribute links (Django `pageTypeCreate`).
pub async fn create_page_type(
    db: &sea_orm::DatabaseConnection,
    name: Option<String>,
    slug: Option<String>,
    attributes: Vec<i32>,
) -> Result<i32> {
    let name = name.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).ok_or_else(|| fail("name is required"))?;
    let slug = slug.map(|s| slugify(&s)).filter(|s| !s.is_empty()).unwrap_or_else(|| slugify(&name));
    if page_pagetype::Entity::find()
        .filter(page_pagetype::Column::Slug.eq(&slug))
        .one(db)
        .await?
        .is_some()
    {
        return Err(fail("page type with this slug already exists"));
    }
    let txn = db.begin().await?;
    let row = page_pagetype::ActiveModel {
        metadata: Set(json!({})),
        private_metadata: Set(json!({})),
        name: Set(name),
        slug: Set(slug),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    for (i, aid) in attributes.iter().enumerate() {
        if attribute_attribute::Entity::find_by_id(*aid).one(&txn).await?.is_none() {
            return Err(fail(format!("attribute {aid} not found")));
        }
        attribute_attributepage::ActiveModel {
            sort_order: Set(Some(i as i32)),
            attribute_id: Set(*aid),
            page_type_id: Set(row.id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
    }
    txn.commit().await?;
    Ok(row.id)
}

/// Update a page type (Django `pageTypeUpdate`).
pub async fn update_page_type(
    db: &sea_orm::DatabaseConnection,
    id: i32,
    name: Option<String>,
    slug: Option<String>,
    add_attributes: Vec<i32>,
    remove_attributes: Vec<i32>,
) -> Result<()> {
    let txn = db.begin().await?;
    let pt = page_pagetype::Entity::find_by_id(id)
        .one(&txn)
        .await?
        .ok_or_else(|| fail(format!("page type {id} not found")))?;
    let mut am: page_pagetype::ActiveModel = pt.into();
    if let Some(n) = name {
        if n.trim().is_empty() {
            return Err(fail("name cannot be empty"));
        }
        am.name = Set(n.trim().to_string());
    }
    if let Some(s) = slug {
        let s = slugify(&s);
        if s.is_empty() {
            return Err(fail("slug cannot be empty"));
        }
        am.slug = Set(s);
    }
    am.update(&txn).await?;
    for aid in &add_attributes {
        if attribute_attribute::Entity::find_by_id(*aid).one(&txn).await?.is_none() {
            return Err(fail(format!("attribute {aid} not found")));
        }
        let exists = attribute_attributepage::Entity::find()
            .filter(attribute_attributepage::Column::PageTypeId.eq(id))
            .filter(attribute_attributepage::Column::AttributeId.eq(*aid))
            .one(&txn)
            .await?
            .is_some();
        if !exists {
            let max: Option<i32> = attribute_attributepage::Entity::find()
                .select_only()
                .column(attribute_attributepage::Column::SortOrder)
                .filter(attribute_attributepage::Column::PageTypeId.eq(id))
                .order_by_desc(attribute_attributepage::Column::SortOrder)
                .into_tuple()
                .one(&txn)
                .await?
                .flatten();
            attribute_attributepage::ActiveModel {
                sort_order: Set(Some(max.unwrap_or(-1) + 1)),
                attribute_id: Set(*aid),
                page_type_id: Set(id),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
        }
    }
    if !remove_attributes.is_empty() {
        attribute_attributepage::Entity::delete_many()
            .filter(attribute_attributepage::Column::PageTypeId.eq(id))
            .filter(attribute_attributepage::Column::AttributeId.is_in(remove_attributes))
            .exec(&txn)
            .await?;
    }
    txn.commit().await?;
    Ok(())
}

/// Delete a page type (Django `pageTypeDelete`): pages block the delete.
pub async fn delete_page_type(db: &sea_orm::DatabaseConnection, id: i32) -> Result<()> {
    let txn = db.begin().await?;
    if page_pagetype::Entity::find_by_id(id).one(&txn).await?.is_none() {
        return Err(fail(format!("page type {id} not found")));
    }
    let n = page_page::Entity::find()
        .filter(page_page::Column::PageTypeId.eq(id))
        .count(&txn)
        .await?;
    if n > 0 {
        return Err(fail("page type with pages cannot be deleted"));
    }
    attribute_attributepage::Entity::delete_many()
        .filter(attribute_attributepage::Column::PageTypeId.eq(id))
        .exec(&txn)
        .await?;
    if let Some(pt) = page_pagetype::Entity::find_by_id(id).one(&txn).await? {
        let am: page_pagetype::ActiveModel = pt.into();
        am.delete(&txn).await?;
    }
    txn.commit().await?;
    Ok(())
}

/// Reorder a page type's attributes (Django `pageTypeReorderAttributes`).
pub async fn reorder_page_type_attributes(
    db: &sea_orm::DatabaseConnection,
    page_type_id: i32,
    moves: Vec<(i32, i32)>,
) -> Result<()> {
    let txn = db.begin().await?;
    if page_pagetype::Entity::find_by_id(page_type_id).one(&txn).await?.is_none() {
        return Err(fail(format!("page type {page_type_id} not found")));
    }
    for (attr_id, sort) in moves {
        if let Some(link) = attribute_attributepage::Entity::find()
            .filter(attribute_attributepage::Column::PageTypeId.eq(page_type_id))
            .filter(attribute_attributepage::Column::AttributeId.eq(attr_id))
            .one(&txn)
            .await?
        {
            let mut am: attribute_attributepage::ActiveModel = link.into();
            am.sort_order = Set(Some(sort));
            am.update(&txn).await?;
        }
    }
    txn.commit().await?;
    Ok(())
}

/// Reorder one page's attribute values (Django `pageReorderAttributeValues`).
pub async fn reorder_page_attribute_values(
    db: &sea_orm::DatabaseConnection,
    page_id: i32,
    attribute_id: i32,
    moves: Vec<(i32, i32)>,
) -> Result<()> {
    let txn = db.begin().await?;
    if page_page::Entity::find_by_id(page_id).one(&txn).await?.is_none() {
        return Err(fail(format!("page {page_id} not found")));
    }
    for (value_id, sort) in moves {
        if let Some(row) = attribute_assignedpageattributevalue::Entity::find()
            .filter(attribute_assignedpageattributevalue::Column::PageId.eq(page_id))
            .filter(attribute_assignedpageattributevalue::Column::ValueId.eq(value_id))
            .one(&txn)
            .await?
        {
            // Guard: the value must belong to the named attribute.
            if let Some(v) = attribute_attributevalue::Entity::find_by_id(value_id).one(&txn).await? {
                if v.attribute_id != attribute_id {
                    continue;
                }
            }
            let mut am: attribute_assignedpageattributevalue::ActiveModel = row.into();
            am.sort_order = Set(Some(sort));
            am.update(&txn).await?;
        }
    }
    txn.commit().await?;
    Ok(())
}

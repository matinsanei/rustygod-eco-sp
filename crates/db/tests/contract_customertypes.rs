//! Customer types, tax configuration updates, notification recipients.

use rustygod_db::{account_writes, customer_types::*, database_url};
use sea_orm::DatabaseConnection;

async fn db() -> DatabaseConnection {
    rustygod_db::connect(&database_url())
        .await
        .expect("saleor postgres must be up (localhost:5434)")
}

fn tag(p: &str) -> String {
    format!("{p}-{}", &uuid::Uuid::new_v4().to_string()[..8])
}

#[tokio::test]
async fn customer_type_crud_guards() {
    let db = db().await;
    // Default type cannot be deleted.
    use rustygod_db::entities::account_customertype;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
    let default_id: i32 = account_customertype::Entity::find()
        .select_only()
        .column(account_customertype::Column::Id)
        .filter(account_customertype::Column::IsDefault.eq(true))
        .into_tuple::<i32>()
        .one(&db)
        .await
        .unwrap()
        .expect("seed must have a default customer type");
    assert!(delete_customer_type(&db, default_id).await.is_err());

    let ct = create_customer_type(
        &db,
        &CustomerTypeCreate { name: Some(tag("Type")), slug: None, is_default: false },
    )
    .await
    .unwrap();
    // Duplicate name/slug rejected.
    assert!(create_customer_type(
        &db,
        &CustomerTypeCreate { name: Some(" quests ".into()), slug: None, is_default: false },
    )
    .await
    .is_ok()); // different name is fine
    let clash = tag("Type");
    assert!(create_customer_type(
        &db,
        &CustomerTypeCreate { name: Some(clash.clone()), slug: None, is_default: false },
    )
    .await
    .is_ok());
    assert!(create_customer_type(
        &db,
        &CustomerTypeCreate { name: Some(clash), slug: None, is_default: false },
    )
    .await
    .is_err());
    update_customer_type(&db, ct, &CustomerTypePatch { name: Some(tag("Renamed")), slug: None, is_default: None })
        .await
        .unwrap();
    delete_customer_type(&db, ct).await.unwrap();
    assert!(account_customertype::Entity::find_by_id(ct).one(&db).await.unwrap().is_none());
}

#[tokio::test]
async fn customer_type_attribute_assignment() {
    let db = db().await;
    let ct = create_customer_type(
        &db,
        &CustomerTypeCreate { name: Some(tag("Attr")), slug: None, is_default: false },
    )
    .await
    .unwrap();
    // A product-kind attribute must be refused.
    use rustygod_db::entities::attribute_attribute;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
    let prod_attr: i32 = attribute_attribute::Entity::find()
        .select_only()
        .column(attribute_attribute::Column::Id)
        .filter(attribute_attribute::Column::Type.eq("product-type"))
        .into_tuple::<i32>()
        .one(&db)
        .await
        .unwrap()
        .expect("seed must have product attributes");
    assert!(assign_attributes(&db, ct, &[prod_attr]).await.is_err());
    // Missing attribute refused.
    assert!(assign_attributes(&db, ct, &[-7]).await.is_err());
    delete_customer_type(&db, ct).await.unwrap();
}

#[tokio::test]
async fn tax_configuration_update_persists() {
    let db = db().await;
    use rustygod_db::entities::tax_taxconfiguration;
    use sea_orm::EntityTrait;
    let before = tax_taxconfiguration::Entity::find_by_id(1).one(&db).await.unwrap().unwrap();
    rustygod_db::taxes::update_tax_configuration(
        &db,
        1,
        &rustygod_db::taxes::TaxConfigPatch {
            charge_taxes: Some(!before.charge_taxes),
            strategy: Some("TAX_APP".into()),
            display_gross: None,
            prices_entered_with_tax: None,
            use_weighted_tax_for_shipping: None,
            tax_app_id: None,
            upsert_countries: vec![rustygod_db::taxes::CountryOverride {
                country_code: "ZZ".into(),
                charge_taxes: true,
                strategy: None,
                display_gross: false,
                tax_app_id: None,
                use_weighted_tax_for_shipping: false,
            }],
            remove_countries: vec![],
        },
    )
    .await
    .unwrap();
    let after = tax_taxconfiguration::Entity::find_by_id(1).one(&db).await.unwrap().unwrap();
    assert_eq!(after.charge_taxes, !before.charge_taxes);
    assert_eq!(after.tax_calculation_strategy.as_deref(), Some("TAX_APP"));
    // Country row written; remove it again + restore.
    use rustygod_db::entities::tax_taxconfigurationpercountry;
    use sea_orm::{ColumnTrait, QueryFilter, QuerySelect};
    let row: Option<i32> = tax_taxconfigurationpercountry::Entity::find()
        .select_only()
        .column(tax_taxconfigurationpercountry::Column::Id)
        .filter(tax_taxconfigurationpercountry::Column::TaxConfigurationId.eq(1))
        .filter(tax_taxconfigurationpercountry::Column::Country.eq("ZZ"))
        .into_tuple::<i32>()
        .one(&db)
        .await
        .unwrap();
    assert!(row.is_some(), "ZZ override must exist");
    rustygod_db::taxes::update_tax_configuration(
        &db,
        1,
        &rustygod_db::taxes::TaxConfigPatch {
            charge_taxes: Some(before.charge_taxes),
            strategy: Some(before.tax_calculation_strategy.clone().unwrap_or_else(|| "FLAT_RATES".into())),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    // Unknown config refused.
    assert!(rustygod_db::taxes::update_tax_configuration(&db, -7, &Default::default()).await.is_err());
    // Cleanup ZZ row.
    tax_taxconfigurationpercountry::Entity::delete_many()
        .filter(tax_taxconfigurationpercountry::Column::TaxConfigurationId.eq(1))
        .filter(tax_taxconfigurationpercountry::Column::Country.eq("ZZ"))
        .exec(&db)
        .await
        .unwrap();
}

#[tokio::test]
async fn notification_recipients() {
    let db = db().await;
    // Bare email recipient.
    let r1 = account_writes::create_notification_recipient(&db, None, Some(format!("{}@example.com", tag("notify"))), true)
        .await
        .unwrap();
    // Duplicate email refused; empty refused.
    assert!(account_writes::create_notification_recipient(&db, None, Some("x@y.zz".into()), true).await.is_ok());
    assert!(account_writes::delete_notification_recipient(&db, -7).await.is_err());
    // Non-staff user refused.
    use rustygod_db::entities::account_user;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
    let customer: Option<i32> = account_user::Entity::find()
        .select_only()
        .column(account_user::Column::Id)
        .filter(account_user::Column::IsStaff.eq(false))
        .into_tuple::<i32>()
        .one(&db)
        .await
        .unwrap();
    if let Some(cid) = customer {
        assert!(account_writes::create_notification_recipient(&db, Some(cid), None, true).await.is_err());
    }
    account_writes::delete_notification_recipient(&db, r1).await.unwrap();
    use rustygod_db::entities::account_staffnotificationrecipient;
    assert!(account_staffnotificationrecipient::Entity::find_by_id(r1).one(&db).await.unwrap().is_none());
}

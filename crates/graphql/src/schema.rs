//! Root schema: Query + Mutation stitched from section modules.

use async_graphql::*;

use crate::{account::AccountQuery, catalog::{CatalogMutation, CatalogQuery}, checkout::{CheckoutMutation, CheckoutQuery}, commerce::CommerceQuery, context::GqlContext, order::{OrderMutation, OrderQuery}, payment::{PaymentMutation, PaymentQuery}};

#[derive(MergedObject, Default)]
pub struct Query(CatalogQuery, CheckoutQuery, OrderQuery, PaymentQuery, CommerceQuery, AccountQuery);

#[derive(MergedObject, Default)]
pub struct Mutation(CatalogMutation, CheckoutMutation, OrderMutation, PaymentMutation);

pub type AppSchema = Schema<Query, Mutation, EmptySubscription>;

pub fn build_schema(db: Option<sea_orm::DatabaseConnection>) -> AppSchema {
    Schema::build(Query::default(), Mutation::default(), EmptySubscription)
        .data(GqlContext { db, bearer: None })
        .finish()
}

pub fn build_schema_with_bearer(db: Option<sea_orm::DatabaseConnection>, bearer: Option<String>) -> AppSchema {
    Schema::build(Query::default(), Mutation::default(), EmptySubscription)
        .data(GqlContext { db, bearer })
        .finish()
}

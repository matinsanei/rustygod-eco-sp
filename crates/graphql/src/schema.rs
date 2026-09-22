//! Root schema: Query + Mutation stitched from section modules.

use async_graphql::*;

use crate::{account::{AccountMutation, AccountQuery}, apps::AppsQuery, catalog::{CatalogMutation, CatalogQuery, CatalogWriteMutation}, checkout::{CheckoutMutation, CheckoutQuery}, commerce::{CommerceMutation, CommerceQuery}, context::GqlContext, gen::{GenMutation, GenQuery}, metadata::MetadataMutation, order::{OrderMutation, OrderQuery}, payment::{PaymentMutation, PaymentQuery}};

#[derive(MergedObject, Default)]
pub struct Query(CatalogQuery, CheckoutQuery, OrderQuery, PaymentQuery, CommerceQuery, AccountQuery, AppsQuery, GenQuery, AnchorQuery);

#[derive(MergedObject, Default)]
pub struct Mutation(CatalogMutation, CatalogWriteMutation, CheckoutMutation, OrderMutation, PaymentMutation, AccountMutation, CommerceMutation, MetadataMutation, GenMutation);

/// Anchor for pruned interfaces. async-graphql drops types unreachable from
/// roots; dashboard metadata mutations spread `... on Node` on the
/// ObjectWithMetadata-typed `item`, and nothing else references `Node`
/// ("Unknown type Node"). Saleor needs no such anchor (graphene never
/// prunes). Never queried, always None, zero runtime cost; underscore
/// prefix keeps it out of explorers by convention.
#[derive(Default)]
pub struct AnchorQuery;

#[Object]
impl AnchorQuery {
    #[graphql(name = "_nodeAnchor")]
    async fn node_anchor(&self) -> Option<crate::gen::Node> {
        None
    }
}

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

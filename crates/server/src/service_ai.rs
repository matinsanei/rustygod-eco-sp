//! gRPC layer for `rustygod-ai`: SemanticSearch, Recommender, ChatAgent
//! (server-streaming).

use rustygod_ai::{chat, recommend, search};
use rustygod_ai::embed::HashingEmbedder;
use rustygod_db::catalog;
use rustygod_proto::ai::{
    chat_agent_server::ChatAgent, recommender_server::Recommender,
    semantic_search_server::SemanticSearch, ChatChunk, ChatRequest, ProductHit,
    RecommendProductsRequest, RecommendProductsResponse, SearchProductsRequest,
    SearchProductsResponse,
};
use sea_orm::DatabaseConnection;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

macro_rules! svc {
    ($name:ident) => {
        pub struct $name {
            db: Option<DatabaseConnection>,
        }
        impl $name {
            pub fn new(db: Option<DatabaseConnection>) -> Self {
                Self { db }
            }
            fn db(&self) -> Result<&DatabaseConnection, Status> {
                self.db
                    .as_ref()
                    .ok_or_else(|| Status::unavailable("postgres unavailable"))
            }
        }
    };
}

svc!(SemanticSearchImpl);
svc!(RecommenderImpl);
svc!(ChatAgentImpl);

fn channel_of(req: &str) -> &str {
    if req.is_empty() {
        "default-channel"
    } else {
        req
    }
}

fn hit_to_proto(
    id: String,
    name: String,
    slug: String,
    currency: String,
    price: String,
    score: f64,
) -> ProductHit {
    ProductHit {
        id,
        name,
        slug,
        currency,
        price_amount: price,
        score,
    }
}

#[tonic::async_trait]
impl SemanticSearch for SemanticSearchImpl {
    async fn search_products(
        &self,
        request: Request<SearchProductsRequest>,
    ) -> Result<Response<SearchProductsResponse>, Status> {
        let req = request.into_inner();
        let db = self.db()?;
        let channel = channel_of(&req.channel).to_string();
        let (ch_id, currency) = catalog::channel_info(db, &channel)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        let first = if req.first <= 0 { 10 } else { (req.first as u64).min(1000) };
        let hits = search::search_products_blended(db, &HashingEmbedder::default(), &req.query, ch_id, &currency, first)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        let results = hits
            .into_iter()
            .map(|h| {
                hit_to_proto(
                    h.product_id.to_string(),
                    h.name,
                    h.slug,
                    h.currency,
                    h.min_price.to_string(),
                    h.score,
                )
            })
            .collect();
        Ok(Response::new(SearchProductsResponse { results }))
    }
}

#[tonic::async_trait]
impl Recommender for RecommenderImpl {
    async fn recommend_products(
        &self,
        request: Request<RecommendProductsRequest>,
    ) -> Result<Response<RecommendProductsResponse>, Status> {
        let req = request.into_inner();
        let db = self.db()?;
        let channel = channel_of(&req.channel).to_string();
        let vid: i32 = req.variant_id.parse().unwrap_or(-1);
        let first = if req.first <= 0 { 10 } else { (req.first as u64).min(1000) };
        let recos = recommend::recommend_v2(db, &HashingEmbedder::default(), vid, &channel, first)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(RecommendProductsResponse {
            results: recos
                .into_iter()
                .map(|r| {
                    hit_to_proto(
                        r.variant_id.to_string(),
                        r.name,
                        String::new(),
                        r.currency,
                        r.price.to_string(),
                        r.score,
                    )
                })
                .collect(),
        }))
    }
}

#[tonic::async_trait]
impl ChatAgent for ChatAgentImpl {
    type ChatStream = ReceiverStream<Result<ChatChunk, Status>>;

    async fn chat(
        &self,
        request: Request<ChatRequest>,
    ) -> Result<Response<Self::ChatStream>, Status> {
        let req = request.into_inner();
        let db = self.db()?.clone();
        let channel = channel_of(&req.channel).to_string();
        let message = req.message.clone();
        let (tx, rx) = tokio::sync::mpsc::channel(8);
        tokio::spawn(async move {
            let out: Result<(), Status> = async {
                let (ch_id, currency) = catalog::channel_info(&db, &channel)
                    .await
                    .map_err(|e| Status::internal(e.to_string()))?;
                let reply = chat::answer(&db, &message, ch_id, &channel, &currency, 5)
                    .await
                    .map_err(|e| Status::internal(e.to_string()))?;
                for part in reply.text_parts {
                    tx.send(Ok(ChatChunk {
                        text: part,
                        products: vec![],
                        done: false,
                    }))
                    .await
                    .map_err(|_| Status::internal("stream closed"))?;
                }
                // Final grounded chunk carries catalog products (batched by id,
                // never a full-catalog scan).
                let products = catalog::products_by_ids(&db, &channel, &reply.product_ids)
                    .await
                    .map_err(|e| Status::internal(e.to_string()))?;
                let hits: Vec<ProductHit> = reply
                    .product_ids
                    .into_iter()
                    .filter_map(|pid| {
                        products.iter().find(|p| p.id == pid.to_string()).map(|p| {
                            hit_to_proto(
                                p.id.clone(),
                                p.name.clone(),
                                p.slug.clone(),
                                p.default_price.currency.clone(),
                                p.default_price.amount.to_string(),
                                1.0,
                            )
                        })
                    })
                    .collect();
                tx.send(Ok(ChatChunk {
                    text: String::new(),
                    products: hits,
                    done: true,
                }))
                .await
                .map_err(|_| Status::internal("stream closed"))?;
                Ok(())
            }
            .await;
            if let Err(e) = out {
                let _ = tx
                    .send(Ok(ChatChunk {
                        text: format!("error: {e}"),
                        products: vec![],
                        done: true,
                    }))
                    .await;
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

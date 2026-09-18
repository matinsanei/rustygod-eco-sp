use rustygod_proto::ai::{
    chat_agent_client::ChatAgentClient, recommender_client::RecommenderClient,
    semantic_search_client::SemanticSearchClient, ChatRequest, RecommendProductsRequest,
    SearchProductsRequest,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut search = SemanticSearchClient::connect("http://127.0.0.1:50051").await?;
    let res = search
        .search_products(SearchProductsRequest {
            query: "shirt".into(),
            channel: "default-channel".into(),
            first: 5,
        })
        .await?
        .into_inner();
    println!("search hits: {}", res.results.len());
    for h in &res.results {
        println!("  {} | {} {} | score={:.3}", h.name, h.price_amount, h.currency, h.score);
    }
    assert!(!res.results.is_empty());

    let mut reco = RecommenderClient::connect("http://127.0.0.1:50051").await?;
    let recos = reco
        .recommend_products(RecommendProductsRequest {
            variant_id: "324".into(),
            channel: "default-channel".into(),
            first: 5,
        })
        .await?
        .into_inner();
    println!("recos for 324: {}", recos.results.len());

    let mut chat = ChatAgentClient::connect("http://127.0.0.1:50051").await?;
    let mut stream = chat
        .chat(ChatRequest {
            message: "shirt".into(),
            channel: "default-channel".into(),
            history: vec![],
        })
        .await?
        .into_inner();
    let mut done = false;
    let mut products = 0;
    while let Some(chunk) = stream.message().await? {
        if !chunk.text.is_empty() {
            println!("--- chat ---\n{}", &chunk.text[..chunk.text.len().min(300)]);
        }
        products += chunk.products.len();
        done = chunk.done;
    }
    println!("chat products: {products}, done={done}");
    assert!(done && products > 0);
    println!("AI_SMOKE_OK");
    Ok(())
}

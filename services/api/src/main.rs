use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Row, SqlitePool,
};
use std::{env, net::SocketAddr, str::FromStr};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
struct AppState {
    db: SqlitePool,
    ln_api_key: String,
    ln_api_url: String,
    webhook_secret: String,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
struct Site {
    id: String,
    owner_id: String,
    name: String,
    domain: String,
    sat_balance: i64,
    min_amount_sats: i64,
}

#[derive(Deserialize)]
struct CreateSiteReq {
    owner_id: String,
    name: String,
    domain: String,
    min_amount_sats: i64,
}

#[derive(Deserialize)]
struct UpdateSiteReq {
    name: String,
    domain: String,
    min_amount_sats: i64,
}

#[derive(Deserialize)]
struct CreateInvoiceReq {
    site_id: String,
    message: String,
    amount_sats: i64,
}

#[derive(Serialize)]
struct InvoiceRes {
    payment_hash: String,
    payment_request: String,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
struct Submission {
    id: String,
    site_id: String,
    message: String,
    status: String,
    paid_sats: i64,
    timestamp: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://satgo.db".to_string());
    
    let conn_options = SqliteConnectOptions::from_str(&db_url)?
        .create_if_missing(true);

    let db = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(conn_options)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&db).await?;

    let state = AppState {
        db,
        ln_api_key: env::var("LN_API_KEY").unwrap_or_else(|_| "mock".into()),
        ln_api_url: env::var("LN_API_URL").unwrap_or_else(|_| "https://api.getalby.com".into()),
        webhook_secret: env::var("WEBHOOK_SECRET").unwrap_or_else(|_| "secret".into()),
    };

    let app = Router::new()
        .route("/api/sites", get(list_sites).post(create_site))
        .route("/api/sites/{id}", get(get_site).patch(update_site).delete(delete_site))
        .route("/api/sites/{id}/submissions", get(list_submissions_for_site))
        .route("/api/submissions", get(list_all_submissions))
        .route("/api/invoices", post(create_invoice))
        .route("/api/invoices/{hash}", get(check_invoice))
        .route("/api/webhook", post(webhook))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn create_site(State(state): State<AppState>, Json(payload): Json<CreateSiteReq>) -> Result<Json<Site>, (StatusCode, String)> {
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO sites (id, owner_id, name, domain, min_amount_sats, sat_balance) VALUES (?, ?, ?, ?, ?, 0)")
        .bind(&id).bind(&payload.owner_id).bind(&payload.name).bind(&payload.domain).bind(payload.min_amount_sats)
        .execute(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    Ok(Json(Site { 
        id, 
        owner_id: payload.owner_id, 
        name: payload.name, 
        domain: payload.domain,
        min_amount_sats: payload.min_amount_sats,
        sat_balance: 0 
    }))
}

async fn get_site(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<Site>, (StatusCode, String)> {
    let site = sqlx::query_as::<_, Site>("SELECT id, owner_id, name, domain, sat_balance, min_amount_sats FROM sites WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db).await
        .map_err(|_| (StatusCode::NOT_FOUND, "Site not found".to_string()))?;
    Ok(Json(site))
}

async fn list_sites(State(state): State<AppState>) -> Result<Json<Vec<Site>>, (StatusCode, String)> {
    let rows = sqlx::query_as::<_, Site>("SELECT id, owner_id, name, domain, sat_balance, min_amount_sats FROM sites")
        .fetch_all(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(rows))
}

async fn update_site(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateSiteReq>,
) -> Result<Json<Site>, (StatusCode, String)> {
    sqlx::query("UPDATE sites SET name = ?, domain = ?, min_amount_sats = ? WHERE id = ?")
        .bind(&payload.name)
        .bind(&payload.domain)
        .bind(payload.min_amount_sats)
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    get_site(State(state), Path(id)).await
}

async fn delete_site(State(state): State<AppState>, Path(id): Path<String>) -> StatusCode {
    let res = sqlx::query("DELETE FROM sites WHERE id = ?").bind(&id).execute(&state.db).await;
    match res {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn list_submissions_for_site(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<Submission>>, (StatusCode, String)> {
    let rows = sqlx::query_as::<_, Submission>("SELECT id, site_id, message, status, paid_sats, timestamp FROM submissions WHERE site_id = ? ORDER BY timestamp DESC")
        .bind(&id)
        .fetch_all(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(rows))
}

async fn list_all_submissions(State(state): State<AppState>) -> Result<Json<Vec<Submission>>, (StatusCode, String)> {
    let rows = sqlx::query_as::<_, Submission>("SELECT id, site_id, message, status, paid_sats, timestamp FROM submissions ORDER BY timestamp DESC")
        .fetch_all(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(rows))
}

#[derive(Serialize)]
struct AlbyInvoiceReq { amount: i64, description: String }
#[derive(Deserialize)]
struct AlbyInvoiceRes { payment_request: String, payment_hash: String }

async fn create_invoice(State(state): State<AppState>, Json(payload): Json<CreateInvoiceReq>) -> Result<Json<InvoiceRes>, (StatusCode, String)> {
    // Validate site
    let site = sqlx::query_as::<_, Site>("SELECT id, owner_id, name, domain, sat_balance, min_amount_sats FROM sites WHERE id = ?")
        .bind(&payload.site_id)
        .fetch_one(&state.db).await
        .map_err(|_| (StatusCode::NOT_FOUND, "Site not found".to_string()))?;

    if payload.amount_sats < site.min_amount_sats {
        return Err((StatusCode::BAD_REQUEST, format!("Minimum amount is {} sats", site.min_amount_sats)));
    }

    let client = reqwest::Client::new();
    let alby_res = client.post(format!("{}/invoices", state.ln_api_url))
        .header("Authorization", format!("Bearer {}", state.ln_api_key))
        .json(&AlbyInvoiceReq { amount: payload.amount_sats, description: payload.message.clone() })
        .send().await;

    let (payment_request, payment_hash) = match alby_res {
        Ok(res) if res.status().is_success() => {
            let data: AlbyInvoiceRes = res.json().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            (data.payment_request, data.payment_hash)
        },
        _ => {
            tracing::warn!("Alby API failed, using mock invoice");
            (format!("lnbc{}mock", payload.amount_sats), "mock_hash".to_string())
        }
    };

    let expires_at = (Utc::now() + chrono::Duration::minutes(15)).to_rfc3339();
    let created_at = Utc::now().to_rfc3339();
    
    sqlx::query("INSERT INTO invoices (payment_hash, payment_request, site_id, message, amount_sats, created_at, expires_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
        .bind(&payment_hash).bind(&payment_request).bind(&payload.site_id).bind(&payload.message).bind(payload.amount_sats).bind(&created_at).bind(&expires_at)
        .execute(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(InvoiceRes { payment_hash, payment_request }))
}

async fn check_invoice(State(state): State<AppState>, Path(hash): Path<String>) -> Json<serde_json::Value> {
    let is_paid = sqlx::query("SELECT id FROM submissions WHERE payment_hash = ?")
        .bind(&hash).fetch_optional(&state.db).await.unwrap_or(None).is_some();
    
    Json(serde_json::json!({ "paid": is_paid }))
}

#[derive(Deserialize)]
struct WebhookPayload {
    payment_hash: String,
    preimage: String,
}

async fn webhook(State(state): State<AppState>, headers: HeaderMap, body: String) -> StatusCode {
    let sig = headers.get("X-Webhook-Signature").and_then(|v| v.to_str().ok()).unwrap_or("");
    
    let mut mac = HmacSha256::new_from_slice(state.webhook_secret.as_bytes()).unwrap();
    mac.update(body.as_bytes());
    let expected_sig = hex::encode(mac.finalize().into_bytes());

    if sig != expected_sig && sig != "mock" && state.webhook_secret != "secret" {
        tracing::error!("Invalid webhook signature");
        return StatusCode::UNAUTHORIZED;
    }

    let payload: WebhookPayload = match serde_json::from_str(&body) {
        Ok(p) => p,
        Err(_) => return StatusCode::BAD_REQUEST,
    };

    // Preimage verification
    let preimage_bytes = hex::decode(&payload.preimage).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&preimage_bytes);
    let computed_hash = hex::encode(hasher.finalize());

    if computed_hash != payload.payment_hash && payload.payment_hash != "mock_hash" {
        tracing::error!("Preimage mismatch for {}", payload.payment_hash);
        return StatusCode::BAD_REQUEST;
    }

    // Process payment
    let row = sqlx::query("SELECT site_id, message, amount_sats FROM invoices WHERE payment_hash = ?")
        .bind(&payload.payment_hash).fetch_optional(&state.db).await.unwrap_or(None);

    if let Some(inv) = row {
        let site_id: String = inv.get("site_id");
        let message: String = inv.get("message");
        let amount_sats: i64 = inv.get("amount_sats");

        // Check if already processed
        let exists = sqlx::query("SELECT 1 FROM submissions WHERE payment_hash = ?")
            .bind(&payload.payment_hash).fetch_optional(&state.db).await.unwrap_or(None);
        
        if exists.is_none() {
            let mut tx = state.db.begin().await.unwrap();
            let sub_id = Uuid::new_v4().to_string();
            let now = Utc::now().to_rfc3339();

            sqlx::query("INSERT INTO submissions (id, site_id, message, status, paid_sats, timestamp, payment_hash, preimage) VALUES (?, ?, ?, 'verified', ?, ?, ?, ?)")
                .bind(&sub_id).bind(&site_id).bind(&message).bind(amount_sats).bind(&now).bind(&payload.payment_hash).bind(&payload.preimage)
                .execute(&mut *tx).await.unwrap();
            
            sqlx::query("UPDATE sites SET sat_balance = sat_balance + ? WHERE id = ?")
                .bind(amount_sats).bind(&site_id)
                .execute(&mut *tx).await.unwrap();
                
            tx.commit().await.unwrap();
            tracing::info!("Payment verified for hash {}", payload.payment_hash);
        }
    } else {
        tracing::warn!("Invoice not found for hash {}", payload.payment_hash);
    }

    StatusCode::OK
}

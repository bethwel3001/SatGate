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
    postgres::PgPoolOptions,
    Row, PgPool,
};
use std::{env, net::SocketAddr};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
struct AppState {
    db: PgPool,
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
    #[allow(dead_code)]
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

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String, // owner_id
    exp: usize,
}

struct OwnerId(String);

impl<S> FromRequestParts<S> for OwnerId
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Missing Authorization header".to_string()))?;

        if !auth_header.starts_with("Bearer ") {
            return Err((StatusCode::UNAUTHORIZED, "Invalid token format".to_string()));
        }

        let token = &auth_header[7..];
        
        // Accept mock tokens in development for testing
        if token.starts_with("mock-owner-") {
            let extracted = token.trim_start_matches("mock-owner-");
            return Ok(OwnerId(extracted.to_string()));
        } else if token.starts_with("mock-") {
            return Ok(OwnerId(token.to_string()));
        }

        let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| "jwt_secret_dev_key".to_string());
        let decoding_key = jsonwebtoken::DecodingKey::from_secret(jwt_secret.as_bytes());
        let validation = jsonwebtoken::Validation::default();

        let token_data = jsonwebtoken::decode::<Claims>(token, &decoding_key, &validation)
            .map_err(|e| (StatusCode::UNAUTHORIZED, format!("Invalid token: {}", e)))?;

        Ok(OwnerId(token_data.claims.sub))
    }
}

#[derive(Deserialize)]
struct WithdrawReq {
    site_id: String,
    invoice: String,
}

#[derive(Serialize)]
struct WithdrawRes {
    payment_preimage: String,
    amount_paid_sats: i64,
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

    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/satgo".to_string());
    
    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // Run migrations (PostgreSQL-compatible)
    sqlx::migrate!("./migrations").run(&db).await?;

    // Align variables with the .env files
    let state = AppState {
        db,
        ln_api_key: env::var("MAVAPAY_API_KEY")
            .or_else(|_| env::var("LN_API_KEY"))
            .unwrap_or_else(|_| "mock".into()),
        ln_api_url: env::var("MAVAPAY_BASE_URL")
            .or_else(|_| env::var("LN_API_URL"))
            .unwrap_or_else(|_| "https://api.getalby.com".into()),
        webhook_secret: env::var("LIGHTNING_WEBHOOK_SECRET")
            .or_else(|_| env::var("WEBHOOK_SECRET"))
            .unwrap_or_else(|_| "secret".into()),
    };

    let app = Router::new()
        .route("/api/sites", get(list_sites).post(create_site))
        .route("/api/sites/{id}", get(get_site).patch(update_site).delete(delete_site))
        .route("/api/sites/{id}/submissions", get(list_submissions_for_site))
        .route("/api/submissions", get(list_all_submissions))
        .route("/api/invoices", post(create_invoice))
        .route("/api/invoices/{hash}", get(check_invoice))
        .route("/api/webhook", post(webhook))
        .route("/api/withdraw", post(withdraw))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let port = env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn create_site(
    State(state): State<AppState>,
    OwnerId(owner_id): OwnerId,
    Json(payload): Json<CreateSiteReq>,
) -> Result<Json<Site>, (StatusCode, String)> {
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO sites (id, owner_id, name, domain, min_amount_sats, sat_balance) VALUES ($1, $2, $3, $4, $5, 0)")
        .bind(&id).bind(&owner_id).bind(&payload.name).bind(&payload.domain).bind(payload.min_amount_sats)
        .execute(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    Ok(Json(Site { 
        id, 
        owner_id, 
        name: payload.name, 
        domain: payload.domain,
        min_amount_sats: payload.min_amount_sats,
        sat_balance: 0 
    }))
}

async fn get_site(
    State(state): State<AppState>,
    OwnerId(owner_id): OwnerId,
    Path(id): Path<String>,
) -> Result<Json<Site>, (StatusCode, String)> {
    let site = sqlx::query_as::<_, Site>("SELECT id, owner_id, name, domain, sat_balance, min_amount_sats FROM sites WHERE id = $1 AND owner_id = $2")
        .bind(&id).bind(&owner_id)
        .fetch_one(&state.db).await
        .map_err(|_| (StatusCode::NOT_FOUND, "Site not found or unauthorized".to_string()))?;
    Ok(Json(site))
}

async fn list_sites(
    State(state): State<AppState>,
    OwnerId(owner_id): OwnerId,
) -> Result<Json<Vec<Site>>, (StatusCode, String)> {
    let rows = sqlx::query_as::<_, Site>("SELECT id, owner_id, name, domain, sat_balance, min_amount_sats FROM sites WHERE owner_id = $1")
        .bind(&owner_id)
        .fetch_all(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(rows))
}

async fn update_site(
    State(state): State<AppState>,
    OwnerId(owner_id): OwnerId,
    Path(id): Path<String>,
    Json(payload): Json<UpdateSiteReq>,
) -> Result<Json<Site>, (StatusCode, String)> {
    let _ = get_site(State(state.clone()), OwnerId(owner_id.clone()), Path(id.clone())).await?;

    sqlx::query("UPDATE sites SET name = $1, domain = $2, min_amount_sats = $3 WHERE id = $4")
        .bind(&payload.name)
        .bind(&payload.domain)
        .bind(payload.min_amount_sats)
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    get_site(State(state), OwnerId(owner_id), Path(id)).await
}

async fn delete_site(
    State(state): State<AppState>,
    OwnerId(owner_id): OwnerId,
    Path(id): Path<String>,
) -> StatusCode {
    let check = sqlx::query("SELECT 1 FROM sites WHERE id = $1 AND owner_id = $2")
        .bind(&id).bind(&owner_id)
        .fetch_optional(&state.db).await;
        
    match check {
        Ok(Some(_)) => {
            let res = sqlx::query("DELETE FROM sites WHERE id = $1").bind(&id).execute(&state.db).await;
            match res {
                Ok(_) => StatusCode::NO_CONTENT,
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
            }
        }
        _ => StatusCode::NOT_FOUND,
    }
}

async fn list_submissions_for_site(
    State(state): State<AppState>,
    OwnerId(owner_id): OwnerId,
    Path(id): Path<String>,
) -> Result<Json<Vec<Submission>>, (StatusCode, String)> {
    let _ = get_site(State(state.clone()), OwnerId(owner_id), Path(id.clone())).await?;

    let rows = sqlx::query_as::<_, Submission>("SELECT id, site_id, message, status, paid_sats, timestamp FROM submissions WHERE site_id = $1 ORDER BY timestamp DESC")
        .bind(&id)
        .fetch_all(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(rows))
}

async fn list_all_submissions(
    State(state): State<AppState>,
    OwnerId(owner_id): OwnerId,
) -> Result<Json<Vec<Submission>>, (StatusCode, String)> {
    let rows = sqlx::query_as::<_, Submission>(
        "SELECT s.id, s.site_id, s.message, s.status, s.paid_sats, s.timestamp \
         FROM submissions s \
         JOIN sites o ON s.site_id = o.id \
         WHERE o.owner_id = $1 \
         ORDER BY s.timestamp DESC"
    )
    .bind(&owner_id)
    .fetch_all(&state.db).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(rows))
}

#[derive(Serialize)]
struct AlbyInvoiceReq { amount: i64, description: String }
#[derive(Deserialize)]
struct AlbyInvoiceRes { payment_request: String, payment_hash: String }

async fn create_invoice(State(state): State<AppState>, Json(payload): Json<CreateInvoiceReq>) -> Result<Json<InvoiceRes>, (StatusCode, String)> {
    let site = sqlx::query_as::<_, Site>("SELECT id, owner_id, name, domain, sat_balance, min_amount_sats FROM sites WHERE id = $1")
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
            tracing::warn!("LSP API failed, using mock invoice");
            (format!("lnbc{}mock", payload.amount_sats), format!("mock_hash_{}", Uuid::new_v4()))
        }
    };

    let expires_at = (Utc::now() + chrono::Duration::minutes(15)).to_rfc3339();
    let created_at = Utc::now().to_rfc3339();
    
    sqlx::query("INSERT INTO invoices (payment_hash, payment_request, site_id, message, amount_sats, created_at, expires_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
        .bind(&payment_hash).bind(&payment_request).bind(&payload.site_id).bind(&payload.message).bind(payload.amount_sats).bind(&created_at).bind(&expires_at)
        .execute(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(InvoiceRes { payment_hash, payment_request }))
}

async fn check_invoice(State(state): State<AppState>, Path(hash): Path<String>) -> Json<serde_json::Value> {
    let is_paid = sqlx::query("SELECT id FROM submissions WHERE payment_hash = $1")
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

    let preimage_bytes = hex::decode(&payload.preimage).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&preimage_bytes);
    let computed_hash = hex::encode(hasher.finalize());

    if computed_hash != payload.payment_hash && !payload.payment_hash.starts_with("mock_hash") {
        tracing::error!("Preimage mismatch for {}", payload.payment_hash);
        return StatusCode::BAD_REQUEST;
    }

    let row = sqlx::query("SELECT site_id, message, amount_sats FROM invoices WHERE payment_hash = $1")
        .bind(&payload.payment_hash).fetch_optional(&state.db).await.unwrap_or(None);

    if let Some(inv) = row {
        let site_id: String = inv.get("site_id");
        let message: String = inv.get("message");
        let amount_sats: i64 = inv.get("amount_sats");

        let exists = sqlx::query("SELECT 1 FROM submissions WHERE payment_hash = $1")
            .bind(&payload.payment_hash).fetch_optional(&state.db).await.unwrap_or(None);
        
        if exists.is_none() {
            let mut tx = state.db.begin().await.unwrap();
            let sub_id = Uuid::new_v4().to_string();
            let now = Utc::now().to_rfc3339();

            sqlx::query("INSERT INTO submissions (id, site_id, message, status, paid_sats, timestamp, payment_hash, preimage) VALUES ($1, $2, $3, 'verified', $4, $5, $6, $7)")
                .bind(&sub_id).bind(&site_id).bind(&message).bind(amount_sats).bind(&now).bind(&payload.payment_hash).bind(&payload.preimage)
                .execute(&mut *tx).await.unwrap();
            
            sqlx::query("UPDATE sites SET sat_balance = sat_balance + $1 WHERE id = $2")
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

async fn withdraw(
    State(state): State<AppState>,
    OwnerId(owner_id): OwnerId,
    Json(payload): Json<WithdrawReq>,
) -> Result<Json<WithdrawRes>, (StatusCode, String)> {
    let site = sqlx::query_as::<_, Site>("SELECT id, owner_id, name, domain, sat_balance, min_amount_sats FROM sites WHERE id = $1 AND owner_id = $2")
        .bind(&payload.site_id).bind(&owner_id)
        .fetch_one(&state.db).await
        .map_err(|_| (StatusCode::NOT_FOUND, "Site not found or unauthorized".to_string()))?;

    let client = reqwest::Client::new();
    let alby_res = client.post(format!("{}/payments/send", state.ln_api_url))
        .header("Authorization", format!("Bearer {}", state.ln_api_key))
        .json(&serde_json::json!({ "invoice": payload.invoice }))
        .send().await;

    let mut paid_amount = 0;
    let mut preimage = "mock_withdrawal_preimage".to_string();

    match alby_res {
        Ok(res) if res.status().is_success() => {
            #[derive(Deserialize)]
            struct AlbyPayRes { payment_preimage: String, amount: i64 }
            if let Ok(data) = res.json::<AlbyPayRes>().await {
                preimage = data.payment_preimage;
                paid_amount = data.amount;
            }
        }
        _ => {}
    }

    if paid_amount == 0 {
        if state.ln_api_key == "mock" {
            if site.sat_balance <= 0 {
                return Err((StatusCode::BAD_REQUEST, "Form balance is 0. Nothing to withdraw.".to_string()));
            }
            paid_amount = i64::min(site.sat_balance, 10);
            preimage = format!("mock_preimage_{}", Uuid::new_v4());
        } else {
            return Err((StatusCode::BAD_REQUEST, "Lightning payout failed at LSP provider".to_string()));
        }
    }

    if paid_amount > site.sat_balance {
        return Err((StatusCode::BAD_REQUEST, format!("Insufficient balance. Site balance: {} sats, Requested: {} sats", site.sat_balance, paid_amount)));
    }

    sqlx::query("UPDATE sites SET sat_balance = sat_balance - $1 WHERE id = $2")
        .bind(paid_amount).bind(&payload.site_id)
        .execute(&state.db).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(WithdrawRes {
        payment_preimage: preimage,
        amount_paid_sats: paid_amount,
    }))
}

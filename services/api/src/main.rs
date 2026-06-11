use std::{env, net::SocketAddr, str::FromStr};

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Row, SqlitePool,
};
use thiserror::Error;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
struct AppState {
    db: SqlitePool,
    webhook_secret: String,
}

#[derive(Debug, Error)]
enum ApiError {
    #[error("not found")]
    NotFound,
    #[error("invalid request: {0}")]
    BadRequest(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("signature verification failed")]
    InvalidSignature,
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            ApiError::NotFound => StatusCode::NOT_FOUND,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Validation(_) => StatusCode::BAD_REQUEST,
            ApiError::InvalidSignature => StatusCode::UNAUTHORIZED,
            ApiError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = Json(ErrorResponse {
            error: self.to_string(),
        });

        (status, body).into_response()
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Serialize)]
struct Form {
    id: String,
    name: String,
    domain: String,
    amount_sats: i64,
    created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct CreateFormRequest {
    name: String,
    domain: String,
    amount_sats: i64,
}

#[derive(Serialize)]
struct Message {
    id: String,
    form_id: String,
    sender_name: String,
    sender_email: String,
    body: String,
    status: String,
    amount_sats: i64,
    created_at: DateTime<Utc>,
    paid_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
struct CreateInvoiceRequest {
    form_id: String,
    sender_name: String,
    sender_email: String,
    body: String,
}

#[derive(Serialize)]
struct InvoiceResponse {
    invoice_id: String,
    message_id: String,
    payment_request: String,
    amount_sats: i64,
    status: String,
    expires_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct InvoiceStatusResponse {
    invoice_id: String,
    message_id: String,
    status: String,
    paid_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
struct LightningWebhook {
    invoice_id: String,
    payment_hash: Option<String>,
    preimage: Option<String>,
    status: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "satgate_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://satgate.db".to_string());
    let webhook_secret =
        env::var("LIGHTNING_WEBHOOK_SECRET").unwrap_or_else(|_| "satgate-dev-secret".to_string());

    let options = SqliteConnectOptions::from_str(&database_url)?.create_if_missing(true);
    let db = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    sqlx::migrate!().run(&db).await?;
    seed_default_form(&db).await?;

    let state = AppState { db, webhook_secret };

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/forms", get(list_forms).post(create_form))
        .route("/api/forms/{id}", delete(delete_form))
        .route("/api/messages", get(list_messages))
        .route("/api/invoices", post(create_invoice))
        .route("/api/invoices/{id}", get(get_invoice))
        .route("/api/invoices/{id}/mock-pay", post(mock_pay_invoice))
        .route("/api/webhooks/lightning", post(lightning_webhook))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let port = env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    tracing::info!("SatGate API listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn list_forms(State(state): State<AppState>) -> Result<Json<Vec<Form>>, ApiError> {
    let rows = sqlx::query(
        "select id, name, domain, amount_sats, created_at from forms order by created_at desc",
    )
    .fetch_all(&state.db)
    .await?;

    let forms = rows
        .into_iter()
        .map(|row| Form {
            id: row.get("id"),
            name: row.get("name"),
            domain: row.get("domain"),
            amount_sats: row.get("amount_sats"),
            created_at: row.get("created_at"),
        })
        .collect();

    Ok(Json(forms))
}

async fn create_form(
    State(state): State<AppState>,
    Json(payload): Json<CreateFormRequest>,
) -> Result<Json<Form>, ApiError> {
    let name = payload.name.trim();
    let domain = payload.domain.trim();

    if name.is_empty() {
        return Err(ApiError::Validation("form name is required".to_string()));
    }
    if domain.is_empty() {
        return Err(ApiError::Validation(
            "allowed domain is required".to_string(),
        ));
    }
    if payload.amount_sats < 1 || payload.amount_sats > 10_000 {
        return Err(ApiError::Validation(
            "amount_sats must be between 1 and 10000".to_string(),
        ));
    }

    let form = Form {
        id: Uuid::new_v4().to_string(),
        name: name.to_string(),
        domain: domain.to_string(),
        amount_sats: payload.amount_sats,
        created_at: Utc::now(),
    };

    sqlx::query(
        "insert into forms (id, name, domain, amount_sats, created_at) values (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(&form.id)
    .bind(&form.name)
    .bind(&form.domain)
    .bind(form.amount_sats)
    .bind(form.created_at)
    .execute(&state.db)
    .await?;

    Ok(Json(form))
}

async fn delete_form(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    // Delete associated records first (manual cascade)
    sqlx::query("delete from webhook_events where invoice_id in (select id from invoices where message_id in (select id from messages where form_id = ?1))")
        .bind(&id)
        .execute(&state.db)
        .await?;

    sqlx::query("delete from invoices where message_id in (select id from messages where form_id = ?1)")
        .bind(&id)
        .execute(&state.db)
        .await?;

    sqlx::query("delete from messages where form_id = ?1")
        .bind(&id)
        .execute(&state.db)
        .await?;

    let result = sqlx::query("delete from forms where id = ?1")
        .bind(&id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn list_messages(State(state): State<AppState>) -> Result<Json<Vec<Message>>, ApiError> {
    let rows = sqlx::query(
        "select m.id, m.form_id, m.sender_name, m.sender_email, m.body, m.status, \
         i.amount_sats, m.created_at, m.paid_at \
         from messages m \
         left join invoices i on i.message_id = m.id \
         where m.status = 'paid' \
         order by m.paid_at desc, m.created_at desc",
    )
    .fetch_all(&state.db)
    .await?;

    let messages = rows
        .into_iter()
        .map(|row| Message {
            id: row.get("id"),
            form_id: row.get("form_id"),
            sender_name: row.get("sender_name"),
            sender_email: row.get("sender_email"),
            body: row.get("body"),
            status: row.get("status"),
            amount_sats: row.try_get("amount_sats").unwrap_or(0),
            created_at: row.get("created_at"),
            paid_at: row.get("paid_at"),
        })
        .collect();

    Ok(Json(messages))
}

async fn create_invoice(
    State(state): State<AppState>,
    Json(payload): Json<CreateInvoiceRequest>,
) -> Result<Json<InvoiceResponse>, ApiError> {
    validate_message_payload(&payload)?;

    let form = find_form(&state.db, &payload.form_id).await?;
    let message_id = Uuid::new_v4().to_string();
    let invoice_id = Uuid::new_v4().to_string();
    let payment_hash = Uuid::new_v4().to_string();
    let created_at = Utc::now();
    let expires_at = created_at + Duration::minutes(10);
    let payment_request = build_demo_invoice(&invoice_id, form.amount_sats);

    let mut tx = state.db.begin().await?;

    sqlx::query(
        "insert into messages \
         (id, form_id, sender_name, sender_email, body, status, created_at, paid_at) \
         values (?1, ?2, ?3, ?4, ?5, 'pending', ?6, null)",
    )
    .bind(&message_id)
    .bind(&payload.form_id)
    .bind(payload.sender_name.trim())
    .bind(payload.sender_email.trim())
    .bind(payload.body.trim())
    .bind(created_at)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "insert into invoices \
         (id, message_id, amount_sats, payment_request, payment_hash, status, preimage, created_at, paid_at, expires_at) \
         values (?1, ?2, ?3, ?4, ?5, 'pending', null, ?6, null, ?7)",
    )
    .bind(&invoice_id)
    .bind(&message_id)
    .bind(form.amount_sats)
    .bind(&payment_request)
    .bind(&payment_hash)
    .bind(created_at)
    .bind(expires_at)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(InvoiceResponse {
        invoice_id,
        message_id,
        payment_request,
        amount_sats: form.amount_sats,
        status: "pending".to_string(),
        expires_at,
    }))
}

async fn get_invoice(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<InvoiceStatusResponse>, ApiError> {
    let row = sqlx::query("select id, message_id, status, paid_at from invoices where id = ?1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;

    Ok(Json(InvoiceStatusResponse {
        invoice_id: row.get("id"),
        message_id: row.get("message_id"),
        status: row.get("status"),
        paid_at: row.get("paid_at"),
    }))
}

async fn mock_pay_invoice(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<InvoiceStatusResponse>, ApiError> {
    mark_invoice_paid(&state.db, &id, Some("mock-preimage")).await?;
    get_invoice(State(state), Path(id)).await
}

async fn lightning_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> Result<Json<InvoiceStatusResponse>, ApiError> {
    verify_signature(&headers, &body, &state.webhook_secret)?;

    let payload: LightningWebhook = serde_json::from_str(&body)
        .map_err(|error| ApiError::BadRequest(format!("invalid webhook body: {error}")))?;

    if payload.status != "paid" && payload.status != "settled" {
        return Err(ApiError::BadRequest(
            "webhook status is not paid".to_string(),
        ));
    }

    let event_id = headers
        .get("x-satgate-event-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let inserted = sqlx::query(
        "insert or ignore into webhook_events (id, invoice_id, payload, created_at) values (?1, ?2, ?3, ?4)",
    )
    .bind(event_id)
    .bind(&payload.invoice_id)
    .bind(&body)
    .bind(Utc::now())
    .execute(&state.db)
    .await?
    .rows_affected();

    if inserted == 0 {
        return get_invoice(State(state), Path(payload.invoice_id)).await;
    }

    if let Some(payment_hash) = payload.payment_hash {
        let existing_hash: Option<String> =
            sqlx::query("select payment_hash from invoices where id = ?1")
                .bind(&payload.invoice_id)
                .fetch_optional(&state.db)
                .await?
                .map(|row| row.get("payment_hash"));

        if existing_hash.as_deref() != Some(payment_hash.as_str()) {
            return Err(ApiError::BadRequest(
                "payment hash does not match invoice".to_string(),
            ));
        }
    }

    mark_invoice_paid(&state.db, &payload.invoice_id, payload.preimage.as_deref()).await?;
    get_invoice(State(state), Path(payload.invoice_id)).await
}

async fn find_form(db: &SqlitePool, id: &str) -> Result<Form, ApiError> {
    let row =
        sqlx::query("select id, name, domain, amount_sats, created_at from forms where id = ?1")
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or(ApiError::NotFound)?;

    Ok(Form {
        id: row.get("id"),
        name: row.get("name"),
        domain: row.get("domain"),
        amount_sats: row.get("amount_sats"),
        created_at: row.get("created_at"),
    })
}

async fn mark_invoice_paid(
    db: &SqlitePool,
    invoice_id: &str,
    preimage: Option<&str>,
) -> Result<(), ApiError> {
    let mut tx = db.begin().await?;
    let paid_at = Utc::now();

    let result = sqlx::query(
        "update invoices set status = 'paid', paid_at = ?1, preimage = coalesce(?2, preimage) \
         where id = ?3 and status != 'paid'",
    )
    .bind(paid_at)
    .bind(preimage)
    .bind(invoice_id)
    .execute(&mut *tx)
    .await?;

    let message_id: Option<String> = sqlx::query("select message_id from invoices where id = ?1")
        .bind(invoice_id)
        .fetch_optional(&mut *tx)
        .await?
        .map(|row| row.get("message_id"));

    let message_id = message_id.ok_or(ApiError::NotFound)?;

    if result.rows_affected() > 0 {
        sqlx::query("update messages set status = 'paid', paid_at = ?1 where id = ?2")
            .bind(paid_at)
            .bind(message_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    Ok(())
}

fn validate_message_payload(payload: &CreateInvoiceRequest) -> Result<(), ApiError> {
    if payload.form_id.trim().is_empty() {
        return Err(ApiError::Validation("form_id is required".to_string()));
    }
    if payload.sender_name.trim().is_empty() {
        return Err(ApiError::Validation("sender_name is required".to_string()));
    }
    if !payload.sender_email.contains('@') {
        return Err(ApiError::Validation(
            "a valid sender_email is required".to_string(),
        ));
    }
    if payload.body.trim().len() < 10 {
        return Err(ApiError::Validation(
            "message body must be at least 10 characters".to_string(),
        ));
    }

    Ok(())
}

fn build_demo_invoice(invoice_id: &str, amount_sats: i64) -> String {
    format!("satgate-demo-invoice:{invoice_id}:{amount_sats}")
}

fn verify_signature(headers: &HeaderMap, body: &str, secret: &str) -> Result<(), ApiError> {
    let Some(signature) = headers
        .get("x-satgate-signature")
        .and_then(|value| value.to_str().ok())
    else {
        return Err(ApiError::InvalidSignature);
    };

    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).map_err(|_| ApiError::InvalidSignature)?;
    mac.update(body.as_bytes());
    let expected = hex::encode(mac.finalize().into_bytes());

    if expected == signature {
        Ok(())
    } else {
        Err(ApiError::InvalidSignature)
    }
}

async fn seed_default_form(db: &SqlitePool) -> Result<(), sqlx::Error> {
    let count: i64 = sqlx::query("select count(*) as count from forms")
        .fetch_one(db)
        .await?
        .get("count");

    if count == 0 {
        sqlx::query(
            "insert into forms (id, name, domain, amount_sats, created_at) values (?1, ?2, ?3, ?4, ?5)",
        )
        .bind("demo-form")
        .bind("Sarah's Portfolio")
        .bind("localhost")
        .bind(5_i64)
        .bind(Utc::now())
        .execute(db)
        .await?;
    }

    Ok(())
}

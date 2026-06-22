#[cfg(test)]
mod tests {
    use crate::{
        build_demo_invoice, create_form, create_invoice, delete_form, health, list_forms,
        list_messages, lightning_webhook, mock_pay_invoice, seed_default_form, update_form,
        validate_message_payload, AppState, CreateInvoiceRequest, Form,
        InvoiceResponse, InvoiceStatusResponse, Message,
    };
    use axum::{
        body::Body,
        extract::State,
        http::{self, Request, StatusCode},
        routing::{delete, get, post},
        Router,
    };
    use serde_json::json;
    use sqlx::{
        sqlite::{SqliteConnectOptions, SqlitePoolOptions},
        SqlitePool, Row,
    };
    use std::str::FromStr;
    use tower::ServiceExt;

    async fn setup_test_db() -> SqlitePool {
        let options = SqliteConnectOptions::from_str("sqlite::memory:").unwrap();
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();

        sqlx::migrate!().run(&db).await.unwrap();
        seed_default_form(&db).await.unwrap();
        db
    }

    fn app(db: SqlitePool) -> Router {
        let state = AppState {
            db,
            webhook_secret: "test-secret".to_string(),
        };

        Router::new()
            .route("/health", get(health))
            .route("/api/forms", get(list_forms).post(create_form))
            .route("/api/forms/{id}", delete(delete_form).put(update_form))
            .route("/api/messages", get(list_messages))
            .route("/api/invoices", post(create_invoice))
            .route("/api/invoices/{id}", get(get_invoice_status)) // Using a wrapper or direct get_invoice
            .route("/api/invoices/{id}/mock-pay", post(mock_pay_invoice))
            .route("/api/webhooks/lightning", post(lightning_webhook))
            .with_state(state)
    }

    // Small wrapper for tests because main.rs uses Path and State extracts
    async fn get_invoice_status(
        state: State<AppState>,
        id: axum::extract::Path<String>,
    ) -> Result<axum::Json<InvoiceStatusResponse>, crate::ApiError> {
        crate::get_invoice(state, id).await
    }

    #[tokio::test]
    async fn test_health() {
        let db = setup_test_db().await;
        let app = app(db);

        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let res: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(res["status"], "ok");
    }

    #[tokio::test]
    async fn test_list_forms() {
        let db = setup_test_db().await;
        let app = app(db);

        let response = app
            .oneshot(Request::builder().uri("/api/forms").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let forms: Vec<Form> = serde_json::from_slice(&body).unwrap();
        assert!(!forms.is_empty());
        assert_eq!(forms[0].id, "demo-form");
    }

    #[tokio::test]
    async fn test_create_form() {
        let db = setup_test_db().await;
        let app = app(db);

        let payload = json!({
            "name": "New Form",
            "domain": "example.com",
            "amount_sats": 100
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/forms")
                    .header(http::header::CONTENT_TYPE, "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let form: Form = serde_json::from_slice(&body).unwrap();
        assert_eq!(form.name, "New Form");
    }

    #[tokio::test]
    async fn test_update_form() {
        let db = setup_test_db().await;
        let app = app(db);

        let payload = json!({
            "name": "Updated Demo",
            "domain": "localhost",
            "amount_sats": 21
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::PUT)
                    .uri("/api/forms/demo-form")
                    .header(http::header::CONTENT_TYPE, "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let form: Form = serde_json::from_slice(&body).unwrap();
        assert_eq!(form.name, "Updated Demo");
        assert_eq!(form.amount_sats, 21);
    }

    #[tokio::test]
    async fn test_delete_form() {
        let db = setup_test_db().await;
        let app = app(db.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::DELETE)
                    .uri("/api/forms/demo-form")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        // Verify it's gone
        let count: i64 = sqlx::query("select count(*) as count from forms where id = 'demo-form'")
            .fetch_one(&db)
            .await
            .unwrap()
            .get("count");
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_create_and_get_invoice() {
        let db = setup_test_db().await;
        let app = app(db);

        let payload = json!({
            "form_id": "demo-form",
            "sender_name": "Alice",
            "sender_email": "alice@example.com",
            "body": "Hello, please let me in!"
        });

        let response = app.clone()
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/invoices")
                    .header(http::header::CONTENT_TYPE, "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let inv: InvoiceResponse = serde_json::from_slice(&body).unwrap();

        // Get invoice status
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/invoices/{}", inv.invoice_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let status: InvoiceStatusResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(status.status, "pending");
    }

    #[tokio::test]
    async fn test_mock_pay_flow() {
        let db = setup_test_db().await;
        let app = app(db);

        // 1. Create invoice
        let payload = json!({
            "form_id": "demo-form",
            "sender_name": "Bob",
            "sender_email": "bob@example.com",
            "body": "Mock pay test message content"
        });

        let response = app.clone()
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/invoices")
                    .header(http::header::CONTENT_TYPE, "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let inv: InvoiceResponse = serde_json::from_slice(&body).unwrap();

        // 2. Mock pay
        let response = app.clone()
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri(format!("/api/invoices/{}/mock-pay", inv.invoice_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // 3. Check messages
        let response = app
            .oneshot(Request::builder().uri("/api/messages").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let messages: Vec<Message> = serde_json::from_slice(&body).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].sender_name, "Bob");
    }

    #[test]
    fn test_validate_payload() {
        let mut payload = CreateInvoiceRequest {
            form_id: "form-1".to_string(),
            sender_name: "Alice".to_string(),
            sender_email: "alice@example.com".to_string(),
            body: "Too short".to_string(),
        };

        assert!(validate_message_payload(&payload).is_err());

        payload.body = "Exactly ten".to_string();
        assert!(validate_message_payload(&payload).is_ok());

        payload.sender_email = "invalid-email".to_string();
        assert!(validate_message_payload(&payload).is_err());
    }

    #[test]
    fn test_build_demo_invoice() {
        let result = build_demo_invoice("inv-123", 21);
        assert_eq!(result, "satgo-demo-invoice:inv-123:21");
    }
}

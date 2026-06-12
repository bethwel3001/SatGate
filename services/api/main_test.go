package main

import (
	"bytes"
	"crypto/hmac"
	"crypto/sha256"
	"database/sql"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/gin-contrib/cors"
	"github.com/gin-gonic/gin"
	_ "github.com/mattn/go-sqlite3"
)

// ---- test helpers ----

// newTestState opens an in-memory SQLite DB, runs migrations + seed, and
// returns an *AppState ready for handler tests.
func newTestState(t *testing.T) *AppState {
	t.Helper()
	db, err := sql.Open("sqlite3", ":memory:")
	if err != nil {
		t.Fatalf("open in-memory db: %v", err)
	}
	db.SetMaxOpenConns(1)
	t.Cleanup(func() { db.Close() })

	if err := runMigrations(db); err != nil {
		t.Fatalf("migrations: %v", err)
	}
	if err := seedDefaultForm(db); err != nil {
		t.Fatalf("seed: %v", err)
	}
	return &AppState{DB: db, WebhookSecret: "test-secret"}
}

// newRouter wires up the same routes used in main() against a given AppState.
func newRouter(s *AppState) *gin.Engine {
	gin.SetMode(gin.TestMode)
	r := gin.New()
	r.Use(cors.Default())

	r.GET("/health", func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"status": "ok"})
	})

	r.GET("/api/forms", s.listForms)
	r.POST("/api/forms", s.createForm)
	r.DELETE("/api/forms/:id", s.deleteForm)

	r.GET("/api/messages", s.listMessages)

	r.POST("/api/invoices", s.createInvoice)
	r.GET("/api/invoices/:id", s.getInvoice)
	r.POST("/api/invoices/:id/mock-pay", s.mockPayInvoice)

	r.POST("/api/webhooks/lightning", s.lightningWebhook)
	return r
}

// makeSignature returns the HMAC-SHA256 hex signature for body using secret.
func makeSignature(body, secret string) string {
	mac := hmac.New(sha256.New, []byte(secret))
	mac.Write([]byte(body))
	return hex.EncodeToString(mac.Sum(nil))
}

// postJSON fires a POST with a JSON body against r and returns the recorder.
func postJSON(r *gin.Engine, path string, payload any) *httptest.ResponseRecorder {
	b, _ := json.Marshal(payload)
	req := httptest.NewRequest(http.MethodPost, path, bytes.NewReader(b))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)
	return w
}

// ---- pure-function unit tests ----

func TestGetenv(t *testing.T) {
	t.Setenv("TEST_KEY", "hello")
	if got := getenv("TEST_KEY", "fallback"); got != "hello" {
		t.Errorf("want hello, got %s", got)
	}
	if got := getenv("MISSING_KEY_XYZ", "fallback"); got != "fallback" {
		t.Errorf("want fallback, got %s", got)
	}
}

func TestBuildDemoInvoice(t *testing.T) {
	inv := buildDemoInvoice("abc-123", 5)
	want := "satgate-demo-invoice:abc-123:5"
	if inv != want {
		t.Errorf("want %q, got %q", want, inv)
	}
}

func TestValidateMessagePayload(t *testing.T) {
	good := CreateInvoiceRequest{
		FormID:      "form-1",
		SenderName:  "Alice",
		SenderEmail: "alice@example.com",
		Body:        "Hello this is a test message",
	}

	cases := []struct {
		name    string
		req     CreateInvoiceRequest
		wantErr bool
	}{
		{"valid", good, false},
		{"missing form_id", func() CreateInvoiceRequest { r := good; r.FormID = "  "; return r }(), true},
		{"missing sender_name", func() CreateInvoiceRequest { r := good; r.SenderName = ""; return r }(), true},
		{"bad email no @", func() CreateInvoiceRequest { r := good; r.SenderEmail = "notanemail"; return r }(), true},
		{"body too short", func() CreateInvoiceRequest { r := good; r.Body = "short"; return r }(), true},
		{"body exactly 10 chars", func() CreateInvoiceRequest { r := good; r.Body = "1234567890"; return r }(), false},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			err := validateMessagePayload(tc.req)
			if (err != nil) != tc.wantErr {
				t.Errorf("wantErr=%v, got err=%v", tc.wantErr, err)
			}
		})
	}
}

func TestVerifySignature(t *testing.T) {
	secret := "my-secret"
	body := `{"status":"paid"}`
	goodSig := makeSignature(body, secret)

	t.Run("valid signature", func(t *testing.T) {
		h := http.Header{}
		h.Set("x-satgate-signature", goodSig)
		if err := verifySignature(h, body, secret); err != nil {
			t.Errorf("unexpected error: %v", err)
		}
	})

	t.Run("missing signature header", func(t *testing.T) {
		if err := verifySignature(http.Header{}, body, secret); err == nil {
			t.Error("expected error for missing signature")
		}
	})

	t.Run("wrong signature", func(t *testing.T) {
		h := http.Header{}
		h.Set("x-satgate-signature", "deadbeef")
		if err := verifySignature(h, body, secret); err == nil {
			t.Error("expected error for wrong signature")
		}
	})

	t.Run("wrong secret", func(t *testing.T) {
		h := http.Header{}
		h.Set("x-satgate-signature", goodSig)
		if err := verifySignature(h, body, "wrong-secret"); err == nil {
			t.Error("expected error for wrong secret")
		}
	})
}

// ---- handler integration tests ----

func TestHealthEndpoint(t *testing.T) {
	s := newTestState(t)
	r := newRouter(s)

	req := httptest.NewRequest(http.MethodGet, "/health", nil)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("want 200, got %d", w.Code)
	}
}

func TestListForms_SeedExists(t *testing.T) {
	s := newTestState(t)
	r := newRouter(s)

	req := httptest.NewRequest(http.MethodGet, "/api/forms", nil)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("want 200, got %d", w.Code)
	}

	var forms []Form
	if err := json.Unmarshal(w.Body.Bytes(), &forms); err != nil {
		t.Fatalf("decode: %v", err)
	}
	if len(forms) == 0 {
		t.Error("expected at least the seeded demo form")
	}
}

func TestCreateForm_Valid(t *testing.T) {
	s := newTestState(t)
	r := newRouter(s)

	payload := CreateFormRequest{Name: "Test Form", Domain: "example.com", AmountSats: 10}
	w := postJSON(r, "/api/forms", payload)

	if w.Code != http.StatusOK {
		t.Fatalf("want 200, got %d — body: %s", w.Code, w.Body.String())
	}

	var form Form
	if err := json.Unmarshal(w.Body.Bytes(), &form); err != nil {
		t.Fatalf("decode: %v", err)
	}
	if form.Name != "Test Form" {
		t.Errorf("want name 'Test Form', got %q", form.Name)
	}
	if form.ID == "" {
		t.Error("expected a non-empty ID")
	}
}

func TestCreateForm_Validation(t *testing.T) {
	s := newTestState(t)
	r := newRouter(s)

	cases := []struct {
		name    string
		payload CreateFormRequest
	}{
		{"missing name", CreateFormRequest{Name: "", Domain: "example.com", AmountSats: 5}},
		{"missing domain", CreateFormRequest{Name: "Form", Domain: "", AmountSats: 5}},
		{"amount too low", CreateFormRequest{Name: "Form", Domain: "example.com", AmountSats: 0}},
		{"amount too high", CreateFormRequest{Name: "Form", Domain: "example.com", AmountSats: 10_001}},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			w := postJSON(r, "/api/forms", tc.payload)
			if w.Code != http.StatusBadRequest {
				t.Errorf("want 400, got %d", w.Code)
			}
		})
	}
}

func TestDeleteForm(t *testing.T) {
	s := newTestState(t)
	r := newRouter(s)

	// create a form first
	payload := CreateFormRequest{Name: "To Delete", Domain: "del.io", AmountSats: 1}
	w := postJSON(r, "/api/forms", payload)
	if w.Code != http.StatusOK {
		t.Fatalf("create form: %d", w.Code)
	}
	var form Form
	json.Unmarshal(w.Body.Bytes(), &form)

	// delete it
	req := httptest.NewRequest(http.MethodDelete, "/api/forms/"+form.ID, nil)
	wr := httptest.NewRecorder()
	r.ServeHTTP(wr, req)
	if wr.Code != http.StatusNoContent {
		t.Errorf("want 204, got %d", wr.Code)
	}

	// delete again — should 404
	req2 := httptest.NewRequest(http.MethodDelete, "/api/forms/"+form.ID, nil)
	wr2 := httptest.NewRecorder()
	r.ServeHTTP(wr2, req2)
	if wr2.Code != http.StatusNotFound {
		t.Errorf("want 404 on second delete, got %d", wr2.Code)
	}
}

func TestDeleteForm_NotFound(t *testing.T) {
	s := newTestState(t)
	r := newRouter(s)

	req := httptest.NewRequest(http.MethodDelete, "/api/forms/nonexistent-id", nil)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)
	if w.Code != http.StatusNotFound {
		t.Errorf("want 404, got %d", w.Code)
	}
}

func TestCreateAndGetInvoice(t *testing.T) {
	s := newTestState(t)
	r := newRouter(s)

	// use the seeded demo form
	payload := CreateInvoiceRequest{
		FormID:      "demo-form",
		SenderName:  "Bob",
		SenderEmail: "bob@example.com",
		Body:        "This is a ten-char+ message",
	}
	w := postJSON(r, "/api/invoices", payload)
	if w.Code != http.StatusOK {
		t.Fatalf("createInvoice: %d — %s", w.Code, w.Body.String())
	}

	var inv InvoiceResponse
	if err := json.Unmarshal(w.Body.Bytes(), &inv); err != nil {
		t.Fatalf("decode: %v", err)
	}
	if inv.InvoiceID == "" || inv.MessageID == "" {
		t.Error("expected non-empty InvoiceID and MessageID")
	}
	if inv.Status != "pending" {
		t.Errorf("want status pending, got %q", inv.Status)
	}

	// GET the invoice
	req := httptest.NewRequest(http.MethodGet, "/api/invoices/"+inv.InvoiceID, nil)
	wr := httptest.NewRecorder()
	r.ServeHTTP(wr, req)
	if wr.Code != http.StatusOK {
		t.Fatalf("getInvoice: %d", wr.Code)
	}

	var status InvoiceStatusResponse
	json.Unmarshal(wr.Body.Bytes(), &status)
	if status.Status != "pending" {
		t.Errorf("want pending, got %q", status.Status)
	}
}

func TestCreateInvoice_FormNotFound(t *testing.T) {
	s := newTestState(t)
	r := newRouter(s)

	payload := CreateInvoiceRequest{
		FormID:      "does-not-exist",
		SenderName:  "Bob",
		SenderEmail: "bob@example.com",
		Body:        "A valid message body here",
	}
	w := postJSON(r, "/api/invoices", payload)
	if w.Code != http.StatusNotFound {
		t.Errorf("want 404, got %d", w.Code)
	}
}

func TestMockPayInvoice(t *testing.T) {
	s := newTestState(t)
	r := newRouter(s)

	// create invoice
	payload := CreateInvoiceRequest{
		FormID:      "demo-form",
		SenderName:  "Carol",
		SenderEmail: "carol@example.com",
		Body:        "Testing mock payment flow",
	}
	w := postJSON(r, "/api/invoices", payload)
	var inv InvoiceResponse
	json.Unmarshal(w.Body.Bytes(), &inv)

	// mock-pay it
	req := httptest.NewRequest(http.MethodPost, "/api/invoices/"+inv.InvoiceID+"/mock-pay", nil)
	wr := httptest.NewRecorder()
	r.ServeHTTP(wr, req)
	if wr.Code != http.StatusOK {
		t.Fatalf("mock-pay: %d — %s", wr.Code, wr.Body.String())
	}

	var status InvoiceStatusResponse
	json.Unmarshal(wr.Body.Bytes(), &status)
	if status.Status != "paid" {
		t.Errorf("want paid, got %q", status.Status)
	}
	if status.PaidAt == nil {
		t.Error("expected PaidAt to be set after payment")
	}
}

func TestMockPayInvoice_NotFound(t *testing.T) {
	s := newTestState(t)
	r := newRouter(s)

	req := httptest.NewRequest(http.MethodPost, "/api/invoices/ghost-id/mock-pay", nil)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)
	if w.Code != http.StatusNotFound {
		t.Errorf("want 404, got %d", w.Code)
	}
}

func TestListMessages_OnlyPaid(t *testing.T) {
	s := newTestState(t)
	r := newRouter(s)

	// create + pay one invoice
	payload := CreateInvoiceRequest{
		FormID:      "demo-form",
		SenderName:  "Dave",
		SenderEmail: "dave@example.com",
		Body:        "This message should appear after payment",
	}
	w := postJSON(r, "/api/invoices", payload)
	var inv InvoiceResponse
	json.Unmarshal(w.Body.Bytes(), &inv)

	// before payment: messages list should be empty
	req := httptest.NewRequest(http.MethodGet, "/api/messages", nil)
	wr := httptest.NewRecorder()
	r.ServeHTTP(wr, req)
	var before []Message
	json.Unmarshal(wr.Body.Bytes(), &before)
	if len(before) != 0 {
		t.Errorf("want 0 messages before payment, got %d", len(before))
	}

	// pay it
	postJSON(r, "/api/invoices/"+inv.InvoiceID+"/mock-pay", nil)

	// after payment: message should appear
	req2 := httptest.NewRequest(http.MethodGet, "/api/messages", nil)
	wr2 := httptest.NewRecorder()
	r.ServeHTTP(wr2, req2)
	var after []Message
	json.Unmarshal(wr2.Body.Bytes(), &after)
	if len(after) != 1 {
		t.Errorf("want 1 message after payment, got %d", len(after))
	}
}

func TestLightningWebhook_ValidPayment(t *testing.T) {
	s := newTestState(t)
	r := newRouter(s)

	// create an invoice to get a real ID
	payload := CreateInvoiceRequest{
		FormID:      "demo-form",
		SenderName:  "Eve",
		SenderEmail: "eve@example.com",
		Body:        "Webhook payment test message",
	}
	w := postJSON(r, "/api/invoices", payload)
	var inv InvoiceResponse
	json.Unmarshal(w.Body.Bytes(), &inv)

	// fire webhook
	preimage := "abc123preimage"
	hook := LightningWebhook{
		InvoiceID:   inv.InvoiceID,
		Preimage:    &preimage,
		Status:      "paid",
	}
	body, _ := json.Marshal(hook)
	sig := makeSignature(string(body), s.WebhookSecret)

	req := httptest.NewRequest(http.MethodPost, "/api/webhooks/lightning", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("x-satgate-signature", sig)
	req.Header.Set("x-satgate-event-id", "evt-001")
	wr := httptest.NewRecorder()
	r.ServeHTTP(wr, req)

	if wr.Code != http.StatusOK {
		t.Fatalf("webhook: %d — %s", wr.Code, wr.Body.String())
	}

	var status InvoiceStatusResponse
	json.Unmarshal(wr.Body.Bytes(), &status)
	if status.Status != "paid" {
		t.Errorf("want paid, got %q", status.Status)
	}
}

func TestLightningWebhook_BadSignature(t *testing.T) {
	s := newTestState(t)
	r := newRouter(s)

	hook := LightningWebhook{InvoiceID: "any", Status: "paid"}
	body, _ := json.Marshal(hook)

	req := httptest.NewRequest(http.MethodPost, "/api/webhooks/lightning", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("x-satgate-signature", "badsignature")
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusUnauthorized {
		t.Errorf("want 401, got %d", w.Code)
	}
}

func TestLightningWebhook_NonPaidStatus(t *testing.T) {
	s := newTestState(t)
	r := newRouter(s)

	hook := LightningWebhook{InvoiceID: "any", Status: "pending"}
	body, _ := json.Marshal(hook)
	sig := makeSignature(string(body), s.WebhookSecret)

	req := httptest.NewRequest(http.MethodPost, "/api/webhooks/lightning", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("x-satgate-signature", sig)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("want 400, got %d", w.Code)
	}
}

func TestLightningWebhook_IdempotentDuplicate(t *testing.T) {
	s := newTestState(t)
	r := newRouter(s)

	// create invoice
	payload := CreateInvoiceRequest{
		FormID:      "demo-form",
		SenderName:  "Frank",
		SenderEmail: "frank@example.com",
		Body:        "Idempotency test message body",
	}
	w := postJSON(r, "/api/invoices", payload)
	var inv InvoiceResponse
	json.Unmarshal(w.Body.Bytes(), &inv)

	hook := LightningWebhook{InvoiceID: inv.InvoiceID, Status: "paid"}
	body, _ := json.Marshal(hook)
	sig := makeSignature(string(body), s.WebhookSecret)

	sendWebhook := func() *httptest.ResponseRecorder {
		req := httptest.NewRequest(http.MethodPost, "/api/webhooks/lightning", bytes.NewReader(body))
		req.Header.Set("Content-Type", "application/json")
		req.Header.Set("x-satgate-signature", sig)
		req.Header.Set("x-satgate-event-id", "evt-idempotent-001") // same ID both times
		wr := httptest.NewRecorder()
		r.ServeHTTP(wr, req)
		return wr
	}

	w1 := sendWebhook()
	w2 := sendWebhook()

	if w1.Code != http.StatusOK {
		t.Errorf("first webhook: want 200, got %d", w1.Code)
	}
	if w2.Code != http.StatusOK {
		t.Errorf("duplicate webhook: want 200, got %d — body: %s", w2.Code, w2.Body.String())
	}
}

// ---- DB-level helper tests ----

func TestMarkInvoicePaid_Idempotent(t *testing.T) {
	s := newTestState(t)
	r := newRouter(s)

	payload := CreateInvoiceRequest{
		FormID:      "demo-form",
		SenderName:  "Grace",
		SenderEmail: "grace@example.com",
		Body:        "Testing idempotent markInvoicePaid",
	}
	w := postJSON(r, "/api/invoices", payload)
	var inv InvoiceResponse
	json.Unmarshal(w.Body.Bytes(), &inv)

	preimage := "preimage-test"
	if err := s.markInvoicePaid(inv.InvoiceID, &preimage); err != nil {
		t.Fatalf("first markInvoicePaid: %v", err)
	}
	// second call should not return an error
	if err := s.markInvoicePaid(inv.InvoiceID, &preimage); err != nil {
		t.Fatalf("second markInvoicePaid (idempotent): %v", err)
	}

	// confirm still paid
	status, err := s.fetchInvoiceStatus(inv.InvoiceID)
	if err != nil {
		t.Fatalf("fetchInvoiceStatus: %v", err)
	}
	if status.Status != "paid" {
		t.Errorf("want paid, got %q", status.Status)
	}
}

func TestSeedDefaultForm_OnlyOnce(t *testing.T) {
	db, _ := sql.Open("sqlite3", ":memory:")
	db.SetMaxOpenConns(1)
	defer db.Close()
	runMigrations(db)

	seedDefaultForm(db)
	seedDefaultForm(db) // second call should be a no-op

	var count int
	db.QueryRow("SELECT COUNT(*) FROM forms WHERE id = 'demo-form'").Scan(&count)
	if count != 1 {
		t.Errorf("want exactly 1 seeded form, got %d", count)
	}
}

// Ensure invoice expiry is set in the future on creation
func TestInvoiceExpiresAt_InFuture(t *testing.T) {
	s := newTestState(t)
	r := newRouter(s)

	payload := CreateInvoiceRequest{
		FormID:      "demo-form",
		SenderName:  "Henry",
		SenderEmail: "henry@example.com",
		Body:        "Checking expiry is set correctly",
	}
	w := postJSON(r, "/api/invoices", payload)
	var inv InvoiceResponse
	json.Unmarshal(w.Body.Bytes(), &inv)

	if !inv.ExpiresAt.After(time.Now()) {
		t.Errorf("expected ExpiresAt to be in the future, got %v", inv.ExpiresAt)
	}
}

// Verify that amount_sats in the invoice matches the form's configured amount
func TestInvoiceAmountMatchesForm(t *testing.T) {
	s := newTestState(t)
	r := newRouter(s)

	// create a form with a specific amount
	formPayload := CreateFormRequest{Name: "Priced Form", Domain: "price.io", AmountSats: 42}
	fw := postJSON(r, "/api/forms", formPayload)
	var form Form
	json.Unmarshal(fw.Body.Bytes(), &form)

	invPayload := CreateInvoiceRequest{
		FormID:      form.ID,
		SenderName:  "Iris",
		SenderEmail: "iris@example.com",
		Body:        "Checking amount matches form config",
	}
	w := postJSON(r, "/api/invoices", invPayload)
	var inv InvoiceResponse
	json.Unmarshal(w.Body.Bytes(), &inv)

	if inv.AmountSats != 42 {
		t.Errorf("want AmountSats=42, got %d", inv.AmountSats)
	}
}

// GetInvoice on a non-existent ID should return 404
func TestGetInvoice_NotFound(t *testing.T) {
	s := newTestState(t)
	r := newRouter(s)

	req := httptest.NewRequest(http.MethodGet, "/api/invoices/ghost-invoice", nil)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)
	if w.Code != http.StatusNotFound {
		t.Errorf("want 404, got %d", w.Code)
	}
}

func TestMain(m *testing.M) {
	os.MkdirAll("dist", 0755)
	os.Exit(m.Run())
}

// Unused import guard — fmt is used in buildDemoInvoice
var _ = fmt.Sprintf
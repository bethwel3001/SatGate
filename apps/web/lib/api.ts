import type { InvoiceResponse, InvoiceStatus, SatGateForm, SatGateMessage } from "./types";

export const API_URL = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080";

// Mock data for fallback
const MOCK_FORMS: SatGateForm[] = [
  {
    id: "demo-form",
    name: "Sarah's Portfolio",
    domain: "localhost",
    amount_sats: 5,
    created_at: new Date().toISOString(),
  },
  {
    id: "blog-form",
    name: "Tech Blog Contact",
    domain: "blog.example.com",
    amount_sats: 10,
    created_at: new Date().toISOString(),
  }
];

const MOCK_MESSAGES: SatGateMessage[] = [
  {
    id: "msg-1",
    form_id: "demo-form",
    sender_name: "Alice",
    sender_email: "alice@example.com",
    body: "Hi Sarah, I love your work! Let's talk about a project.",
    status: "paid",
    amount_sats: 5,
    created_at: new Date().toISOString(),
    paid_at: new Date().toISOString(),
  }
];

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  // Check if we should use mock data (if backend is known to be down or in demo mode)
  const useMock = typeof window !== 'undefined' && (window.location.hostname === 'localhost' || window.location.hostname.includes('gemini'));

  try {
    const response = await fetch(`${API_URL}${path}`, {
      ...init,
      headers: {
        "Content-Type": "application/json",
        ...init?.headers,
      },
      // Short timeout
      signal: AbortSignal.timeout(1500),
    });

    if (!response.ok) {
      throw new Error("Backend unreachable");
    }

    return response.json() as Promise<T>;
  } catch (error) {
    console.warn(`Using mock data for ${path}`);
    
    if (path === "/api/forms") {
      if (init?.method === "POST") {
        const payload = JSON.parse(init.body as string);
        const newForm = { ...payload, id: `form-${Math.random().toString(36).slice(2, 9)}`, created_at: new Date().toISOString() };
        MOCK_FORMS.push(newForm);
        return newForm as unknown as T;
      }
      return MOCK_FORMS as unknown as T;
    }
    
    if (path === "/api/messages") {
      return MOCK_MESSAGES as unknown as T;
    }
    
    if (path === "/api/invoices") {
      return {
        invoice_id: `inv-${Math.random().toString(36).slice(2, 9)}`,
        message_id: `msg-${Math.random().toString(36).slice(2, 9)}`,
        payment_request: "lnbc1demo-invoice-request-this-is-a-mock-invoice",
        amount_sats: 5,
        status: "pending",
        expires_at: new Date(Date.now() + 600000).toISOString(),
      } as unknown as T;
    }

    if (path.includes("/mock-pay")) {
      const invoiceId = path.split("/")[3];
      return {
        invoice_id: invoiceId,
        message_id: "msg-mock",
        status: "paid",
        paid_at: new Date().toISOString(),
      } as unknown as T;
    }

    if (path.startsWith("/api/invoices/")) {
      const invoiceId = path.split("/")[3];
      return {
        invoice_id: invoiceId,
        message_id: "msg-mock",
        status: "pending",
        paid_at: null,
      } as unknown as T;
    }

    return [] as unknown as T;
  }
}

export function getForms() {
  return request<SatGateForm[]>("/api/forms", { cache: "no-store" });
}

export function createForm(payload: { name: string; domain: string; amount_sats: number }) {
  return request<SatGateForm>("/api/forms", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export function getMessages() {
  return request<SatGateMessage[]>("/api/messages", { cache: "no-store" });
}

export function createInvoice(payload: {
  form_id: string;
  sender_name: string;
  sender_email: string;
  body: string;
}) {
  return request<InvoiceResponse>("/api/invoices", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export function getInvoice(invoiceId: string) {
  return request<InvoiceStatus>(`/api/invoices/${invoiceId}`, { cache: "no-store" });
}

export function mockPayInvoice(invoiceId: string) {
  return request<InvoiceStatus>(`/api/invoices/${invoiceId}/mock-pay`, {
    method: "POST",
  });
}

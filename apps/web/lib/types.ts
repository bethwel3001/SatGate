export type Site = {
  id: string;
  owner_id: string;
  name: string;
  sat_balance: number;
  domain?: string;
  amount_sats?: number;
};

export type Submission = {
  id: string;
  site_id: string;
  message: string;
  status: "verified" | "pending" | "failed";
  paid_sats: number;
  timestamp: string;
  sender_name?: string;
  sender_email?: string;
  body?: string;
  amount_sats?: number;
  form_id?: string;
};

export type SatGoForm = Site & {
  domain: string;
  amount_sats: number;
};

export type SatGoMessage = Submission & {
  sender_name: string;
  sender_email: string;
  body: string;
  amount_sats: number;
  form_id: string;
};

export type InvoiceResponse = {
  payment_hash: string;
  payment_request: string;
};

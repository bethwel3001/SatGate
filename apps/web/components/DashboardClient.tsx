"use client";

import { FormEvent, useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { Check, Copy, ExternalLink, Inbox, Pencil, RefreshCw, ShieldCheck, Trash2, X } from "lucide-react";
import { API_URL, createForm, deleteForm, getForms, getMessages, updateForm } from "@/lib/api";
import type { SatGateForm, SatGateMessage } from "@/lib/types";
import { PrimaryButton } from "./PrimaryButton";
import { StatusBadge } from "./StatusBadge";

function BitcoinPulseLoader() {
  return (
    <main className="mx-auto flex min-h-screen w-full max-w-7xl flex-col gap-6 px-4 py-5 sm:px-6 lg:px-8">
      <div className="flex min-h-screen flex-col items-center justify-center gap-6">
        <div className="relative flex items-center justify-center">
          <div style={{
            position: "absolute",
            width: "90px",
            height: "90px",
            borderRadius: "50%",
            background: "rgba(29,78,216,0.15)",
            animation: "ping 1.4s ease-out infinite",
          }} />
          <div style={{
            position: "absolute",
            width: "70px",
            height: "70px",
            borderRadius: "50%",
            background: "rgba(29,78,216,0.2)",
            animation: "ping 1.4s ease-out infinite 0.3s",
          }} />
          <div style={{
            width: "56px",
            height: "56px",
            borderRadius: "50%",
            background: "linear-gradient(135deg, #1d4ed8, #3b82f6)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            boxShadow: "0 0 24px rgba(29,78,216,0.5)",
            animation: "glow 1.4s ease-in-out infinite alternate",
            zIndex: 1,
          }}>
            <span style={{
              color: "white",
              fontSize: "26px",
              fontWeight: "bold",
              fontFamily: "var(--font-brand)",
              lineHeight: 1,
              marginTop: "-2px",
            }}>₿</span>
          </div>
        </div>
        <div className="flex flex-col items-center gap-1">
          <p className="brand-font text-lg font-bold text-satBlack">SatGate</p>
          <p className="text-sm text-slate-500">Loading your dashboard...</p>
        </div>
        <style>{`
          @keyframes ping {
            0% { transform: scale(0.8); opacity: 0.8; }
            100% { transform: scale(2); opacity: 0; }
          }
          @keyframes glow {
            from { box-shadow: 0 0 12px rgba(29,78,216,0.4); }
            to { box-shadow: 0 0 28px rgba(29,78,216,0.9), 0 0 48px rgba(59,130,246,0.3); }
          }
          @keyframes shimmer {
            0% { background-position: -600px 0; }
            100% { background-position: 600px 0; }
          }
          @keyframes progress {
            0% { width: 0%; }
            20% { width: 25%; }
            50% { width: 60%; }
            80% { width: 85%; }
            95% { width: 95%; }
          }
        `}</style>
      </div>
    </main>
  );
}

function SkeletonCard() {
  const shimmer = {
    background: "linear-gradient(90deg, #f1f5f9 25%, #e2e8f0 50%, #f1f5f9 75%)",
    backgroundSize: "600px 100%",
    animation: "shimmer 1.4s infinite linear",
  };
  return (
    <div className="rounded-lg border border-slate-200 bg-white p-5 shadow-panel">
      <div style={{ ...shimmer, height: "14px", width: "60%", borderRadius: "6px", marginBottom: "12px" }} />
      <div style={{ ...shimmer, height: "32px", width: "40%", borderRadius: "6px" }} />
    </div>
  );
}

function SkeletonMessage() {
  const shimmer = {
    background: "linear-gradient(90deg, #f1f5f9 25%, #e2e8f0 50%, #f1f5f9 75%)",
    backgroundSize: "600px 100%",
    animation: "shimmer 1.4s infinite linear",
  };
  return (
    <div className="rounded-md border border-slate-200 bg-white p-4">
      <div className="flex items-start justify-between">
        <div style={{ flex: 1 }}>
          <div style={{ ...shimmer, height: "16px", width: "35%", borderRadius: "6px", marginBottom: "8px" }} />
          <div style={{ ...shimmer, height: "13px", width: "50%", borderRadius: "6px" }} />
        </div>
        <div style={{ ...shimmer, height: "24px", width: "70px", borderRadius: "20px" }} />
      </div>
      <div style={{ ...shimmer, height: "13px", width: "100%", borderRadius: "6px", marginTop: "16px" }} />
      <div style={{ ...shimmer, height: "13px", width: "80%", borderRadius: "6px", marginTop: "8px" }} />
    </div>
  );
}

function FormProgressBar({ active }: { active: boolean }) {
  if (!active) return null;
  return (
    <div style={{
      position: "absolute",
      top: 0,
      left: 0,
      right: 0,
      height: "3px",
      background: "#e2e8f0",
      borderRadius: "3px 3px 0 0",
      overflow: "hidden",
    }}>
      <div style={{
        height: "100%",
        background: "linear-gradient(90deg, #1d4ed8, #3b82f6)",
        animation: "progress 2.5s ease-out forwards",
        boxShadow: "0 0 8px rgba(29,78,216,0.6)",
      }} />
    </div>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-slate-200 bg-white p-5 shadow-panel">
      <p className="text-sm font-semibold text-slate-600">{label}</p>
      <p className="brand-font mt-2 text-3xl font-bold text-satBlack">{value}</p>
    </div>
  );
}

interface EditState {
  id: string;
  name: string;
  domain: string;
  amount_sats: number;
}

export function DashboardClient() {
  const [forms, setForms] = useState<SatGateForm[]>([]);
  const [messages, setMessages] = useState<SatGateMessage[]>([]);
  const [name, setName] = useState("Portfolio Contact");
  const [domain, setDomain] = useState("localhost");
  const [amount, setAmount] = useState(5);
  const [error, setError] = useState("");
  const [copied, setCopied] = useState(false);
  const [loading, setLoading] = useState(true);
  const [submitting, setSubmitting] = useState(false);
  const [selectedFormId, setSelectedFormId] = useState<string | null>(null);
  const [editState, setEditState] = useState<EditState | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    async function init() {
      setLoading(true);
      try {
        const [f, m] = await Promise.all([getForms(), getMessages()]);
        setForms(f);
        setMessages(m);
        if (f.length > 0) setSelectedFormId(f[0].id);
      } catch (e) {
        console.error(e);
        setError("Failed to load data");
      } finally {
        setLoading(false);
      }
    }
    init();
  }, []);

  const selectedForm = forms.find((f) => f.id === (selectedFormId || forms[0]?.id));

  const filteredMessages = useMemo(() => {
    if (!selectedForm) return [];
    return messages.filter((m) => m.form_id === selectedForm.id);
  }, [messages, selectedForm]);

  const totalSats = filteredMessages.reduce((sum, m) => sum + m.amount_sats, 0);

  const embedUrl = typeof window !== "undefined" && selectedForm
    ? `${window.location.origin}/embed/${selectedForm.id}`
    : "";

  const embedSnippet = selectedForm
    ? `<iframe src="${embedUrl}" title="SatGate contact form" width="100%" height="620" style="border:0"></iframe>`
    : "";

  async function loadData() {
    setLoading(true);
    try {
      const [f, m] = await Promise.all([getForms(), getMessages()]);
      setForms(f);
      setMessages(m);
    } catch {
      setError("Refresh failed");
    } finally {
      setLoading(false);
    }
  }

  async function handleCreateForm(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitting(true);
    try {
      const newForm = await createForm({ name, domain, amount_sats: amount });
      setForms(prev => [newForm, ...prev]);
      setSelectedFormId(newForm.id);
    } catch {
      setError("Form creation failed");
    } finally {
      setSubmitting(false);
    }
  }

  async function handleDeleteForm(id: string) {
    if (!confirm("Are you sure? All associated messages will be deleted.")) return;
    try {
      await deleteForm(id);
      setForms(prev => prev.filter(f => f.id !== id));
      if (selectedFormId === id) setSelectedFormId(forms.find(f => f.id !== id)?.id || null);
    } catch {
      setError("Deletion failed");
    }
  }

  function startEdit(form: SatGateForm) {
    setEditState({ id: form.id, name: form.name, domain: form.domain, amount_sats: form.amount_sats });
  }

  function cancelEdit() {
    setEditState(null);
  }

  async function handleSaveEdit() {
    if (!editState) return;
    setSaving(true);
    setError("");
    try {
      const updated = await updateForm(editState.id, {
        name: editState.name,
        domain: editState.domain,
        amount_sats: editState.amount_sats,
      });
      setForms(prev => prev.map(f => (f.id === updated.id ? updated : f)));
      setEditState(null);
    } catch {
      setError("Update failed");
    } finally {
      setSaving(false);
    }
  }

  async function copyEmbed() {
    if (!embedSnippet) return;
    await navigator.clipboard.writeText(embedSnippet);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  if (loading && forms.length === 0) {
    return <BitcoinPulseLoader />;
  }

  return (
    <main className="mx-auto flex min-h-screen w-full max-w-7xl flex-col gap-6 px-4 py-5 sm:px-6 lg:px-8">
      <header className="flex flex-col gap-4 border-b border-slate-200 pb-5 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <p className="text-sm font-semibold uppercase text-satBlue">Economic proof of intent</p>
          <h1 className="brand-font mt-2 text-3xl font-bold text-satBlack sm:text-4xl">SatGate</h1>
          <p className="mt-2 max-w-2xl text-base text-slate-600">
            Lightning-powered contact forms that make abuse costly and keep verified messages easy to read.
          </p>
        </div>
        <div className="flex flex-wrap gap-3">
          <Link
            className="inline-flex h-11 items-center justify-center gap-2 rounded-md border border-slate-300 bg-white px-4 text-sm font-semibold text-satBlack transition hover:border-satBlue hover:text-satBlue"
            href="/demo"
          >
            <ExternalLink size={17} aria-hidden="true" />
            Demo site
          </Link>
          <PrimaryButton onClick={loadData} disabled={loading}>
            <RefreshCw className={loading ? "animate-spin" : ""} size={17} aria-hidden="true" />
            Refresh
          </PrimaryButton>
        </div>
      </header>

      {error && (
        <section className="rounded-md border border-red-200 bg-red-50 p-4 text-sm font-semibold text-satRed">
          {error}
        </section>
      )}

      <section className="grid gap-4 md:grid-cols-3">
        {loading ? (
          <>
            <SkeletonCard />
            <SkeletonCard />
            <SkeletonCard />
          </>
        ) : (
          <>
            <Metric label="Verified messages" value={filteredMessages.length.toString()} />
            <Metric label="Sats collected" value={totalSats.toString()} />
            <Metric label="Active forms" value={forms.length.toString()} />
          </>
        )}
      </section>

      <section className="grid gap-5 lg:grid-cols-[380px_1fr]">
        <div className="flex flex-col gap-5">
          <div className="rounded-lg border border-slate-200 bg-white p-5 shadow-panel">
            <h2 className="brand-font text-xl font-bold text-satBlack">Select Form</h2>
            <div className="mt-4 flex flex-col gap-2">
              {forms.length === 0 && !loading && (
                <p className="text-sm italic text-slate-500">No forms found.</p>
              )}
              {forms.map((form) =>
                editState?.id === form.id ? (
                  // ── inline edit row ──
                  <div
                    key={form.id}
                    className="flex flex-col gap-2 rounded-md border border-satBlue bg-blue-50/40 p-3"
                  >
                    <input
                      className="h-9 w-full rounded border border-slate-300 px-2 text-sm outline-none focus:border-satBlue focus:ring-2 focus:ring-blue-100"
                      placeholder="Form name"
                      value={editState.name}
                      onChange={(e) => setEditState({ ...editState, name: e.target.value })}
                    />
                    <input
                      className="h-9 w-full rounded border border-slate-300 px-2 text-sm outline-none focus:border-satBlue focus:ring-2 focus:ring-blue-100"
                      placeholder="Allowed domain"
                      value={editState.domain}
                      onChange={(e) => setEditState({ ...editState, domain: e.target.value })}
                    />
                    <input
                      className="h-9 w-full rounded border border-slate-300 px-2 text-sm outline-none focus:border-satBlue focus:ring-2 focus:ring-blue-100"
                      type="number"
                      min={1}
                      max={10000}
                      placeholder="Sats"
                      value={editState.amount_sats}
                      onChange={(e) => setEditState({ ...editState, amount_sats: Number(e.target.value) })}
                    />
                    <div className="flex gap-2">
                      <button
                        onClick={handleSaveEdit}
                        disabled={saving}
                        className="flex flex-1 items-center justify-center gap-1.5 rounded-md bg-satBlue px-3 py-1.5 text-xs font-bold text-white transition hover:opacity-90 disabled:opacity-50"
                      >
                        <Check size={13} />
                        {saving ? "Saving…" : "Save"}
                      </button>
                      <button
                        onClick={cancelEdit}
                        className="flex flex-1 items-center justify-center gap-1.5 rounded-md border border-slate-300 px-3 py-1.5 text-xs font-bold text-slate-600 transition hover:border-slate-400"
                      >
                        <X size={13} />
                        Cancel
                      </button>
                    </div>
                  </div>
                ) : (
                  // ── normal form row ──
                  <div key={form.id} className="group relative">
                    <button
                      onClick={() => setSelectedFormId(form.id)}
                      className={`flex w-full flex-col rounded-md border p-3 text-left transition ${
                        selectedForm?.id === form.id
                          ? "border-satBlue bg-blue-50/50"
                          : "border-slate-200 hover:border-blue-200"
                      }`}
                    >
                      <span className="pr-16 text-sm font-bold text-satBlack">{form.name}</span>
                      <span className="text-xs text-slate-500">{form.domain} · {form.amount_sats} sats</span>
                    </button>
                    <div className="absolute right-2 top-1/2 flex -translate-y-1/2 gap-1 opacity-0 transition group-hover:opacity-100">
                      <button
                        onClick={(e) => { e.stopPropagation(); startEdit(form); }}
                        className="flex h-8 w-8 items-center justify-center rounded-md text-slate-400 transition hover:bg-blue-50 hover:text-satBlue"
                        title="Edit form"
                      >
                        <Pencil size={15} />
                      </button>
                      <button
                        onClick={(e) => { e.stopPropagation(); handleDeleteForm(form.id); }}
                        className="flex h-8 w-8 items-center justify-center rounded-md text-slate-400 transition hover:bg-red-50 hover:text-satRed"
                        title="Delete form"
                      >
                        <Trash2 size={15} />
                      </button>
                    </div>
                  </div>
                )
              )}
            </div>
          </div>

          <div className="relative rounded-lg border border-slate-200 bg-white p-5 shadow-panel overflow-hidden">
            <FormProgressBar active={submitting} />
            <div className="flex items-start justify-between gap-3">
              <div>
                <h2 className="brand-font text-xl font-bold text-satBlack">New form</h2>
                <p className="mt-1 text-sm text-slate-600">Create an iframe-ready SatGate form.</p>
              </div>
              <ShieldCheck className="text-satBlue" size={24} aria-hidden="true" />
            </div>

            <form className="mt-5 space-y-4" onSubmit={handleCreateForm}>
              <label className="block text-sm font-semibold text-satBlack">
                Form name
                <input
                  className="mt-1 h-11 w-full rounded-md border border-slate-300 px-3 text-sm outline-none transition focus:border-satBlue focus:ring-2 focus:ring-blue-100"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                />
              </label>
              <label className="block text-sm font-semibold text-satBlack">
                Allowed domain
                <input
                  className="mt-1 h-11 w-full rounded-md border border-slate-300 px-3 text-sm outline-none transition focus:border-satBlue focus:ring-2 focus:ring-blue-100"
                  value={domain}
                  onChange={(e) => setDomain(e.target.value)}
                />
              </label>
              <label className="block text-sm font-semibold text-satBlack">
                Price in sats
                <input
                  className="mt-1 h-11 w-full rounded-md border border-slate-300 px-3 text-sm outline-none transition focus:border-satBlue focus:ring-2 focus:ring-blue-100"
                  min={1}
                  max={10000}
                  type="number"
                  value={amount}
                  onChange={(e) => setAmount(Number(e.target.value))}
                />
              </label>
              <PrimaryButton className="w-full" type="submit" disabled={submitting}>
                {submitting ? "Creating..." : "Create form"}
              </PrimaryButton>
            </form>

            <div className="mt-6 border-t border-slate-200 pt-5">
              <h3 className="text-sm font-bold text-satBlack">Embed snippet</h3>
              <pre className="mt-2 min-h-28 overflow-auto rounded-md bg-satBlack p-3 text-xs leading-5 text-white">
                {embedSnippet || "Create a form to generate an iframe snippet."}
              </pre>
              <PrimaryButton className="mt-3 w-full" disabled={!embedSnippet} onClick={copyEmbed} tone="neutral">
                <Copy size={17} aria-hidden="true" />
                {copied ? "Copied" : "Copy snippet"}
              </PrimaryButton>
            </div>
          </div>
        </div>

        <div className="rounded-lg border border-slate-200 bg-white p-5 shadow-panel">
          <div className="flex flex-col gap-3 border-b border-slate-200 pb-4 sm:flex-row sm:items-center sm:justify-between">
            <div>
              <h2 className="brand-font text-xl font-bold text-satBlack">Verified inbox</h2>
              <p className="mt-1 text-sm text-slate-600">Only paid contact submissions appear here.</p>
            </div>
          </div>

          <div className="mt-4 space-y-3">
            {loading ? (
              <>
                <SkeletonMessage />
                <SkeletonMessage />
                <SkeletonMessage />
              </>
            ) : filteredMessages.length === 0 ? (
              <div className="flex min-h-72 flex-col items-center justify-center rounded-md border border-dashed border-slate-300 p-6 text-center">
                <Inbox className="text-slate-400" size={36} aria-hidden="true" />
                <h3 className="brand-font mt-3 text-lg font-bold text-satBlack">No verified messages yet</h3>
                <p className="mt-2 max-w-md text-sm text-slate-600">
                  Open the demo site, submit the widget, and use the mock pay button to settle the invoice.
                </p>
              </div>
            ) : (
              filteredMessages.map((message) => (
                <article
                  className="rounded-md border border-slate-200 bg-white p-4 transition hover:border-blue-200 hover:bg-blue-50/40"
                  key={message.id}
                >
                  <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                    <div>
                      <h3 className="text-base font-bold text-satBlack">{message.sender_name}</h3>
                      <p className="text-sm text-slate-600">{message.sender_email}</p>
                    </div>
                    <div className="flex items-center gap-2">
                      <StatusBadge status={message.status} />
                      <span className="rounded-full bg-green-50 px-2.5 py-1 text-xs font-bold text-satGreen">
                        +{message.amount_sats} sats
                      </span>
                    </div>
                  </div>
                  <p className="mt-3 whitespace-pre-wrap text-sm leading-6 text-slate-700">{message.body}</p>
                </article>
              ))
            )}
          </div>
        </div>
      </section>
    </main>
  );
}
"use client";

import { FormEvent, useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { Copy, ExternalLink, Inbox, RefreshCw, ShieldCheck } from "lucide-react";
import { API_URL, createForm, getForms, getMessages } from "@/lib/api";
import type { SatGateForm, SatGateMessage } from "@/lib/types";
import { PrimaryButton } from "./PrimaryButton";
import { StatusBadge } from "./StatusBadge";

export function DashboardClient() {
  const [forms, setForms] = useState<SatGateForm[]>([]);
  const [messages, setMessages] = useState<SatGateMessage[]>([]);
  const [name, setName] = useState("Portfolio Contact");
  const [domain, setDomain] = useState("localhost");
  const [amount, setAmount] = useState(5);
  const [error, setError] = useState("");
  const [copied, setCopied] = useState(false);
  const [loading, setLoading] = useState(true);

  const selectedForm = forms[0];
  const embedUrl =
    typeof window === "undefined" || !selectedForm
      ? ""
      : `${window.location.origin}/embed/${selectedForm.id}`;
  const embedSnippet = selectedForm
    ? `<iframe src="${embedUrl}" title="SatGate contact form" width="100%" height="620" style="border:0"></iframe>`
    : "";

  const totalSats = useMemo(
    () => messages.reduce((total, message) => total + message.amount_sats, 0),
    [messages],
  );

  async function loadData() {
    setLoading(true);
    setError("");

    try {
      const [nextForms, nextMessages] = await Promise.all([getForms(), getMessages()]);
      setForms(nextForms);
      setMessages(nextMessages);
    } catch (loadError) {
      setError(loadError instanceof Error ? loadError.message : "Could not load SatGate data");
    } finally {
      setLoading(false);
    }
  }

  async function handleCreateForm(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError("");

    try {
      await createForm({ name, domain, amount_sats: amount });
      await loadData();
    } catch (createError) {
      setError(createError instanceof Error ? createError.message : "Could not create form");
    }
  }

  async function copyEmbed() {
    if (!embedSnippet) return;

    await navigator.clipboard.writeText(embedSnippet);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1800);
  }

  useEffect(() => {
    loadData();
  }, []);

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
            <RefreshCw size={17} aria-hidden="true" />
            Refresh
          </PrimaryButton>
        </div>
      </header>

      {error ? (
        <section className="rounded-md border border-red-200 bg-red-50 p-4 text-sm font-semibold text-satRed">
          {error}
        </section>
      ) : null}

      <section className="grid gap-4 md:grid-cols-3">
        <Metric label="Verified messages" value={messages.length.toString()} />
        <Metric label="Sats collected" value={totalSats.toString()} />
        <Metric label="Active forms" value={forms.length.toString()} />
      </section>

      <section className="grid gap-5 lg:grid-cols-[380px_1fr]">
        <div className="rounded-lg border border-slate-200 bg-white p-5 shadow-panel">
          <div className="flex items-start justify-between gap-3">
            <div>
              <h2 className="brand-font text-xl font-bold text-satBlack">Form setup</h2>
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
                onChange={(event) => setName(event.target.value)}
              />
            </label>
            <label className="block text-sm font-semibold text-satBlack">
              Allowed domain
              <input
                className="mt-1 h-11 w-full rounded-md border border-slate-300 px-3 text-sm outline-none transition focus:border-satBlue focus:ring-2 focus:ring-blue-100"
                value={domain}
                onChange={(event) => setDomain(event.target.value)}
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
                onChange={(event) => setAmount(Number(event.target.value))}
              />
            </label>
            <PrimaryButton className="w-full" type="submit">
              Create form
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

        <div className="rounded-lg border border-slate-200 bg-white p-5 shadow-panel">
          <div className="flex flex-col gap-3 border-b border-slate-200 pb-4 sm:flex-row sm:items-center sm:justify-between">
            <div>
              <h2 className="brand-font text-xl font-bold text-satBlack">Verified inbox</h2>
              <p className="mt-1 text-sm text-slate-600">Only paid contact submissions appear here.</p>
            </div>
            <p className="rounded-md bg-blue-50 px-3 py-2 text-xs font-semibold text-satBlue">
              API: {API_URL}
            </p>
          </div>

          <div className="mt-4 space-y-3">
            {messages.length === 0 ? (
              <div className="flex min-h-72 flex-col items-center justify-center rounded-md border border-dashed border-slate-300 p-6 text-center">
                <Inbox className="text-slate-400" size={36} aria-hidden="true" />
                <h3 className="brand-font mt-3 text-lg font-bold text-satBlack">No verified messages yet</h3>
                <p className="mt-2 max-w-md text-sm text-slate-600">
                  Open the demo site, submit the widget, and use the mock pay button to settle the invoice.
                </p>
              </div>
            ) : (
              messages.map((message) => (
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

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-slate-200 bg-white p-5 shadow-panel">
      <p className="text-sm font-semibold text-slate-600">{label}</p>
      <p className="brand-font mt-2 text-3xl font-bold text-satBlack">{value}</p>
    </div>
  );
}

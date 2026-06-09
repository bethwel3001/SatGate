import Link from "next/link";
import { ArrowLeft, ExternalLink } from "lucide-react";

export default function DemoPage() {
  return (
    <main className="min-h-screen bg-white text-satBlack">
      <section className="border-b border-slate-200 bg-satBlack text-white">
        <div className="mx-auto grid w-full max-w-7xl gap-8 px-4 py-8 sm:px-6 lg:grid-cols-[1fr_420px] lg:px-8">
          <div className="flex flex-col justify-center">
            <Link
              className="inline-flex w-fit items-center gap-2 rounded-md border border-white/20 px-3 py-2 text-sm font-semibold text-white transition hover:border-white"
              href="/"
            >
              <ArrowLeft size={16} aria-hidden="true" />
              Back to dashboard
            </Link>
            <p className="mt-8 text-sm font-bold uppercase text-blue-200">Demo portfolio site</p>
            <h1 className="brand-font mt-2 max-w-3xl text-4xl font-bold sm:text-5xl">
              Sarah builds sharp websites for independent founders.
            </h1>
            <p className="mt-4 max-w-2xl text-base leading-7 text-slate-200">
              This page shows how SatGate fits into a normal website. The contact form below is embedded as an iframe and only delivers paid messages.
            </p>
          </div>

          <div className="rounded-lg border border-white/10 bg-white p-5 text-satBlack">
            <p className="text-sm font-bold text-satBlue">Services</p>
            <ul className="mt-3 space-y-3 text-sm leading-6 text-slate-700">
              <li>Landing pages for new products</li>
              <li>Responsive portfolio sites</li>
              <li>Conversion-focused redesigns</li>
            </ul>
            <a
              className="mt-5 inline-flex items-center gap-2 rounded-md bg-satBlue px-4 py-3 text-sm font-semibold text-white transition hover:bg-blue-700"
              href="#contact"
            >
              <ExternalLink size={16} aria-hidden="true" />
              Contact Sarah
            </a>
          </div>
        </div>
      </section>

      <section className="mx-auto grid w-full max-w-7xl gap-6 px-4 py-8 sm:px-6 lg:grid-cols-[1fr_560px] lg:px-8" id="contact">
        <div>
          <p className="text-sm font-bold uppercase text-satBlue">Protected contact</p>
          <h2 className="brand-font mt-2 text-3xl font-bold">Spam-resistant by default</h2>
          <p className="mt-3 max-w-2xl text-base leading-7 text-slate-600">
            A visitor pays a few sats before the message reaches Sarah. Real leads keep a simple path, while automated bulk spam gets a direct cost.
          </p>
          <div className="mt-6 grid gap-3 sm:grid-cols-3">
            <Fact label="Widget" value="iframe" />
            <Fact label="Payment" value="WebLN or QR" />
            <Fact label="Demo price" value="5 sats" />
          </div>
        </div>

        <div className="overflow-hidden rounded-lg border border-slate-200 shadow-panel">
          <iframe
            className="h-[620px] w-full"
            src="/embed/demo-form"
            title="SatGate contact form"
          />
        </div>
      </section>
    </main>
  );
}

function Fact({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border border-slate-200 bg-slate-50 p-4">
      <p className="text-xs font-bold uppercase text-slate-500">{label}</p>
      <p className="brand-font mt-1 text-lg font-bold text-satBlack">{value}</p>
    </div>
  );
}

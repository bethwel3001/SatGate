import Link from "next/link";
import { ArrowLeft } from "lucide-react";

export default function DemoPage() {
  return (
    <main className="min-h-screen bg-[#f8fafc] text-satBlack selection:bg-blue-100">
      <nav className="mx-auto flex max-w-7xl items-center justify-between px-6 py-8">
        <Link
          className="group flex items-center gap-2 text-sm font-semibold text-slate-500 transition hover:text-satBlue"
          href="/"
        >
          <ArrowLeft className="transition-transform group-hover:-translate-x-1" size={18} aria-hidden="true" />
          Back to dashboard
        </Link>
      </nav>

      <div className="mx-auto max-w-7xl px-6 pb-24 pt-12">
        <div className="grid gap-16 lg:grid-cols-[1fr_500px]">
          <header className="flex flex-col justify-center">
            <h1 className="brand-font text-5xl font-bold leading-[1.1] tracking-tight text-satBlack sm:text-7xl">
              Sarah builds <span className="text-satBlue">sharp</span> websites for founders.
            </h1>
            <p className="mt-8 max-w-xl text-lg leading-relaxed text-slate-600">
              This demo shows how SatGate protects high-value contact forms. 
              Only verified, paid submissions reach Sarah's inbox.
            </p>
          </header>

          <aside className="relative">
            <div className="absolute -inset-4 rounded-[2rem] bg-gradient-to-tr from-blue-50 to-indigo-50 opacity-50 blur-2xl" />
            <div className="relative overflow-hidden rounded-2xl border border-slate-200 bg-white shadow-2xl">
              <div className="flex items-center gap-2 border-b border-slate-100 bg-slate-50/50 px-4 py-3">
                <div className="flex gap-1.5">
                  <div className="h-3 w-3 rounded-full bg-slate-200" />
                  <div className="h-3 w-3 rounded-full bg-slate-200" />
                  <div className="h-3 w-3 rounded-full bg-slate-200" />
                </div>
              </div>
              <iframe
                className="h-[640px] w-full"
                src="/embed/demo-form"
                title="SatGate contact form"
              />
            </div>
          </aside>
        </div>
      </div>
    </main>
  );
}

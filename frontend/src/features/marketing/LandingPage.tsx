import { useState } from 'react'
import { Link } from 'react-router-dom'

// ─── Logo ────────────────────────────────────────────────────────────────────

function Logo() {
  return (
    <div className="flex items-center gap-2.5">
      <div className="w-[30px] h-[30px] rounded-[9px] bg-(--lp-malt) flex items-center justify-center shrink-0">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <path d="M3 1 L13 1 L13 5 Q13 8 10 9.5 L10 15 L6 15 L6 9.5 Q3 8 3 5 Z" fill="white" />
          <circle cx="8" cy="5" r="1.4" fill="var(--lp-malt)" />
        </svg>
      </div>
      <span className="text-[18px] font-bold tracking-[-0.5px] text-(--lp-ink) font-dm-sans">BatchWise</span>
    </div>
  )
}

// ─── Nav ─────────────────────────────────────────────────────────────────────

function Nav() {
  return (
    <div className="px-8 py-5 flex justify-center sticky top-0 z-50">
      <div
        className="bg-(--lp-card) rounded-full pl-6 pr-2 py-2 flex items-center gap-7 border border-(--lp-rule)"
        style={{ boxShadow: '0 1px 2px rgba(28,17,8,.05), 0 8px 32px rgba(28,17,8,.08)' }}
      >
        <Logo />
        <nav className="hidden md:flex gap-6 text-[14px] text-(--lp-muted) font-medium">
          {['Recipes', 'Brewhouse', 'Inventory', 'Calendar', 'Pricing'].map((l) => (
            <a key={l} href="#" className="hover:text-(--lp-ink) transition-colors duration-150">{l}</a>
          ))}
        </nav>
        <div className="flex items-center gap-1">
          <Link
            to="/login"
            className="text-[13px] text-(--lp-muted) px-4 py-2 hover:text-(--lp-ink) transition-colors duration-150"
          >
            Sign in
          </Link>
          <Link
            to="/register"
            className="bg-(--lp-malt-deep) text-white text-[13px] font-semibold px-5 py-2.5 rounded-full hover:brightness-110 transition-[background,transform] duration-150 hover:translate-y-[-1px]"
          >
            Get started →
          </Link>
        </div>
      </div>
    </div>
  )
}

// ─── Hero product card ────────────────────────────────────────────────────────

function HeroProductCard() {
  return (
    <div
      className="bg-(--lp-card) rounded-[24px] p-6 border border-(--lp-rule) relative"
      style={{ boxShadow: '0 24px 60px rgba(122,59,20,.10)' }}
    >
      {/* Header */}
      <div className="flex justify-between items-baseline mb-4">
        <div>
          <div className="text-[11px] font-bold text-(--lp-faint) tracking-[1.2px] font-dm-mono">BATCH B-142 · DAY 4</div>
          <div className="text-[22px] font-bold tracking-[-0.6px] text-(--lp-ink) mt-0.5">
            Honey Saison{' '}
            <span className="text-(--lp-muted) font-medium text-[15px]">· Belgian</span>
          </div>
        </div>
        <div className="bg-(--lp-hop-soft) text-(--lp-hop) px-3 py-1.5 rounded-full text-[11px] font-bold tracking-[0.8px] flex items-center gap-1.5">
          <span className="lp-pulse inline-block">●</span> FERMENTING
        </div>
      </div>

      {/* Telemetry tiles */}
      <div className="grid grid-cols-4 gap-2 mb-3">
        {[
          { v: '1.024', l: 'GRAVITY', c: 'var(--lp-malt)', sub: '↓ 0.012' },
          { v: '67.4°F', l: 'TEMP', c: 'var(--lp-hop)', sub: 'set 68°' },
          { v: '4.32', l: 'PH', c: 'var(--lp-muted)', sub: 'nominal' },
          { v: '~3d', l: 'TO FG', c: 'var(--lp-malt-deep)', sub: 'on track' },
        ].map(({ v, l, c, sub }) => (
          <div key={l} className="bg-(--lp-card-soft) p-3 rounded-[12px]">
            <div className="text-[10px] font-bold text-(--lp-faint) tracking-[1px] font-dm-mono">{l}</div>
            <div className="text-[22px] font-bold tracking-[-0.5px] mt-0.5 font-dm-mono leading-none" style={{ color: c }}>{v}</div>
            <div className="text-[10px] text-(--lp-muted) font-medium mt-1 font-dm-mono">{sub}</div>
          </div>
        ))}
      </div>

      {/* Chart */}
      <div className="bg-(--lp-card-soft) rounded-[14px] p-4 mb-3">
        <div className="flex justify-between text-[10px] font-bold text-(--lp-faint) tracking-[1px] font-dm-mono mb-2">
          <span>GRAVITY × TEMP — 96 H</span>
          <span className="text-(--lp-hop)">● TILT-PINK · LIVE</span>
        </div>
        <svg viewBox="0 0 440 100" width="100%" height="80" className="block">
          <defs>
            <linearGradient id="lp-sg" x1="0" x2="0" y1="0" y2="1">
              <stop offset="0%" stopColor="var(--lp-malt)" stopOpacity="0.35" />
              <stop offset="100%" stopColor="var(--lp-malt)" stopOpacity="0" />
            </linearGradient>
          </defs>
          {[0, 24, 48, 72].map((x) => (
            <line key={x} x1={x * 5.5} y1="0" x2={x * 5.5} y2="100" stroke="var(--lp-rule)" strokeDasharray="2 3" />
          ))}
          <path d="M0,10 C50,12 100,18 150,32 C200,52 260,62 320,68 C380,72 440,74 440,74 L440,100 L0,100 Z" fill="url(#lp-sg)" />
          <path d="M0,10 C50,12 100,18 150,32 C200,52 260,62 320,68 C380,72 440,74 440,74" fill="none" stroke="var(--lp-malt)" strokeWidth="2.2" />
          <path d="M0,52 C50,50 100,54 150,48 C200,52 260,50 320,52 C380,50 440,52 440,52" fill="none" stroke="var(--lp-hop)" strokeWidth="1.8" strokeDasharray="4 3" />
          {/* Live dot */}
          <circle cx="440" cy="74" r="4" fill="var(--lp-malt)" />
          <circle cx="440" cy="74" r="7" fill="var(--lp-malt)" fillOpacity="0.2" />
        </svg>
      </div>

      {/* Footer hint */}
      <div className="flex justify-between items-center px-1">
        <span className="text-[12px] text-(--lp-muted)">
          ↳ Next: <strong className="text-(--lp-ink)">dry-hop charge 1</strong> · in 18h
        </span>
        <span className="text-[12px] text-(--lp-malt) font-bold tracking-[0.3px]">VIEW BATCH →</span>
      </div>
    </div>
  )
}

// ─── Hero section ─────────────────────────────────────────────────────────────

type Audience = 'home' | 'pro'

const HERO_COPY = {
  home: {
    h: (<>Brew your<br /><span className="text-(--lp-malt)">best beer yet.</span></>),
    sub: 'The recipe book, brewday log, and pantry tracker for homebrewers. Free forever, no card required.',
    cta: 'Start your first batch',
  },
  pro: {
    h: (<>Built for the<br /><span className="text-(--lp-malt)">whole brewhouse.</span></>),
    sub: 'Production scheduling, ingredient traceability, and TTB-ready inventory for microbreweries and brewpubs.',
    cta: 'Book a brewhouse tour',
  },
}

function Hero({ aud, setAud }: { aud: Audience; setAud: (a: Audience) => void }) {
  const copy = HERO_COPY[aud]
  return (
    <section className="px-16 pt-10 pb-6 max-w-[1280px] mx-auto">
      <div className="grid grid-cols-1 lg:grid-cols-[1.05fr_1fr] gap-14 items-center">
        {/* Left */}
        <div>
          {/* Audience toggle */}
          <div className="inline-flex bg-(--lp-card-soft) rounded-full p-1 mb-7 border border-(--lp-rule)">
            {([['home', "I'm a homebrewer"], ['pro', 'I run a brewery']] as [Audience, string][]).map(([k, label]) => (
              <button
                key={k}
                onClick={() => setAud(k)}
                className={`px-[18px] py-[9px] rounded-full text-[13px] font-semibold font-dm-sans transition-all duration-200 cursor-pointer border-none ${
                  aud === k
                    ? 'bg-(--lp-malt-deep) text-white'
                    : 'bg-transparent text-(--lp-muted) hover:text-(--lp-ink)'
                }`}
              >
                {label}
              </button>
            ))}
          </div>

          {/* Headline */}
          <h1 className="text-[84px] leading-[0.96] tracking-[-3.2px] font-bold text-(--lp-ink) m-0 font-dm-sans">
            {copy.h}
          </h1>

          {/* Sub */}
          <p className="text-[18px] leading-[1.5] text-(--lp-muted) mt-6 mb-7 max-w-[460px] font-dm-sans">
            {copy.sub}
          </p>

          {/* CTAs */}
          <div className="flex gap-2.5 mb-8">
            <Link
              to="/register"
              className="bg-(--lp-malt-deep) text-white text-[15px] font-semibold px-7 py-[14px] rounded-full font-dm-sans hover:brightness-110 transition-[background,transform] duration-150 hover:translate-y-[-1px] no-underline"
            >
              {copy.cta}
            </Link>
            <a
              href="#how-it-works"
              className="bg-(--lp-card) text-(--lp-ink) text-[15px] font-medium px-6 py-[13px] rounded-full border-[1.5px] border-(--lp-rule) font-dm-sans hover:bg-(--lp-bg-alt) transition-colors duration-150 no-underline"
            >
              See how it works
            </a>
          </div>

          {/* Social proof */}
          <div className="flex items-center gap-6 text-[13px] text-(--lp-muted)">
            <div className="flex">
              {['var(--lp-malt)', 'var(--lp-hop)', 'var(--lp-malt-deep)', 'var(--lp-faint)'].map((c, i) => (
                <div
                  key={i}
                  className="w-7 h-7 rounded-full border-2 border-(--lp-bg)"
                  style={{ background: c, marginLeft: i > 0 ? -8 : 0 }}
                />
              ))}
            </div>
            <div>
              <div className="text-(--lp-ink) font-semibold">4,200+ brewers</div>
              <div className="text-[12px]">from kitchen to 30-barrel</div>
            </div>
          </div>
        </div>

        {/* Right */}
        <HeroProductCard />
      </div>
    </section>
  )
}

// ─── Integration strip ────────────────────────────────────────────────────────

function IntegrationStrip() {
  return (
    <div className="px-16 py-6 max-w-[1280px] mx-auto">
      <div className="bg-(--lp-card) border border-(--lp-rule) rounded-full px-7 py-[14px] flex items-center justify-between flex-wrap gap-3">
        <span className="text-[11px] font-bold text-(--lp-malt) tracking-[1.5px] font-dm-mono">INTEGRATIONS</span>
        {['Tilt Hydrometer', 'Plaato Airlock', 'BrewPi', 'Square POS', 'Untappd', 'QuickBooks', 'BeerXML'].map((x) => (
          <span key={x} className="text-[13px] text-(--lp-muted) font-medium">{x}</span>
        ))}
      </div>
    </div>
  )
}

// ─── Traceability ─────────────────────────────────────────────────────────────

const TRACE_STEPS = [
  { tag: 'SUPPLIER', title: 'Avangard Malz', meta: 'INV-9821 · 02 Mar', bg: 'var(--lp-card-soft)', accent: 'var(--lp-malt-deep)', border: 'var(--lp-rule)' },
  { tag: 'RECIPE', title: 'Honey Saison v4', meta: '8 lb · 84% bill', bg: 'var(--lp-malt-soft)', accent: 'var(--lp-malt)', border: 'var(--lp-rule)' },
  { tag: 'BATCH', title: 'B-142 · FV-04', meta: 'Brewed 14 Mar', bg: 'var(--lp-malt-soft)', accent: 'var(--lp-malt)', border: 'var(--lp-rule)' },
  { tag: 'PACKAGE', title: '12 × ½ BBL kegs', meta: 'KG-401 to KG-412', bg: 'var(--lp-hop-soft)', accent: 'var(--lp-hop)', border: 'var(--lp-rule)' },
  { tag: 'POUR', title: 'On tap · 9 venues', meta: 'First pour 28 Mar', bg: 'var(--lp-hop-soft)', accent: 'var(--lp-hop)', border: 'var(--lp-rule)' },
]

function Traceability() {
  return (
    <section className="px-16 pt-10 pb-8 max-w-[1280px] mx-auto">
      {/* Section header */}
      <div className="grid grid-cols-1 lg:grid-cols-[1fr_auto] gap-6 items-end mb-8">
        <div>
          <div className="text-[13px] font-bold text-(--lp-malt) tracking-[1.5px] font-dm-mono mb-2.5">FULL CHAIN TRACEABILITY</div>
          <h2 className="text-[44px] font-bold tracking-[-1.4px] leading-[1.05] text-(--lp-ink) m-0 font-dm-sans">
            Every pint, back to its grain.
          </h2>
        </div>
        <p className="text-[16px] text-(--lp-muted) max-w-[380px] leading-[1.5] m-0">
          Lot-level provenance from supplier delivery to the pour. Recall a single lot without recalling the brewery.
        </p>
      </div>

      {/* Pipeline card */}
      <div className="bg-(--lp-card) border border-(--lp-rule) rounded-[24px] p-7">
        {/* Card header */}
        <div className="flex justify-between items-baseline mb-5">
          <div className="text-[13px] font-semibold text-(--lp-ink)">
            Tracing lot <span className="font-dm-mono text-(--lp-malt)">#PM-2891</span> · Pilsner Malt · Avangard
          </div>
          <div className="text-[12px] text-(--lp-muted) font-dm-mono">Received Mar 02 · 50 lb · 7 lb remaining</div>
        </div>

        {/* Pipeline */}
        <div className="grid grid-cols-5 gap-0 items-stretch">
          {TRACE_STEPS.map((step, i) => (
            <div key={step.tag} className="relative px-1.5">
              {i < 4 && (
                <svg className="absolute top-1/2 right-[-9px] w-[18px] h-[12px] -translate-y-1/2 z-10" viewBox="0 0 18 12">
                  <path d="M0 6 L13 6 M9 2 L13 6 L9 10" stroke="var(--lp-faint)" strokeWidth="1.5" fill="none" strokeLinecap="round" />
                </svg>
              )}
              <div
                className="rounded-[14px] p-3.5 min-h-[130px] border"
                style={{ background: step.bg, borderColor: step.border }}
              >
                <div className="text-[10px] font-bold tracking-[1.2px] font-dm-mono mb-2.5" style={{ color: step.accent }}>{step.tag}</div>
                <div className="text-[15px] font-bold leading-[1.2] tracking-[-0.3px] mb-1 text-(--lp-ink)">{step.title}</div>
                <div className="text-[11px] text-(--lp-muted) font-dm-mono">{step.meta}</div>
              </div>
            </div>
          ))}
        </div>

        {/* Compliance footer */}
        <div className="mt-5 px-4 py-3.5 bg-(--lp-bg) rounded-[12px] flex items-center justify-between">
          <div className="flex items-center gap-3 text-[13px] text-(--lp-ink)">
            <span className="w-[26px] h-[26px] rounded-full bg-(--lp-hop-soft) text-(--lp-hop) inline-flex items-center justify-center font-bold shrink-0">✓</span>
            <span>
              <strong>Compliant.</strong> Recall scope:{' '}
              <strong className="text-(--lp-malt)">1 batch · 12 kegs · 9 venues</strong> — CSV, PDF, or TTB-ready.
            </span>
          </div>
          <button className="bg-(--lp-card) text-(--lp-ink) border border-(--lp-rule) px-3.5 py-2 rounded-full text-[12px] font-semibold font-dm-sans hover:bg-(--lp-bg-alt) transition-colors duration-150 cursor-pointer">
            Run audit →
          </button>
        </div>
      </div>
    </section>
  )
}

// ─── Calendar ─────────────────────────────────────────────────────────────────

const DAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']

const CAL_EVENTS = [
  { day: 0, offset: 1, label: 'Brew · Honey Saison v5', bg: 'var(--lp-malt)', text: '#fff', border: null },
  { day: 0, offset: 2.5, label: 'Ferment · FV-04 (6d)', bg: '#fff', text: 'var(--lp-hop)', border: 'var(--lp-hop)' },
  { day: 1, offset: 0, label: 'CIP · FV-02', bg: 'var(--lp-muted)', text: '#fff', border: null },
  { day: 2, offset: 1, label: 'Brew · Apostle Stout', bg: 'var(--lp-malt-deep)', text: '#fff', border: null },
  { day: 2, offset: 2.5, label: 'Ferment · FV-01 (5d)', bg: '#fff', text: 'var(--lp-hop)', border: 'var(--lp-hop)' },
  { day: 3, offset: 0, label: 'Package · Mainline Pils', bg: 'var(--lp-malt)', text: '#fff', border: null },
  { day: 4, offset: 1, label: 'Brew · West Coast IPA', bg: 'var(--lp-malt)', text: '#fff', border: null },
  { day: 5, offset: 0, label: 'Hop delivery · Yakima', bg: 'var(--lp-malt-deep)', text: '#fff', border: null },
  { day: 6, offset: 1, label: 'Tasting panel · 11am', bg: 'var(--lp-muted)', text: '#fff', border: null },
]

function CalendarSection() {
  return (
    <section className="px-16 pt-10 pb-10 max-w-[1280px] mx-auto">
      {/* Section header */}
      <div className="grid grid-cols-1 lg:grid-cols-[auto_1fr] gap-6 items-end mb-8">
        <div>
          <div className="text-[13px] font-bold text-(--lp-malt) tracking-[1.5px] font-dm-mono mb-2.5">BREW CALENDAR</div>
          <h2 className="text-[44px] font-bold tracking-[-1.4px] leading-[1.05] text-(--lp-ink) m-0 font-dm-sans">
            Plan the month{' '}
            <span className="text-(--lp-malt)">in minutes.</span>
          </h2>
        </div>
        <p className="text-[16px] text-(--lp-muted) max-w-[380px] leading-[1.5] m-0 lg:justify-self-end">
          Drag a recipe onto a date. BatchWise checks tanks, stock, and brewer hours — then orders what you're missing.
        </p>
      </div>

      {/* Calendar card */}
      <div className="bg-(--lp-card) border border-(--lp-rule) rounded-[24px] overflow-hidden">
        {/* Card header */}
        <div className="flex items-center justify-between px-6 py-[18px] border-b border-(--lp-rule) bg-(--lp-card-soft) flex-wrap gap-3">
          <div className="flex items-center gap-3.5">
            <span className="text-[18px] font-bold tracking-[-0.4px] text-(--lp-ink)">March 16 – 22</span>
            <div className="flex gap-1">
              {['←', '→'].map((a) => (
                <button key={a} className="w-7 h-7 rounded-lg border border-(--lp-rule) bg-(--lp-card) text-[14px] text-(--lp-muted) flex items-center justify-center hover:bg-(--lp-bg-alt) transition-colors cursor-pointer">{a}</button>
              ))}
            </div>
          </div>
          <div className="flex gap-1">
            {['Week', 'Month', 'Tanks'].map((v, i) => (
              <button
                key={v}
                className={`px-3.5 py-[7px] rounded-full text-[13px] font-semibold font-dm-sans border-none cursor-pointer transition-colors duration-150 ${
                  i === 0 ? 'bg-(--lp-malt-deep) text-white' : 'bg-transparent text-(--lp-muted) hover:text-(--lp-ink)'
                }`}
              >
                {v}
              </button>
            ))}
          </div>
          <div className="flex gap-3 text-[12px] text-(--lp-muted) flex-wrap">
            {[['Brew', 'var(--lp-malt)'], ['Ferment', 'var(--lp-hop)'], ['Package', 'var(--lp-malt-deep)'], ['Task', 'var(--lp-muted)']].map(([l, c]) => (
              <span key={l} className="flex items-center gap-1.5">
                <span className="w-2 h-2 rounded-[2px] inline-block" style={{ background: c }} />{l}
              </span>
            ))}
          </div>
        </div>

        {/* Day headers */}
        <div className="grid grid-cols-7 border-b border-(--lp-rule)">
          {DAYS.map((d, i) => (
            <div
              key={d}
              className="px-3.5 py-3 text-[11px] font-bold text-(--lp-faint) tracking-[1.2px] font-dm-mono"
              style={{ borderRight: i < 6 ? '1px solid var(--lp-rule)' : 'none' }}
            >
              {d.toUpperCase()}{' '}
              <span className="text-(--lp-ink) font-bold text-[18px] ml-1.5 tracking-[-0.5px]">{16 + i}</span>
            </div>
          ))}
        </div>

        {/* Event grid */}
        <div className="grid grid-cols-7 min-h-[260px]">
          {DAYS.map((d, i) => (
            <div
              key={d}
              className="p-1.5"
              style={{ borderRight: i < 6 ? '1px solid var(--lp-rule)' : 'none' }}
            >
              {CAL_EVENTS.filter((e) => e.day === i).map((e, j) => (
                <div
                  key={j}
                  className="px-2.5 py-1.5 rounded-lg text-[11px] font-semibold mb-1 leading-[1.3]"
                  style={{
                    background: e.bg,
                    color: e.text,
                    border: e.border ? `1.5px solid ${e.border}` : 'none',
                    marginTop: (e.offset || 0) * 28,
                  }}
                >
                  {e.label}
                </div>
              ))}
            </div>
          ))}
        </div>

        {/* Footer */}
        <div className="border-t border-(--lp-rule) px-6 py-3.5 flex items-center justify-between bg-(--lp-card-soft) flex-wrap gap-3">
          <div className="flex gap-6 text-[12px] text-(--lp-muted) flex-wrap">
            <span><strong className="text-(--lp-ink) text-[14px]">4 brews</strong> · 88 BBL planned</span>
            <span><strong className="text-(--lp-ink) text-[14px]">3 fermenters</strong> · in use</span>
            <span><strong className="text-(--lp-malt-deep) text-[14px]">2 reorders</strong> · before Thursday</span>
          </div>
          <button className="bg-(--lp-malt-deep) text-white border-none px-4 py-2 rounded-full font-dm-sans text-[13px] font-semibold hover:brightness-110 transition-[background,transform] duration-150 hover:translate-y-[-1px] cursor-pointer">
            + Schedule a brew
          </button>
        </div>
      </div>
    </section>
  )
}

// ─── Three pillars ────────────────────────────────────────────────────────────

const PILLARS = [
  {
    tag: 'RECIPES',
    title: 'Build recipes that scale.',
    body: 'Branch, version, and fork. Scale a kitchen batch to 30 BBL — water, IBU, and pitch rebalance automatically. BeerXML/JSON import and export.',
    headerBg: 'var(--lp-malt-soft)',
    accent: 'var(--lp-malt)',
  },
  {
    tag: 'BREWHOUSE',
    title: 'Log brewdays from your phone.',
    body: 'Mash, boil, chill — tap through it. Tilt and Plaato push live gravity and temp. Offline-first; works wherever the kettle is.',
    headerBg: 'var(--lp-hop-soft)',
    accent: 'var(--lp-hop)',
  },
  {
    tag: 'INVENTORY',
    title: 'A pantry that pays attention.',
    body: 'Lot-tracked grain, hops, yeast, and packaging. Auto-deducted on every brew. TTB BROP and excise reports ready in two clicks.',
    headerBg: 'var(--lp-malt-soft)',
    accent: 'var(--lp-malt-deep)',
  },
]

function ThreePillars() {
  return (
    <section className="px-16 py-14 max-w-[1280px] mx-auto">
      <div className="mb-10">
        <div className="text-[13px] font-bold text-(--lp-malt) tracking-[1.5px] font-dm-mono mb-2.5">THE STACK</div>
        <h2 className="text-[44px] font-bold tracking-[-1.4px] leading-[1.05] text-(--lp-ink) m-0 max-w-[720px] font-dm-sans">
          Recipe to glass, in one tool.
        </h2>
      </div>
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
        {PILLARS.map((p) => (
          <div key={p.tag} className="bg-(--lp-card) border border-(--lp-rule) rounded-[24px] overflow-hidden">
            {/* Colored header strip */}
            <div
              className="h-20 relative border-b border-(--lp-rule)"
              style={{ background: p.headerBg }}
            >
              <svg
                className="absolute right-4 top-2 opacity-85"
                width="80" height="64" viewBox="0 0 80 64"
              >
                <path d="M20 8 L60 8 L56 56 L24 56 Z" fill="none" stroke={p.accent} strokeWidth="1.5" />
                <path d="M22 14 L58 14 L55 50 L25 50 Z" fill={p.accent} fillOpacity="0.5" />
                <ellipse cx="40" cy="10" rx="18" ry="3" fill={p.accent} fillOpacity="0.3" />
              </svg>
            </div>
            <div className="p-7">
              <div className="text-[11px] font-bold tracking-[1.5px] font-dm-mono mb-2.5" style={{ color: p.accent }}>{p.tag}</div>
              <h3 className="text-[21px] font-bold tracking-[-0.4px] leading-[1.2] text-(--lp-ink) m-0 mb-2.5 font-dm-sans">{p.title}</h3>
              <p className="text-[14px] leading-[1.55] text-(--lp-muted) m-0">{p.body}</p>
            </div>
          </div>
        ))}
      </div>
    </section>
  )
}

// ─── CTA section ─────────────────────────────────────────────────────────────

const CTA_STATS = [
  ['4,212', 'breweries'],
  ['189k', 'batches brewed'],
  ['11 hr', 'saved per week'],
  ['4.9★', 'App Store'],
]

function CTASection() {
  return (
    <section className="px-16 pb-16 max-w-[1280px] mx-auto">
      <div
        className="bg-(--lp-ink) rounded-[32px] px-16 py-14 grid grid-cols-1 lg:grid-cols-[1.4fr_1fr] gap-12 items-center"
      >
        {/* Left */}
        <div>
          <h2 className="text-[48px] font-bold tracking-[-1.4px] leading-[1.05] m-0 font-dm-sans" style={{ color: 'var(--lp-bg)' }}>
            Pour your first<br />
            <span className="text-(--lp-malt)">perfect</span> pint.
          </h2>
          <p className="text-[16px] mt-4 mb-7 max-w-[440px] leading-[1.5]" style={{ color: '#bcae9a' }}>
            Free forever for homebrewers. 60-day trial and white-glove migration for commercial.
          </p>
          <div className="flex gap-2.5">
            <Link
              to="/register"
              className="bg-(--lp-malt) text-(--lp-ink) text-[15px] font-bold px-7 py-[14px] rounded-full font-dm-sans hover:brightness-110 transition-[background,transform] duration-150 hover:translate-y-[-1px] no-underline"
            >
              Start free
            </Link>
            <a
              href="#"
              className="text-[15px] font-medium px-6 py-[13px] rounded-full font-dm-sans transition-colors duration-150 no-underline hover:bg-white/10"
              style={{ color: 'var(--lp-bg)', border: '1.5px solid rgba(255,255,255,0.2)' }}
            >
              Talk to a brewer
            </a>
          </div>
        </div>

        {/* Stats grid */}
        <div className="grid grid-cols-2 gap-3">
          {CTA_STATS.map(([n, l]) => (
            <div key={l} className="rounded-[14px] p-[18px]" style={{ background: 'rgba(255,255,255,0.06)' }}>
              <div className="text-[26px] font-bold tracking-[-0.8px] text-(--lp-malt) font-dm-mono">{n}</div>
              <div className="text-[13px] mt-0.5" style={{ color: '#bcae9a' }}>{l}</div>
            </div>
          ))}
        </div>
      </div>
    </section>
  )
}

// ─── Footer ──────────────────────────────────────────────────────────────────

function Footer() {
  return (
    <footer className="px-16 pb-10 max-w-[1280px] mx-auto">
      <div className="border-t border-(--lp-rule) pt-8 flex items-center justify-between flex-wrap gap-4">
        <Logo />
        <div className="flex gap-6 text-[13px] text-(--lp-muted)">
          {['Privacy', 'Terms', 'Contact'].map((l) => (
            <a key={l} href="#" className="hover:text-(--lp-ink) transition-colors duration-150">{l}</a>
          ))}
        </div>
        <div className="text-[12px] text-(--lp-faint)">© 2026 BatchWise</div>
      </div>
    </footer>
  )
}

// ─── Page ─────────────────────────────────────────────────────────────────────

export function LandingPage() {
  const [aud, setAud] = useState<Audience>('home')

  return (
    <div className="min-h-screen bg-(--lp-bg) text-(--lp-ink) font-dm-sans">
      <Nav />
      <Hero aud={aud} setAud={setAud} />
      <IntegrationStrip />
      <Traceability />
      <CalendarSection />
      <ThreePillars />
      <CTASection />
      <Footer />
    </div>
  )
}

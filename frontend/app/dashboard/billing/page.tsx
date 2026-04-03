'use client';
import { useState, useEffect } from 'react';
import { createClient } from '@/lib/supabase/client';

const PLAN_LIMITS: Record<string, number> = { Free: 5, General: 30, Pro: 100, Enterprise: -1 };
const plans = [
  { name: 'Free', price: '0', priceLabel: 'Free', features: ['5 requests / day', '100 API calls / hr', 'Community support'], priceId: '', highlight: false },
  { name: 'General', price: '1,500', priceLabel: '1,500 / mo', features: ['30 requests / day', '1,000 API calls / hr', 'Public sharing', 'API access'], priceId: 'price_general', highlight: false },
  { name: 'Pro', price: '5,000', priceLabel: '5,000 / mo', features: ['100 requests / day', '10,000 API calls / hr', 'Private projects', 'Priority support'], priceId: 'price_pro', highlight: true },
  { name: 'Enterprise', price: '', priceLabel: 'Contact us', features: ['Unlimited requests', 'Unlimited API calls', 'SLA guarantee', 'Custom deployment'], priceId: '', highlight: false },
];

export default function BillingPage() {
  const [plan, setPlan] = useState('Free');
  const [todayCount, setTodayCount] = useState(0);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    (async () => {
      try {
        const supabase = createClient();
        const { data: { user } } = await supabase.auth.getUser();
        if (!user) return;
        const { data: profile } = await supabase.from('profiles').select('plan').eq('id', user.id).single();
        if (profile) setPlan(profile.plan || 'Free');
        const startOfDay = new Date(); startOfDay.setHours(0, 0, 0, 0);
        const { count } = await supabase.from('generations').select('id', { count: 'exact', head: true }).eq('user_id', user.id).gte('created_at', startOfDay.toISOString());
        setTodayCount(count || 0);
      } catch {} finally { setLoading(false); }
    })();
  }, []);

  const limit = PLAN_LIMITS[plan] ?? 5;
  const limitLabel = limit === -1 ? 'Unlimited' : String(limit);
  const usagePercent = limit === -1 ? 0 : Math.min(100, (todayCount / limit) * 100);

  const handleUpgrade = async (p: typeof plans[0]) => {
    if (p.name === 'Enterprise') { window.open('mailto:sakamoro@alicelaw.net?subject=ALICE LB%20Enterprise', '_blank'); return; }
    if (!p.priceId) return;
    try { const r = await fetch('/api/stripe/checkout', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ priceId: p.priceId, plan: p.name }) }); const { url } = await r.json(); if (url) window.location.href = url; } catch {}
  };

  return (
    <div className="p-6 space-y-6">
      <h1 className="text-2xl font-bold">Billing</h1>
      <div className="border rounded-lg p-4 max-w-md">
        <p className="text-sm text-muted-foreground">Today&apos;s usage</p>
        {loading ? <p className="text-sm text-muted-foreground mt-1">Loading...</p> : (
          <>
            <p className="text-2xl font-bold">{todayCount} / {limitLabel} <span className="text-sm font-normal text-muted-foreground">requests</span></p>
            <div className="mt-2 h-2 bg-muted rounded-full overflow-hidden"><div className="h-full bg-primary rounded-full transition-all" style={{ width: `${usagePercent}%` }} /></div>
            <p className="text-xs text-muted-foreground mt-2">Plan: {plan}</p>
          </>
        )}
      </div>
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 max-w-5xl">
        {plans.map((p) => (
          <div key={p.name} className={`border rounded-lg p-6 space-y-4 ${plan === p.name ? 'border-primary ring-2 ring-primary/20' : p.highlight ? 'border-primary/50' : 'border-border'}`}>
            {p.highlight && <span className="text-xs font-medium bg-primary text-primary-foreground px-2 py-0.5 rounded-full">Popular</span>}
            <h3 className="text-lg font-semibold">{p.name}</h3>
            <p className="text-2xl font-bold">{p.price ? (<><span className="text-base font-normal">&#165;</span>{p.price}<span className="text-sm font-normal text-muted-foreground"> / mo</span></>) : <span className="text-lg">{p.priceLabel}</span>}</p>
            <ul className="space-y-2">{p.features.map((f) => (<li key={f} className="text-sm text-muted-foreground flex items-center gap-2"><span className="text-primary">&#10003;</span>{f}</li>))}</ul>
            {plan === p.name ? <p className="text-sm text-primary font-medium text-center">Current plan</p> : <button onClick={() => handleUpgrade(p)} className="w-full px-4 py-2 bg-primary text-primary-foreground rounded-md text-sm font-medium hover:opacity-90">{p.name === 'Enterprise' ? 'Contact Sales' : 'Upgrade'}</button>}
          </div>
        ))}
      </div>
    </div>
  );
}

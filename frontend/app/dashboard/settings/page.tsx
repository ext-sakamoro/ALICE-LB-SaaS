'use client';
import { useState, useEffect } from 'react';
import { createClient } from '@/lib/supabase/client';
export default function SettingsPage() {
  const [email, setEmail] = useState('');
  const [apiKey, setApiKey] = useState('');
  const [plan, setPlan] = useState('Free');
  useEffect(() => { (async () => { try { const supabase = createClient(); const { data: { user } } = await supabase.auth.getUser(); if (user) { setEmail(user.email || ''); const { data } = await supabase.from('profiles').select('plan, api_key').eq('id', user.id).single(); if (data) { setPlan(data.plan || 'Free'); setApiKey(data.api_key || ''); } } } catch {} })(); }, []);
  const generateApiKey = async () => { const key = `ak_${crypto.randomUUID().replace(/-/g, '')}`; try { const supabase = createClient(); const { data: { user } } = await supabase.auth.getUser(); if (!user) return; await supabase.from('profiles').update({ api_key: key }).eq('id', user.id); setApiKey(key); } catch {} };
  return (
    <div className="p-6 space-y-6">
      <h1 className="text-2xl font-bold">Settings</h1>
      <div className="space-y-4 max-w-lg">
        <div><label className="text-sm font-medium">Email</label><p className="text-sm text-muted-foreground mt-1">{email || '\u2014'}</p></div>
        <div><label className="text-sm font-medium">Plan</label><p className="text-sm text-muted-foreground mt-1">{plan}</p></div>
        <div><label className="text-sm font-medium">API Key</label><div className="flex gap-2 mt-1"><code className="flex-1 px-3 py-2 bg-muted rounded-md text-xs font-mono break-all">{apiKey || 'No API key'}</code><button onClick={generateApiKey} className="px-3 py-2 bg-primary text-primary-foreground rounded-md text-xs font-medium hover:opacity-90">Generate</button></div></div>
      </div>
    </div>
  );
}

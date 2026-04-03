'use client';
import { useState, useEffect, useCallback } from 'react';
import { createClient } from '@/lib/supabase/client';

export interface UsageInfo { today: number; limit: number; plan: string }
const PLAN_LIMITS: Record<string, number> = { Free: 5, General: 30, Pro: 100, Enterprise: -1 };

export function useUsage() {
  const [usage, setUsage] = useState<UsageInfo>({ today: 0, limit: 5, plan: 'Free' });
  const [loading, setLoading] = useState(true);
  const fetch_ = useCallback(async () => {
    try {
      const supabase = createClient();
      const { data: { user } } = await supabase.auth.getUser();
      if (!user) return;
      const { data: profile } = await supabase.from('profiles').select('plan').eq('id', user.id).single();
      const plan = profile?.plan || 'Free';
      const limit = PLAN_LIMITS[plan] ?? 5;
      const startOfDay = new Date(); startOfDay.setHours(0, 0, 0, 0);
      const { count } = await supabase.from('generations').select('id', { count: 'exact', head: true }).eq('user_id', user.id).gte('created_at', startOfDay.toISOString());
      setUsage({ today: count || 0, limit, plan });
    } catch {} finally { setLoading(false); }
  }, []);
  useEffect(() => { fetch_(); }, [fetch_]);
  return { usage, loading, refresh: fetch_ };
}

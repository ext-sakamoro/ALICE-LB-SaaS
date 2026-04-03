'use client';
import { useState, useEffect, useCallback } from 'react';
import { createClient } from '@/lib/supabase/client';

const CAN_DOWNLOAD: Record<string, boolean> = { Free: false, General: true, Pro: true, Enterprise: true };
const DEFAULT_PUBLIC: Record<string, boolean> = { Free: false, General: true, Pro: false, Enterprise: false };

export interface PlanInfo { plan: string; canDownload: boolean; defaultPublic: boolean }

export function usePlan() {
  const [info, setInfo] = useState<PlanInfo>({ plan: 'Free', canDownload: false, defaultPublic: false });
  const [loading, setLoading] = useState(true);
  const fetch_ = useCallback(async () => {
    try {
      const supabase = createClient();
      const { data: { user } } = await supabase.auth.getUser();
      if (!user) return;
      const { data: profile } = await supabase.from('profiles').select('plan').eq('id', user.id).single();
      const plan = profile?.plan || 'Free';
      setInfo({ plan, canDownload: CAN_DOWNLOAD[plan] ?? false, defaultPublic: DEFAULT_PUBLIC[plan] ?? false });
    } catch {} finally { setLoading(false); }
  }, []);
  useEffect(() => { fetch_(); }, [fetch_]);
  return { ...info, loading, refresh: fetch_ };
}

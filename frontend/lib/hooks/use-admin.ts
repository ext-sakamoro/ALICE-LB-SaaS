'use client';
import { useCallback } from 'react';
import { createClient } from '@/lib/supabase/client';

async function getToken(): Promise<string> {
  const supabase = createClient();
  const { data } = await supabase.auth.getSession();
  return data.session?.access_token || '';
}

async function adminFetch(path: string, opts?: RequestInit) {
  const token = await getToken();
  const workerUrl = process.env.NEXT_PUBLIC_WORKER_URL || '';
  return fetch(`${workerUrl}${path}`, {
    ...opts,
    headers: { 'Authorization': `Bearer ${token}`, 'Content-Type': 'application/json', ...opts?.headers },
  });
}

export function useAdmin() {
  const getStats = useCallback(async () => (await adminFetch('/api/v1/admin/stats')).json(), []);
  const getUsers = useCallback(async () => (await adminFetch('/api/v1/admin/users')).json(), []);
  const updateUser = useCallback(async (id: string, data: Record<string, unknown>) =>
    (await adminFetch(`/api/v1/admin/users/${id}`, { method: 'PATCH', body: JSON.stringify(data) })).json(), []);
  const getProjects = useCallback(async () => (await adminFetch('/api/v1/admin/projects')).json(), []);
  const updateProject = useCallback(async (id: string, data: Record<string, unknown>) =>
    (await adminFetch(`/api/v1/admin/projects/${id}`, { method: 'PATCH', body: JSON.stringify(data) })).json(), []);
  const getRevenue = useCallback(async () => (await adminFetch('/api/v1/admin/revenue')).json(), []);
  return { getStats, getUsers, updateUser, getProjects, updateProject, getRevenue };
}

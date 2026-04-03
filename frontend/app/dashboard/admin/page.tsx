'use client';
import { useState, useEffect } from 'react';

interface Stats { uptime_secs: number; total_users: number; total_projects: number; active_rate_limiters: number }
interface User { id: string; email: string; full_name: string; plan: string; role: string; banned: boolean; created_at: string }

async function adminFetch(path: string, opts?: RequestInit) {
  const workerUrl = process.env.NEXT_PUBLIC_WORKER_URL || '';
  return fetch(`${workerUrl}${path}`, { ...opts, headers: { 'Content-Type': 'application/json', ...opts?.headers } });
}

export default function AdminPage() {
  const [stats, setStats] = useState<Stats | null>(null);
  const [users, setUsers] = useState<User[]>([]);
  const [revenue, setRevenue] = useState<{ mrr_jpy: number; subscribers: Record<string, number> } | null>(null);
  const [error, setError] = useState('');

  useEffect(() => {
    (async () => {
      try {
        const [sResp, uResp, rResp] = await Promise.all([
          adminFetch('/api/v1/admin/stats'),
          adminFetch('/api/v1/admin/users'),
          adminFetch('/api/v1/admin/revenue'),
        ]);
        if (sResp.ok) setStats(await sResp.json());
        if (uResp.ok) setUsers(await uResp.json());
        if (rResp.ok) setRevenue(await rResp.json());
        if (!sResp.ok && sResp.status === 403) setError('Admin access required');
      } catch { setError('Failed to load admin data'); }
    })();
  }, []);

  const updateUser = async (id: string, data: Record<string, unknown>) => {
    const resp = await adminFetch(`/api/v1/admin/users/${id}`, { method: 'PATCH', body: JSON.stringify(data) });
    if (resp.ok) setUsers(prev => prev.map(u => u.id === id ? { ...u, ...data } as User : u));
  };

  if (error) return <div className="p-6"><p className="text-red-500">{error}</p></div>;

  return (
    <div className="p-6 space-y-6">
      <h1 className="text-2xl font-bold">Admin Dashboard</h1>

      {stats && (
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="border rounded-lg p-4"><p className="text-sm text-muted-foreground">Uptime</p><p className="text-2xl font-bold">{Math.floor(stats.uptime_secs / 3600)}h</p></div>
          <div className="border rounded-lg p-4"><p className="text-sm text-muted-foreground">Users</p><p className="text-2xl font-bold">{stats.total_users}</p></div>
          <div className="border rounded-lg p-4"><p className="text-sm text-muted-foreground">Projects</p><p className="text-2xl font-bold">{stats.total_projects}</p></div>
          <div className="border rounded-lg p-4"><p className="text-sm text-muted-foreground">Active Sessions</p><p className="text-2xl font-bold">{stats.active_rate_limiters}</p></div>
        </div>
      )}

      {revenue && (
        <div className="border rounded-lg p-4 max-w-md">
          <p className="text-sm text-muted-foreground">Monthly Recurring Revenue</p>
          <p className="text-2xl font-bold">&yen;{revenue.mrr_jpy.toLocaleString()}</p>
          <div className="text-xs text-muted-foreground mt-1">
            General: {revenue.subscribers.general} / Pro: {revenue.subscribers.pro} / Enterprise: {revenue.subscribers.enterprise}
          </div>
        </div>
      )}

      <div>
        <h2 className="text-lg font-semibold mb-3">Users</h2>
        <div className="border rounded-lg overflow-hidden">
          <table className="w-full text-sm">
            <thead><tr className="border-b bg-muted/50"><th className="p-3 text-left">Email</th><th className="p-3 text-left">Plan</th><th className="p-3 text-left">Role</th><th className="p-3 text-left">Banned</th><th className="p-3 text-left">Actions</th></tr></thead>
            <tbody>
              {users.map(u => (
                <tr key={u.id} className="border-b">
                  <td className="p-3">{u.email}</td>
                  <td className="p-3">{u.plan}</td>
                  <td className="p-3">{u.role}</td>
                  <td className="p-3">{u.banned ? 'Yes' : 'No'}</td>
                  <td className="p-3 space-x-2">
                    <button onClick={() => updateUser(u.id, { banned: !u.banned })} className="text-xs text-red-500 hover:underline">{u.banned ? 'Unban' : 'Ban'}</button>
                    <select value={u.plan} onChange={e => updateUser(u.id, { plan: e.target.value })} className="text-xs border rounded px-1 py-0.5">
                      <option>Free</option><option>General</option><option>Pro</option><option>Enterprise</option>
                    </select>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}

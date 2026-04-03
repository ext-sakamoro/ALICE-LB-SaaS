const workerUrl = () => process.env.NEXT_PUBLIC_WORKER_URL || 'http://localhost:8080';

export async function apiGet<T>(path: string): Promise<T> {
  const resp = await fetch(`${workerUrl()}${path}`);
  if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
  return resp.json();
}

export async function apiPost<T>(path: string, body: unknown): Promise<T> {
  const resp = await fetch(`${workerUrl()}${path}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
  return resp.json();
}

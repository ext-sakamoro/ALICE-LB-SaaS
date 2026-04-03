'use client';
import { useState, useEffect, Suspense } from 'react';
import { useSearchParams, useRouter } from 'next/navigation';
import { createClient } from '@/lib/supabase/client';

export default function ConsolePage() {
  return (
    <Suspense fallback={<div className="p-6 text-sm text-muted-foreground">Loading...</div>}>
      <ConsoleInner />
    </Suspense>
  );
}

function ConsoleInner() {
  const searchParams = useSearchParams();
  const router = useRouter();
  const projectId = searchParams.get('project');
  const [projectName, setProjectName] = useState('');
  const [input, setInput] = useState('');

  useEffect(() => {
    if (!projectId) { router.push('/dashboard/projects'); return; }
    (async () => {
      try {
        const supabase = createClient();
        const { data: project } = await supabase.from('projects').select('name').eq('id', projectId).single();
        if (project) setProjectName(project.name);
      } catch {}
    })();
  }, [projectId, router]);

  return (
    <div className="p-6 space-y-4">
      <div className="flex items-center gap-3">
        <button onClick={() => router.push('/dashboard/projects')} className="text-muted-foreground hover:text-foreground text-sm">&larr; Projects</button>
        <h1 className="text-lg font-semibold">{projectName || 'Project'}</h1>
      </div>
      <textarea
        value={input}
        onChange={(e) => setInput(e.target.value)}
        placeholder="Enter your request here..."
        rows={6}
        className="w-full px-3 py-2 border border-input rounded-md bg-background text-sm"
      />
      <button
        disabled={!input.trim()}
        className="px-4 py-3 bg-primary text-primary-foreground rounded-md text-sm font-medium hover:opacity-90 disabled:opacity-50"
      >
        Execute
      </button>
      <p className="text-xs text-muted-foreground">Connect your core engine endpoints to enable domain-specific features.</p>
    </div>
  );
}

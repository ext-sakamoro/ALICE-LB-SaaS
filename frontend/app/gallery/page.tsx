'use client';
import { useState, useEffect } from 'react';
import { createClient } from '@/lib/supabase/client';

interface PublicProject { id: string; name: string; updated_at: string; owner: { full_name: string | null }[] | null }

export default function GalleryPage() {
  const [projects, setProjects] = useState<PublicProject[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    (async () => {
      try {
        const supabase = createClient();
        const { data } = await supabase
          .from('projects')
          .select('id, name, updated_at, owner:profiles!owner_id(full_name)')
          .eq('is_public', true)
          .order('updated_at', { ascending: false })
          .limit(50);
        if (data) setProjects(data as PublicProject[]);
      } catch {} finally { setLoading(false); }
    })();
  }, []);

  return (
    <div className="p-6 max-w-5xl mx-auto space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Gallery</h1>
          <p className="text-sm text-muted-foreground mt-1">Public projects shared by the community</p>
        </div>
        <a href="/auth/login" className="px-4 py-2 bg-primary text-primary-foreground rounded-md text-sm font-medium hover:opacity-90">Sign in</a>
      </div>
      {loading ? (
        <p className="text-sm text-muted-foreground">Loading...</p>
      ) : projects.length === 0 ? (
        <p className="text-sm text-muted-foreground">No public projects yet.</p>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {projects.map((p) => (
            <div key={p.id} className="border border-border rounded-lg p-4">
              <h3 className="font-semibold">{p.name}</h3>
              <p className="text-xs text-muted-foreground mt-1">{p.owner?.[0]?.full_name || 'Anonymous'} - {new Date(p.updated_at).toLocaleDateString()}</p>
            </div>
          ))}
        </div>
      )}
      <div className="text-xs text-muted-foreground border-t pt-4">Powered by ALICE LB</div>
    </div>
  );
}

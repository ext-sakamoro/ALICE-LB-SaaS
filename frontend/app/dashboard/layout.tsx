'use client';
import Link from 'next/link';
import { usePathname, useRouter } from 'next/navigation';
import { createClient } from '@/lib/supabase/client';
const nav = [
  { href: '/dashboard', label: 'Dashboard', exact: true },
  { href: '/dashboard/projects', label: 'Projects' },
  { href: '/dashboard/console', label: 'Console' },
  { href: '/dashboard/history', label: 'History' },
  { href: '/dashboard/billing', label: 'Billing' },
  { href: '/dashboard/settings', label: 'Settings' },
  { href: '/dashboard/admin', label: 'Admin' },
];
export default function DashboardLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const router = useRouter();
  const handleLogout = async () => { const supabase = createClient(); await supabase.auth.signOut(); router.push('/auth/login'); };
  const isActive = (n: typeof nav[0]) => 'exact' in n && n.exact ? pathname === n.href : pathname.startsWith(n.href);
  return (
    <div className="flex h-screen">
      <aside className="w-56 border-r border-border bg-background flex flex-col">
        <div className="p-4 font-bold text-lg border-b border-border">ALICE LB</div>
        <nav className="flex-1 p-2 space-y-1">
          {nav.map((n) => (
            <Link key={n.href} href={n.href} className={`block px-3 py-2 rounded-md text-sm ${isActive(n) ? 'bg-accent text-accent-foreground font-medium' : 'text-muted-foreground hover:bg-accent/50'}`}>{n.label}</Link>
          ))}
        </nav>
        <div className="p-2 border-t border-border">
          <button onClick={handleLogout} className="w-full px-3 py-2 text-sm text-muted-foreground hover:bg-accent/50 rounded-md text-left">Sign out</button>
        </div>
      </aside>
      <main className="flex-1 overflow-auto">{children}</main>
    </div>
  );
}

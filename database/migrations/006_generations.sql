create table if not exists public.generations (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references auth.users(id) on delete cascade,
  prompt text not null,
  result_data jsonb default '{}',
  quality text default 'standard',
  status text not null default 'pending',
  error text,
  created_at timestamptz default now()
);
alter table public.generations enable row level security;
create policy "Users can view own generations" on public.generations for select using (auth.uid() = user_id);
create policy "Users can insert own generations" on public.generations for insert with check (auth.uid() = user_id);
create index idx_generations_user_created on public.generations (user_id, created_at desc);

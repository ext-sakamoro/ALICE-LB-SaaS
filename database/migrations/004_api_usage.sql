create table if not exists public.api_usage (
  id uuid primary key default uuid_generate_v4(),
  user_id uuid references public.profiles(id) on delete cascade not null,
  endpoint text not null, method text not null, status_code int, response_time_ms float,
  created_at timestamptz default now()
);
create index if not exists idx_api_usage_user_time on public.api_usage(user_id, created_at desc);

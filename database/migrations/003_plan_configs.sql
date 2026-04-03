create table if not exists public.plan_configs (
  id uuid primary key default uuid_generate_v4(),
  plan_name text unique not null,
  max_projects int default 5,
  max_api_calls_per_hour int default 100,
  can_download boolean default false,
  default_public boolean default false,
  created_at timestamptz default now(), updated_at timestamptz default now()
);
insert into public.plan_configs (plan_name, max_projects, max_api_calls_per_hour, can_download, default_public)
values
  ('Free', 5, 100, false, false),
  ('General', 20, 1000, true, true),
  ('Pro', 100, 10000, true, false),
  ('Enterprise', -1, -1, true, false)
on conflict (plan_name) do nothing;

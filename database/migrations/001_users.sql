create extension if not exists "uuid-ossp";
create table if not exists public.profiles (
  id uuid primary key references auth.users(id) on delete cascade,
  email text, full_name text,
  plan text default 'Free' check (plan in ('Free','General','Pro','Enterprise')),
  api_key text unique, stripe_customer_id text, stripe_subscription_id text,
  role text default 'user' check (role in ('user', 'admin')),
  banned boolean default false,
  created_at timestamptz default now(), updated_at timestamptz default now()
);
alter table public.profiles enable row level security;
create policy "Users can view own profile" on public.profiles for select using (auth.uid() = id);
create policy "Users can update own profile" on public.profiles for update using (auth.uid() = id);
create policy "Admin can read all profiles" on public.profiles for select using (
  exists (select 1 from public.profiles p where p.id = auth.uid() and p.role = 'admin')
);
create policy "Admin can update all profiles" on public.profiles for update using (
  exists (select 1 from public.profiles p where p.id = auth.uid() and p.role = 'admin')
);
create or replace function public.handle_new_user() returns trigger language plpgsql security definer set search_path = '' as $$
begin insert into public.profiles (id, email) values (new.id, new.email); return new; end; $$;
drop trigger if exists on_auth_user_created on auth.users;
create trigger on_auth_user_created after insert on auth.users for each row execute procedure public.handle_new_user();

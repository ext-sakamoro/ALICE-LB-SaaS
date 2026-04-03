create or replace function public.update_updated_at() returns trigger language plpgsql as $$ begin new.updated_at = now(); return new; end; $$;
create trigger trg_profiles_updated before update on public.profiles for each row execute function public.update_updated_at();
create trigger trg_projects_updated before update on public.projects for each row execute function public.update_updated_at();
create trigger trg_plan_configs_updated before update on public.plan_configs for each row execute function public.update_updated_at();

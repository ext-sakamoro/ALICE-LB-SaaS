create or replace function public.enforce_general_public()
  returns trigger language plpgsql security definer set search_path = '' as $$
begin
  if exists (
    select 1 from public.profiles
    where id = new.owner_id and plan = 'General'
  ) then
    new.is_public := true;
  end if;
  return new;
end;
$$;
drop trigger if exists enforce_general_public_trigger on public.projects;
create trigger enforce_general_public_trigger
  before insert or update on public.projects
  for each row execute function public.enforce_general_public();

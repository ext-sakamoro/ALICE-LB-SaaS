import { createClient } from '@/lib/supabase/server';
import { createCheckoutSession } from '@/lib/stripe/server';
import { NextResponse } from 'next/server';

export async function POST(req: Request) {
  try {
    const supabase = await createClient();
    const { data: { user }, error: authError } = await supabase.auth.getUser();
    if (authError || !user) {
      return NextResponse.json({ error: 'Unauthorized' }, { status: 401 });
    }

    const { priceId } = await req.json();
    if (!priceId) {
      return NextResponse.json({ error: 'Price ID required' }, { status: 400 });
    }

    const origin = req.headers.get('origin') || 'http://localhost:3000';
    const session = await createCheckoutSession({
      userId: user.id,
      userEmail: user.email || '',
      priceId,
      successUrl: `${origin}/dashboard/billing?success=true`,
      cancelUrl: `${origin}/dashboard/billing?cancelled=true`,
    });

    return NextResponse.json({ url: session.url });
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : 'Checkout failed';
    return NextResponse.json({ error: message }, { status: 500 });
  }
}

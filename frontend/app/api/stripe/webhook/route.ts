import { stripe } from '@/lib/stripe/server';
import { createClient } from '@supabase/supabase-js';
import { NextResponse } from 'next/server';

function getSupabaseAdmin() {
  const url = process.env.NEXT_PUBLIC_SUPABASE_URL;
  const serviceKey = process.env.SUPABASE_SERVICE_ROLE_KEY;
  if (!url || !serviceKey) return null;
  return createClient(url, serviceKey);
}

export async function POST(req: Request) {
  if (!stripe) {
    return NextResponse.json({ error: 'Stripe not configured' }, { status: 503 });
  }

  const body = await req.text();
  const sig = req.headers.get('stripe-signature');
  const webhookSecret = process.env.STRIPE_WEBHOOK_SECRET;

  if (!sig || !webhookSecret) {
    return NextResponse.json({ error: 'Missing signature' }, { status: 400 });
  }

  let event;
  try {
    event = stripe.webhooks.constructEvent(body, sig, webhookSecret);
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : 'Invalid signature';
    return NextResponse.json({ error: message }, { status: 400 });
  }

  const supabase = getSupabaseAdmin();

  switch (event.type) {
    case 'checkout.session.completed': {
      const session = event.data.object;
      const userId = session.subscription
        ? (typeof session.subscription === 'string' ? session.metadata?.userId : undefined)
        : session.metadata?.userId;
      const subUserId = session.metadata?.userId || userId;

      if (supabase && subUserId) {
        await supabase
          .from('profiles')
          .update({
            plan: 'Pro',
            stripe_customer_id: session.customer as string,
            stripe_subscription_id: session.subscription as string,
          })
          .eq('id', subUserId);
      }
      break;
    }
    case 'customer.subscription.updated': {
      const subscription = event.data.object;
      const userId = subscription.metadata?.userId;

      if (supabase && userId) {
        const status = subscription.status;
        if (status === 'active') {
          await supabase
            .from('profiles')
            .update({ plan: 'Pro' })
            .eq('id', userId);
        }
      }
      break;
    }
    case 'customer.subscription.deleted': {
      const subscription = event.data.object;
      const userId = subscription.metadata?.userId;

      if (supabase && userId) {
        await supabase
          .from('profiles')
          .update({
            plan: 'Free',
            stripe_subscription_id: null,
          })
          .eq('id', userId);
      }
      break;
    }
    default:
      break;
  }

  return NextResponse.json({ received: true });
}

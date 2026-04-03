import Stripe from 'stripe';

const stripeSecretKey = process.env.STRIPE_SECRET_KEY;

export const stripe = stripeSecretKey
  ? new Stripe(stripeSecretKey, { apiVersion: '2025-08-27.basil', typescript: true })
  : (null as unknown as Stripe);

export async function createCheckoutSession({
  userId, userEmail, priceId, successUrl, cancelUrl,
}: {
  userId: string; userEmail: string; priceId: string;
  successUrl: string; cancelUrl: string;
}) {
  if (!stripe) throw new Error('Stripe is not configured');
  if (!priceId) throw new Error('Price ID is required');

  const customers = await stripe.customers.list({ email: userEmail, limit: 1 });
  const customer = customers.data[0] ?? await stripe.customers.create({
    email: userEmail, metadata: { userId },
  });

  return stripe.checkout.sessions.create({
    customer: customer.id,
    payment_method_types: ['card'],
    line_items: [{ price: priceId, quantity: 1 }],
    mode: 'subscription',
    success_url: successUrl,
    cancel_url: cancelUrl,
    subscription_data: { metadata: { userId } },
  });
}

export async function createCustomerPortalSession({
  customerId, returnUrl,
}: { customerId: string; returnUrl: string }) {
  return stripe.billingPortal.sessions.create({
    customer: customerId, return_url: returnUrl,
  });
}

import { loadStripe } from '@stripe/stripe-js';
import type { Stripe } from '@stripe/stripe-js';

let stripePromise: Promise<Stripe | null>;

export const getStripe = () => {
  if (!stripePromise) {
    const key = process.env.NEXT_PUBLIC_STRIPE_PUBLISHABLE_KEY;
    if (!key) return Promise.resolve(null);
    stripePromise = loadStripe(key);
  }
  return stripePromise;
};

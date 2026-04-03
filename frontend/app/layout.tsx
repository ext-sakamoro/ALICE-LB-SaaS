import type { Metadata } from 'next';
import { Inter } from 'next/font/google';
import '@/styles/globals.css';
const inter = Inter({ subsets: ['latin'] });
export const metadata: Metadata = { title: 'ALICE LB', description: 'ALICE LB - Cloud platform powered by ALICE' };
export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (<html lang="en" suppressHydrationWarning><body className={inter.className}>{children}</body></html>);
}
